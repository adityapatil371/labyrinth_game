//! Every tunable game-feel number lives here, in one struct.
//!
//! This is the point of the whole build: the code is scaffolding, these
//! numbers are the deliverable. Nothing outside this file may hard-code a
//! movement or camera value - read it from [`Tuning`] instead.
//!
//! Units are metres, seconds, and radians throughout.
//!
//! # SPEC
//!
//! - T1: `Tuning::default()` satisfies `run_speed > walk_speed` - running must
//!   be a distinct gear, not a rounding difference.
//! - T2: `fall_gravity >= rise_gravity` - falls are never floatier than rises.
//! - T3: the default jump clears the tallest platform, so every platform in
//!   the scene is reachable and vertical movement is actually testable.
//! - T4: all durations are non-negative; a negative coyote or buffer window
//!   would silently invert the comparison that uses it.

use bevy::prelude::*;

/// All live-tunable movement and camera numbers.
///
/// Edited at runtime through the debug panel; see `debug_ui`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Tuning {
    // ---- ground movement ----
    /// Target horizontal speed when walking.
    pub walk_speed: f32,
    /// Target horizontal speed while the run key is held.
    pub run_speed: f32,
    /// How fast horizontal velocity approaches the target on the ground.
    pub ground_accel: f32,
    /// How fast horizontal velocity decays to zero on the ground with no input.
    pub ground_friction: f32,

    // ---- air movement ----
    /// How fast horizontal velocity approaches the target while airborne.
    pub air_accel: f32,
    /// Fraction of `air_accel` actually applied. 0 = no steering in air,
    /// 1 = full ground authority while airborne.
    pub air_control: f32,

    // ---- jump and gravity ----
    /// Apex height of a jump from standing, in metres.
    pub jump_height: f32,
    /// Downward acceleration while moving up. Lower = floatier rise.
    pub rise_gravity: f32,
    /// Downward acceleration while moving down. Higher than `rise_gravity`
    /// gives the classic snappy platformer fall.
    pub fall_gravity: f32,
    /// Terminal velocity. Caps fall speed so long drops stay controllable.
    pub max_fall_speed: f32,
    /// How long after walking off a ledge a jump still registers.
    pub coyote_time: f32,
    /// How long before landing a jump press is remembered and fired.
    pub jump_buffer_time: f32,

    // ---- camera ----
    /// Distance from the camera pivot to the camera.
    pub cam_distance: f32,
    /// Height of the pivot above the player's origin.
    pub cam_height: f32,
    /// Smoothing time for the camera chasing the player. Larger = laggier.
    pub cam_follow_lag: f32,
    /// How far ahead of the player, in the direction of travel, the camera
    /// aims. 0 disables lead entirely.
    pub cam_lead_amount: f32,
    /// Smoothing time for the lead offset itself, so lead eases in and out
    /// instead of snapping when direction changes.
    pub cam_lead_lag: f32,
    /// Radians of rotation per pixel of mouse motion.
    pub cam_sensitivity: f32,
    /// Lowest pitch, radians. Negative looks down at the player from above.
    pub cam_pitch_min: f32,
    /// Highest pitch, radians.
    pub cam_pitch_max: f32,
    /// Invert vertical mouse look.
    pub cam_invert_y: bool,

    // ---- player body ----
    /// Capsule radius. Also the half-width of the collision AABB.
    pub player_radius: f32,
    /// Half the capsule height *excluding* the hemispheres, matching Bevy's
    /// `Capsule3d::half_length`. Total height is
    /// `2 * (player_half_length + player_radius)`.
    pub player_half_length: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            // A real jog is ~3 m/s, which reads as sluggish on screen.
            walk_speed: 6.0,
            // ~1.7x walk: a gear you can feel, and enough to cross a room.
            run_speed: 10.0,
            // Reaches walk speed in ~0.1 s - responsive without feeling icy.
            ground_accel: 60.0,
            // Slightly less than accel, so stopping reads as deliberate.
            ground_friction: 50.0,

            air_accel: 20.0,
            // Partial authority: steering in air is possible but not free.
            air_control: 0.35,

            // Clears the tallest platform (1.5 m) with margin. See T3.
            jump_height: 1.6,
            // 2h/t^2 for a 1.6 m apex in 0.38 s. Earth gravity feels floaty.
            rise_gravity: 22.0,
            // ~1.8x rise: float up, drop fast. Reads as weight.
            fall_gravity: 40.0,
            max_fall_speed: 50.0,
            // ~7 frames at 60 Hz: forgiving but not obviously wrong.
            coyote_time: 0.12,
            // Slightly longer than coyote; players press early more than late.
            jump_buffer_time: 0.15,

            cam_distance: 7.0,
            cam_height: 1.6,
            cam_follow_lag: 0.12,
            cam_lead_amount: 1.2,
            cam_lead_lag: 0.25,
            cam_sensitivity: 0.0025,
            cam_pitch_min: -1.2,
            cam_pitch_max: 1.2,
            cam_invert_y: false,

            player_radius: 0.4,
            player_half_length: 0.5,
        }
    }
}

impl Tuning {
    /// Total capsule height, hemispheres included.
    pub fn player_height(&self) -> f32 {
        2.0 * (self.player_half_length + self.player_radius)
    }

    /// Distance from the player's origin to the soles of its feet.
    pub fn player_half_height(&self) -> f32 {
        self.player_half_length + self.player_radius
    }

    /// Initial upward velocity that reaches exactly `jump_height` under
    /// `rise_gravity`. Derived so jump *height* stays the tunable, which is
    /// what a designer actually reasons about.
    pub fn jump_velocity(&self) -> f32 {
        (2.0 * self.rise_gravity * self.jump_height).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::TALLEST_PLATFORM_HEIGHT;

    /// T1
    #[test]
    fn running_is_a_distinct_gear() {
        let t = Tuning::default();
        assert!(
            t.run_speed > t.walk_speed,
            "run {} must exceed walk {}",
            t.run_speed,
            t.walk_speed
        );
    }

    /// T2
    #[test]
    fn falls_are_never_floatier_than_rises() {
        let t = Tuning::default();
        assert!(
            t.fall_gravity >= t.rise_gravity,
            "fall gravity {} must be >= rise gravity {}",
            t.fall_gravity,
            t.rise_gravity
        );
    }

    /// T3 - the invariant that keeps the scene testable.
    #[test]
    fn default_jump_clears_the_tallest_platform() {
        let t = Tuning::default();
        assert!(
            t.jump_height > TALLEST_PLATFORM_HEIGHT,
            "jump height {} must clear tallest platform {}",
            t.jump_height,
            TALLEST_PLATFORM_HEIGHT
        );
    }

    /// T4
    #[test]
    fn timing_windows_are_non_negative() {
        let t = Tuning::default();
        assert!(t.coyote_time >= 0.0, "coyote {}", t.coyote_time);
        assert!(t.jump_buffer_time >= 0.0, "buffer {}", t.jump_buffer_time);
        assert!(t.cam_follow_lag >= 0.0, "follow lag {}", t.cam_follow_lag);
        assert!(t.cam_lead_lag >= 0.0, "lead lag {}", t.cam_lead_lag);
    }

    /// Derived jump velocity must actually reach the requested height:
    /// v^2 = 2*g*h, so h = v^2 / (2g).
    #[test]
    fn jump_velocity_reaches_jump_height() {
        let t = Tuning::default();
        let v = t.jump_velocity();
        let reached = v * v / (2.0 * t.rise_gravity);
        assert!(
            (reached - t.jump_height).abs() < 1e-4,
            "derived velocity {} reaches {} but jump_height is {}",
            v,
            reached,
            t.jump_height
        );
    }

    /// The capsule must match Bevy's Capsule3d convention, where half_length
    /// excludes the hemispheres. Getting this backwards buries the feet.
    #[test]
    fn player_height_includes_hemispheres() {
        let t = Tuning::default();
        assert!((t.player_height() - 1.8).abs() < 1e-6, "{}", t.player_height());
        assert!((t.player_half_height() - 0.9).abs() < 1e-6, "{}", t.player_half_height());
    }
}
