//! Live tuning panel.
//!
//! This is the reason the build exists. Every field of [`Tuning`] gets a
//! control here; if a number is added there and not surfaced here, the panel
//! has failed at its job.
//!
//! Opening the panel releases the mouse cursor, because sliders are useless
//! with the cursor locked to camera look.

use crate::player::PlayerBody;
use crate::tuning::Tuning;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Whether the tuning panel is showing.
#[derive(Resource, Debug)]
pub struct DebugPanel {
    pub open: bool,
}

impl Default for DebugPanel {
    fn default() -> Self {
        // Open on launch: the panel is the point of this build, so hiding it
        // by default would bury the feature.
        Self { open: true }
    }
}

/// F1 toggles the panel.
pub fn toggle_panel(keys: Res<ButtonInput<KeyCode>>, mut panel: ResMut<DebugPanel>) {
    if keys.just_pressed(KeyCode::F1) {
        panel.open = !panel.open;
    }
}

/// Draw the panel and let every tunable number be edited live.
pub fn tuning_panel(
    mut contexts: EguiContexts,
    mut tuning: ResMut<Tuning>,
    panel: Res<DebugPanel>,
    player: Query<&PlayerBody>,
) -> Result {
    if !panel.open {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let t = &mut *tuning;

    egui::Window::new("Tuning (F1)")
        .default_width(320.0)
        .show(ctx, |ui| {
            // ---- live readouts ----
            if let Ok(body) = player.single() {
                let speed = Vec2::new(body.velocity.x, body.velocity.z).length();
                ui.label(
                    egui::RichText::new(format!("speed          {speed:>7.2} m/s")).monospace(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "grounded       {:>7}",
                        if body.grounded { "yes" } else { "no" }
                    ))
                    .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "since grounded {:>7.3} s",
                        body.time_since_grounded
                    ))
                    .monospace(),
                );
                ui.label(
                    egui::RichText::new(format!("vertical vel   {:>7.2} m/s", body.velocity.y))
                        .monospace(),
                );
            } else {
                ui.label("no player");
            }

            ui.separator();
            egui::CollapsingHeader::new("ground")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut t.walk_speed, 0.0..=20.0).text("walk speed"));
                    ui.add(egui::Slider::new(&mut t.run_speed, 0.0..=30.0).text("run speed"));
                    ui.add(egui::Slider::new(&mut t.ground_accel, 1.0..=200.0).text("accel"));
                    ui.add(egui::Slider::new(&mut t.ground_friction, 0.0..=200.0).text("friction"));
                });

            egui::CollapsingHeader::new("air")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut t.air_accel, 0.0..=100.0).text("air accel"));
                    ui.add(egui::Slider::new(&mut t.air_control, 0.0..=1.0).text("air control"));
                });

            egui::CollapsingHeader::new("jump & gravity")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut t.jump_height, 0.1..=6.0).text("jump height m"));
                    ui.add(egui::Slider::new(&mut t.rise_gravity, 1.0..=100.0).text("rise gravity"));
                    ui.add(egui::Slider::new(&mut t.fall_gravity, 1.0..=150.0).text("fall gravity"));
                    ui.add(
                        egui::Slider::new(&mut t.max_fall_speed, 1.0..=120.0).text("terminal vel"),
                    );
                    ui.add(egui::Slider::new(&mut t.coyote_time, 0.0..=0.5).text("coyote time s"));
                    ui.add(
                        egui::Slider::new(&mut t.jump_buffer_time, 0.0..=0.5).text("jump buffer s"),
                    );
                    ui.label(
                        egui::RichText::new(format!("derived jump vel {:.2} m/s", t.jump_velocity()))
                            .monospace()
                            .weak(),
                    );
                });

            egui::CollapsingHeader::new("camera")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut t.cam_distance, 1.0..=25.0).text("distance"));
                    ui.add(egui::Slider::new(&mut t.cam_height, -2.0..=8.0).text("pivot height"));
                    ui.add(egui::Slider::new(&mut t.cam_follow_lag, 0.0..=1.0).text("follow lag"));
                    ui.add(egui::Slider::new(&mut t.cam_lead_amount, 0.0..=8.0).text("lead amount"));
                    ui.add(egui::Slider::new(&mut t.cam_lead_lag, 0.0..=1.5).text("lead lag"));
                    ui.add(
                        egui::Slider::new(&mut t.cam_sensitivity, 0.0002..=0.02)
                            .logarithmic(true)
                            .text("sensitivity"),
                    );
                    ui.add(egui::Slider::new(&mut t.cam_pitch_min, -1.5..=0.0).text("pitch min"));
                    ui.add(egui::Slider::new(&mut t.cam_pitch_max, 0.0..=1.5).text("pitch max"));
                    ui.checkbox(&mut t.cam_invert_y, "invert Y");
                });

            egui::CollapsingHeader::new("body (needs respawn to see visually)")
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut t.player_radius, 0.1..=1.5).text("radius"));
                    ui.add(
                        egui::Slider::new(&mut t.player_half_length, 0.1..=2.0).text("half length"),
                    );
                });

            ui.separator();
            if ui.button("reset all to defaults").clicked() {
                *t = Tuning::default();
            }
        });

    Ok(())
}
