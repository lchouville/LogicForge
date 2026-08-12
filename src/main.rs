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
        .add_plugins(DefaultPlugins)
        .add_plugins((SimulationPlugin, EditorPlugin, RenderingPlugin))
        .run();
}
