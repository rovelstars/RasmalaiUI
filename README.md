# 🥣 RasmalaiUI

**A sweet, scriptable, and blazingly fast GPU-accelerated UI framework for the Rust ecosystem.**

**RasmalaiUI** is a modern, retained-mode UI toolkit designed for **RunixOS**, but fully compatible with Linux, macOS, Windows, and Android. It focuses on high-fidelity vector graphics, flexible layout, and dynamic scripting to provide a "Flutter-like" developer experience with "Rust-like" performance.

---

## ✨ Key Features

* **⚡ Vello Rendering:** 100% GPU-accelerated vector graphics using Wgpu and Vello. No more jagged edges or CPU-heavy painting.
* **📐 Flexbox & Grid:** Powered by **Taffy**, providing industry-standard layout capabilities that feel familiar to web and mobile developers.
* **📜 Scriptable with Rune:** Define your UI logic and state in **Rune**. Update your app's behavior without recompiling the entire OS kernel or binary.
* **✍️ Advanced Typography:** Seamless text shaping and font fallback using **Parley**.
* **📱 Mobile First:** Built specifically to shine on high-density displays like the **OnePlus 13s** (Project Silicium), but scales beautifully to desktop.
* **📦 No Legacy Baggage:** No dependencies on GTK, Qt, or heavy C libraries. Pure Rust from top to bottom.

---

## 🛠 The "Sweet" Stack

RasmalaiUI stands on the shoulders of giants:

* **Renderer:** [Vello](https://github.com/linebender/vello) (Compute-centric vector graphics)
* **Layout:** [Taffy](https://github.com/DioxusLabs/taffy) (High-performance UI layout)
* **Scripting:** [Rune](https://rune-rs.github.io/) (Embeddable dynamic language for Rust)
* **Text:** [Parley](https://github.com/linebender/parley) (Rich text layout)
* **Windowing:** [Winit](https://github.com/rust-windowing/winit) + [Wgpu](https://wgpu.rs/)

---

## 🚀 Quick Start (Coming Soon)

```rust
// A taste of what's cooking
use rasmalai::prelude::*;

fn main() {
    // This creates a normal app with server side decorations. Should work properly as a normal app OOTB on any supported platform.
    let app = App::new()
        .with_script("scripts/main.rn")
    app.run();
}

```

And your UI logic in `main.rn`:

```rust
//use rsx macro to get jsx like syntax
use rasmalai::prelude::*;
use rasmalai::components::{Title, Button};
fn main() {
    const state = State::new(0);
    const button = Button::new("Click Me!");
    button.on_click(|state| {
        state.count += 1;
        print("Rasmalai is served!");
    });

    rsx! {
        <div>
            <Title>Rasmalai</Title> // This special component directly overrides the default title bar of the app
            <h1>Rasmalais served: {state.count}</h1>
            {button}
        </div>
    }
}



```

---

## 🏗 Architecture & RunixOS

While RasmalaiUI is cross-platform, it is the primary UI interface for **RunixOS**. This doesn't stop it from being cross-compatible to other platforms.

---

## 🤝 Contributing

We welcome contributions to the "kitchen"! Whether it's optimizing the Vello shaders, adding new widgets, or improving the Rune bindings, feel free to open a PR.

1. Fork the repo.
2. Create your feature branch (`git checkout -b feature/sweet-new-widget`).
3. Commit your changes.
4. Push to the branch.
5. Open a Pull Request.

---

## 📜 License

Distributed under the MIT License. See `LICENSE` for more information.