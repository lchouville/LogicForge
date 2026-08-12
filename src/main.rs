mod constants;
mod editor;
mod grid;
mod rendering;
mod simulation;

use bevy::prelude::*;

use editor::EditorPlugin;
use rendering::RenderingPlugin;
use simulation::SimulationPlugin;

fn main() {
    App::new()
        .add_plugins(default_plugins())
        .add_plugins((SimulationPlugin, EditorPlugin, RenderingPlugin))
        .run();
}

// wgpu's automatic backend choice picks Vulkan on Windows, which crashes on
// this machine's Intel driver when the window moves across monitors
// (swapchain recreation). DX12 doesn't have that issue and is the more
// stable native backend on Windows in general. This customization must stay
// native-only: on wasm32 it breaks the render app's async setup entirely
// (screen stays black, "Render app did not exist" in the console) even
// though the resulting settings look equivalent to the defaults there.
#[cfg(not(target_arch = "wasm32"))]
fn default_plugins() -> impl PluginGroup {
    use bevy::render::RenderPlugin;
    use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};

    let backends = if cfg!(target_os = "windows") {
        Some(Backends::DX12)
    } else {
        None
    };

    DefaultPlugins.set(RenderPlugin {
        render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
            backends,
            ..default()
        })),
        ..default()
    })
}

#[cfg(target_arch = "wasm32")]
fn default_plugins() -> impl PluginGroup {
    DefaultPlugins
}
