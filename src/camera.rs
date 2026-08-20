//! Third-person orbit camera with follow lag and directional lead.
//!
//! The camera deliberately does not sit rigidly on the player. It chases a
//! pivot with exponential smoothing, and aims slightly ahead of where the
//! player is travelling. Both are tunable, including down to zero, which
//! turns them off.
//!
//! # SPEC
//!
//! - K1: zero lag snaps exactly to the target in one step.
//! - K2: positive lag moves toward the target without reaching or passing it.
//! - K3: smoothing is framerate-independent - the same elapsed time produces
//!   the same result whether taken in one step or many.
//! - K4: pitch is clamped to the configured range.
//! - K5: lead points along travel and never exceeds `cam_lead_amount`.
//! - K6: a stationary player produces no lead.

use crate::tuning::Tuning;
use bevy::prelude::*;

/// Orbit state, owned by the camera rig.
#[derive(Component, Debug, Default)]
pub struct CameraRig {
    pub yaw: f32,
    pub pitch: f32,
    /// Smoothed pivot position, chasing the player.
    pub focus: Vec3,
    /// Smoothed lead offset, so lead eases rather than snapping when the
    /// player changes direction.
    pub lead: Vec3,
}

/// Fraction to move toward a target this step.
///
/// `1 - exp(-dt/lag)` rather than a raw lerp constant, so the result depends
/// on elapsed time and not on frame count.
pub fn smooth_factor(lag: f32, dt: f32) -> f32 {
    if lag <= 0.0 {
        return 1.0;
    }
    1.0 - (-dt / lag).exp()
}

/// Clamp pitch into the configured range.
pub fn clamp_pitch(pitch: f32, t: &Tuning) -> f32 {
    pitch.clamp(t.cam_pitch_min, t.cam_pitch_max)
}

/// How far ahead of the player the camera should aim.
///
/// Scales with how fast the player is actually going, relative to run speed,
/// so walking leads less than sprinting.
pub fn lead_offset(velocity_xz: Vec3, t: &Tuning) -> Vec3 {
    let flat = Vec3::new(velocity_xz.x, 0.0, velocity_xz.z);
    let speed = flat.length();
    if speed < 1e-6 || t.cam_lead_amount <= 0.0 {
        return Vec3::ZERO;
    }
    // Lead in proportion to how fast you are actually going, capped at 1 so
    // the offset never runs away above run speed.
    let fraction = (speed / t.run_speed).min(1.0);
    flat / speed * t.cam_lead_amount * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    /// K1
    #[test]
    fn zero_lag_snaps() {
        assert!(
            (smooth_factor(0.0, 1.0 / 60.0) - 1.0).abs() < 1e-6,
            "zero lag must snap fully"
        );
    }

    /// K2
    #[test]
    fn positive_lag_moves_partway() {
        let f = smooth_factor(0.2, 1.0 / 60.0);
        assert!(f > 0.0 && f < 1.0, "factor should be a partial step, got {f}");
    }

    /// K3 - the property that separates real smoothing from a naive lerp.
    #[test]
    fn smoothing_is_framerate_independent() {
        let lag = 0.25;
        // One big step of 0.1s...
        let one = smooth_factor(lag, 0.1);
        // ...versus ten small steps totalling 0.1s, compounded.
        let mut remaining = 1.0f32;
        for _ in 0..10 {
            remaining *= 1.0 - smooth_factor(lag, 0.01);
        }
        let many = 1.0 - remaining;
        assert!(
            (one - many).abs() < 1e-4,
            "one step {one} should match ten steps {many}"
        );
    }

    /// K4
    #[test]
    fn pitch_is_clamped() {
        let t = Tuning::default();
        assert!((clamp_pitch(99.0, &t) - t.cam_pitch_max).abs() < 1e-6, "upper");
        assert!((clamp_pitch(-99.0, &t) - t.cam_pitch_min).abs() < 1e-6, "lower");
        let mid = (t.cam_pitch_min + t.cam_pitch_max) * 0.5;
        assert!((clamp_pitch(mid, &t) - mid).abs() < 1e-6, "middle untouched");
    }

    /// K5
    #[test]
    fn lead_follows_travel_and_is_capped() {
        let t = Tuning::default();
        let fast = lead_offset(Vec3::new(t.run_speed * 5.0, 0.0, 0.0), &t);
        assert!(
            fast.length() <= t.cam_lead_amount + 1e-4,
            "lead {} exceeded cap {}",
            fast.length(),
            t.cam_lead_amount
        );
        assert!(fast.x > 0.0, "lead should point along travel, got {fast:?}");
        assert!(fast.y.abs() < 1e-6, "lead should stay horizontal");
    }

    /// K6
    #[test]
    fn standing_still_produces_no_lead() {
        let t = Tuning::default();
        assert!(
            lead_offset(Vec3::ZERO, &t).length() < 1e-6,
            "stationary player should not lead the camera"
        );
    }
}

// ---------------------------------------------------------------------------
// Bevy systems
// ---------------------------------------------------------------------------

use crate::player::PlayerBody;
use crate::scene::Player;
use bevy::input::mouse::AccumulatedMouseMotion;

/// Spawn the orbit camera.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), CameraRig::default(), Transform::default()));
}

/// Mouse orbits the camera, but only while the cursor is captured.
pub fn mouse_look(
    motion: Res<AccumulatedMouseMotion>,
    tuning: Res<Tuning>,
    cursors: Query<&bevy::window::CursorOptions>,
    mut rigs: Query<&mut CameraRig>,
) {
    let captured = cursors
        .iter()
        .any(|c| c.grab_mode != bevy::window::CursorGrabMode::None);
    if !captured {
        return;
    }
    let Ok(mut rig) = rigs.single_mut() else {
        return;
    };
    let delta = motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    rig.yaw -= delta.x * tuning.cam_sensitivity;
    let sign = if tuning.cam_invert_y { -1.0 } else { 1.0 };
    rig.pitch = clamp_pitch(rig.pitch - delta.y * tuning.cam_sensitivity * sign, &tuning);
}

/// Chase the player with lag, aiming slightly ahead of their travel.
pub fn follow_player(
    time: Res<Time>,
    tuning: Res<Tuning>,
    players: Query<(&Transform, &PlayerBody), (With<Player>, Without<CameraRig>)>,
    mut cameras: Query<(&mut Transform, &mut CameraRig)>,
) {
    let dt = time.delta_secs();
    let Ok((player_tf, body)) = players.single() else {
        return;
    };
    let Ok((mut cam_tf, mut rig)) = cameras.single_mut() else {
        return;
    };

    let desired_focus = player_tf.translation + Vec3::Y * tuning.cam_height;
    // First frame: sit exactly on target rather than easing in from origin.
    if rig.focus == Vec3::ZERO {
        rig.focus = desired_focus;
    }
    let follow = smooth_factor(tuning.cam_follow_lag, dt);
    rig.focus = rig.focus.lerp(desired_focus, follow);

    let desired_lead = lead_offset(body.velocity, &tuning);
    let lead = smooth_factor(tuning.cam_lead_lag, dt);
    rig.lead = rig.lead.lerp(desired_lead, lead);

    let aim = rig.focus + rig.lead;
    let rotation = Quat::from_euler(EulerRot::YXZ, rig.yaw, rig.pitch, 0.0);
    cam_tf.translation = aim + rotation * Vec3::new(0.0, 0.0, tuning.cam_distance);
    cam_tf.look_at(aim, Vec3::Y);
}
