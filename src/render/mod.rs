use std::sync::Arc;
use winit::window::Window;
use vello::peniko::Color;
use vello::util::{RenderContext as VelloRenderContext, RenderSurface};
use vello::{Renderer, RendererOptions, Scene};
use vello::wgpu; // Use vello's re-exported wgpu if available, or just wgpu crate if versions match.

pub struct RenderContext {
    // Vello context
    vello_context: VelloRenderContext,
    renderers: Vec<Option<Renderer>>,
    surface: RenderSurface<'static>,
    scene: Scene,
    use_cpu: bool,
    target_texture: Option<wgpu::Texture>,
    
    // Cached Blit resources
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    blit_bind_group: Option<wgpu::BindGroup>,
    
    start_time: std::time::Instant,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>, use_cpu: bool) -> Self {
        let mut vello_context = VelloRenderContext::new(); 
        
        // Create surface
        let surface = vello_context.create_surface(
            window.clone(), 
            window.inner_size().width, 
            window.inner_size().height,
            wgpu::PresentMode::AutoVsync,
        ).await.expect("failed to create surface");
        
        let renderer_options = RendererOptions {
            antialiasing_support: vello::AaSupport::all(),
            num_init_threads: None,
            pipeline_cache: None,
            use_cpu,
        };

        let device = &vello_context.devices[surface.dev_id].device;
        let renderer = vello::Renderer::new(
            device,
            renderer_options, 
        ).expect("failed to create renderer");

        let scene = Scene::new();
        
        // --- Initialize Blit Pipeline ---
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(r#"
                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                };

                @vertex
                fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
                    var out: VertexOutput;
                    var positions = array<vec2<f32>, 3>(
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>(3.0, -1.0),
                        vec2<f32>(-1.0, 3.0)
                    );
                    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
                    out.uv = positions[vertex_index] * 0.5 + 0.5;
                    out.uv.y = 1.0 - out.uv.y; 
                    return out;
                }

                @group(0) @binding(0) var t_diffuse: texture_2d<f32>;
                @group(0) @binding(1) var s_diffuse: sampler;

                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    return textureSample(t_diffuse, s_diffuse, in.uv);
                }
            "#)),
        });

        let blit_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blit Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface.config.format, 
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            vello_context,
            renderers: vec![Some(renderer)],
            surface,
            scene,
            use_cpu,
            target_texture: None,
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            blit_bind_group: None,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.vello_context.resize_surface(&mut self.surface, size.width, size.height);
        // Invalidate target texture so it gets recreated
        self.target_texture = None;
        self.blit_bind_group = None;
        
        // Re-enable synchronous render for smooth resizing -> Reverted due to lag
        // self.render();
    }

    pub fn render(&mut self) {
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        if width == 0 || height == 0 { return; }

        let device = &self.vello_context.devices[self.surface.dev_id].device;
        let queue = &self.vello_context.devices[self.surface.dev_id].queue;
 
        // 0. Update Scene Content (Rotating Rainbow Triangle)
        self.scene.reset(); 
        
        let time = self.start_time.elapsed().as_secs_f64();
        let center = vello::kurbo::Point::new(width as f64 / 2.0, height as f64 / 2.0);
        let radius = 200.0;
        
        // Create a triangle path
        let mut path = vello::kurbo::BezPath::new();
        for i in 0..3 {
            let angle = time + (i as f64) * (2.0 * std::f64::consts::PI / 3.0);
            let point = vello::kurbo::Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            if i == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close_path();

        // Rainbow gradient
        let stops = [
            vello::peniko::ColorStop { offset: 0.0, color: vello::peniko::Color::from_rgb8(255, 0, 0).into() },
            vello::peniko::ColorStop { offset: 0.14, color: vello::peniko::Color::from_rgb8(255, 165, 0).into() },
            vello::peniko::ColorStop { offset: 0.28, color: vello::peniko::Color::from_rgb8(255, 255, 0).into() },
            vello::peniko::ColorStop { offset: 0.42, color: vello::peniko::Color::from_rgb8(0, 128, 0).into() },
            vello::peniko::ColorStop { offset: 0.57, color: vello::peniko::Color::from_rgb8(0, 0, 255).into() },
            vello::peniko::ColorStop { offset: 0.71, color: vello::peniko::Color::from_rgb8(75, 0, 130).into() },
            vello::peniko::ColorStop { offset: 0.85, color: vello::peniko::Color::from_rgb8(238, 130, 238).into() },
            vello::peniko::ColorStop { offset: 1.0, color: vello::peniko::Color::from_rgb8(255, 0, 0).into() },
        ];
        
        let gradient = vello::peniko::Gradient::new_sweep(
            center,
            0.0,
            std::f64::consts::PI as f32 * 2.0,
        ).with_stops(stops.as_slice());

        self.scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::rotate_about(time, center),
            &gradient,
            None,
            &path
        );

        // 1. Initialize target_texture if needed
        if self.target_texture.is_none() {
             let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Vello Target Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
             });
             self.target_texture = Some(texture);
             // Invalidate bind group since texture changed
             self.blit_bind_group = None;
        }

        let target_view = self.target_texture.as_ref().unwrap().create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Initialize Blit Bind Group if needed
        if self.blit_bind_group.is_none() {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blit Bind Group"),
                layout: &self.blit_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&target_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                    },
                ],
            });
            self.blit_bind_group = Some(bind_group);
        }

        // 3. Ensure renderer exists
        if self.renderers.len() <= self.surface.dev_id {
             self.renderers.resize_with(self.surface.dev_id + 1, || None);
        }
        if self.renderers[self.surface.dev_id].is_none() {
             let renderer = vello::Renderer::new(
                device,
                RendererOptions {
                    antialiasing_support: vello::AaSupport::all(),
                    num_init_threads: None,
                    pipeline_cache: None,
                    use_cpu: self.use_cpu,
                },
            ).expect("failed to create renderer");
            self.renderers[self.surface.dev_id] = Some(renderer);
        }

        let renderer = self.renderers[self.surface.dev_id].as_mut().unwrap();

        // 4. Render to intermediate texture
        renderer
            .render_to_texture(
                device,
                queue,
                &self.scene,
                &target_view,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(20, 20, 20),
                    width,
                    height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .expect("failed to render to intermediate texture");

        // 5. Blit to surface
        let surface_texture = match self.surface.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Surface timeout");
                return;
            }
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                 // Reconfigure or ignore, usually resize handles this next frame. 
                 // We can return early.
                 log::warn!("Surface outdated/lost");
                 return;
            }
            Err(e) => panic!("failed to get surface texture: {:?}", e),
        };
        
        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Blit Encoder") });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store, // Store the result
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.blit_pipeline);
            rpass.set_bind_group(0, self.blit_bind_group.as_ref().unwrap(), &[]);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

pub trait PollsterBlockOn {
    type Output;
    fn pollster_block_on(self) -> Self::Output;
}

impl<F: std::future::Future> PollsterBlockOn for F {
    type Output = F::Output;
    fn pollster_block_on(self) -> Self::Output {
        pollster::block_on(self)
    }
}
