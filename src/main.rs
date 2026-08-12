mod constants;
mod editor;
mod grid;
mod rendering;
mod simulation;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};

use editor::EditorPlugin;
use rendering::RenderingPlugin;
use simulation::SimulationPlugin;

fn main() {
    // wgpu's automatic backend choice picks Vulkan on Windows, which crashes on
    // this Intel driver when the window moves across monitors (swapchain
    // recreation). DX12 doesn't have that issue and is the more stable native
    // backend on Windows in general; other platforms keep wgpu's own default.
    let backends = if cfg!(target_os = "windows") {
        Some(Backends::DX12)
    } else {
        None
    };

    App::new()
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                backends,
                ..default()
            })),
            ..default()
        }))
        .add_plugins((SimulationPlugin, EditorPlugin, RenderingPlugin))
        .run();
}
