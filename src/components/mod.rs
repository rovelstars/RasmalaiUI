// Re-export components
pub struct Title;
pub struct Button;

impl Button {
    pub fn new(_text: &str) -> Self { Self }
    pub fn on_click<F>(&self, _f: F) where F: Fn(crate::prelude::State) {}
}
