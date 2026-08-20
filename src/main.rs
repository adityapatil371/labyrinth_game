//! Third-person character controller test scene.
//!
//! This exists to tune game feel. The code is scaffolding; the numbers in
//! [`tuning::Tuning`] are the deliverable. Press F1 to open the panel and
//! edit every one of them while running.
//!
//! Controls:
//!   WASD    move, relative to camera facing
//!   Shift   run
//!   Space   jump
//!   Mouse   orbit camera
//!   F1      toggle the tuning panel (releases the cursor so you can drag)
//!   Escape  release the cursor
//!   Click   re-capture the cursor
//!
//! API verified against bevy 0.19.1 and bevy_egui 0.42.0. Bevy is pre-1.0;
//! see CLAUDE.md before trusting any recalled API.

pub mod camera;
pub mod collision;
pub mod debug_ui;
pub mod player;
pub mod scene;
pub mod tuning;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .init_resource::<tuning::Tuning>()
        .init_resource::<debug_ui::DebugPanel>()
        .insert_resource(scene::Level {
            blockers: scene::level_blockers(),
        })
        .add_systems(Startup, (scene::spawn_level, camera::spawn_camera))
        .add_systems(
            Update,
            (
                debug_ui::toggle_panel,
                scene::cursor_grab,
                camera::mouse_look,
                player::move_player,
                camera::follow_player,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, debug_ui::tuning_panel)
        .run();
}
