//! Code-driven character controller.
//!
//! Nothing here applies a force to anything. Velocity is computed outright
//! each frame and handed to [`crate::collision::move_and_slide`], which only
//! ever subtracts motion that geometry blocks. That is what keeps "physics
//! never pushes the player" true by construction.
//!
//! The feel logic lives in free functions so it can be tested without a
//! window, a GPU, or a running app.
//!
//! # SPEC
//!
//! - P1: rising uses `rise_gravity`; falling uses `fall_gravity`.
//! - P2: fall speed is clamped to `max_fall_speed`.
//! - P3: a jump is allowed while within `coyote_time` of leaving the ground.
//! - P4: a jump is refused once past `coyote_time`.
//! - P5: a buffered jump press fires on landing; an expired one does not.
//! - P6: ground acceleration approaches the target speed without overshoot.
//! - P7: with no input on the ground, friction decays speed to exactly zero
//!   and never reverses it.
//! - P8: airborne acceleration is scaled by `air_control`.

use crate::tuning::Tuning;
use bevy::prelude::*;

/// Per-frame mutable state of the character.
#[derive(Component, Debug, Clone)]
pub struct PlayerBody {
    pub velocity: Vec3,
    pub grounded: bool,
    /// Seconds since the character was last on the ground. Zero while
    /// grounded. Drives coyote time, and shown in the debug panel.
    pub time_since_grounded: f32,
    /// Seconds remaining on a buffered jump press. Zero means none pending.
    pub jump_buffer: f32,
    /// Yaw the body is facing, radians. Follows the movement direction.
    pub facing_yaw: f32,
}

impl Default for PlayerBody {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            grounded: true,
            time_since_grounded: 0.0,
            jump_buffer: 0.0,
            facing_yaw: 0.0,
        }
    }
}

/// Gravity to apply given current vertical velocity.
///
/// Split so a jump can float on the way up and drop fast on the way down.
pub fn gravity_for(vertical_velocity: f32, t: &Tuning) -> f32 {
    if vertical_velocity > 0.0 {
        t.rise_gravity
    } else {
        // The apex counts as falling: the snappier gravity should take over
        // the instant upward motion stops, not a frame later.
        t.fall_gravity
    }
}

/// Advance vertical velocity by one step, clamped to terminal velocity.
pub fn step_vertical(vertical_velocity: f32, t: &Tuning, dt: f32) -> f32 {
    let v = vertical_velocity - gravity_for(vertical_velocity, t) * dt;
    v.max(-t.max_fall_speed)
}

/// May the character jump right now?
///
/// True when a jump was pressed recently enough to still be buffered, and
/// the character is either grounded or inside the coyote window.
pub fn should_jump(time_since_grounded: f32, jump_buffer: f32, t: &Tuning) -> bool {
    jump_buffer > 0.0 && time_since_grounded <= t.coyote_time
}

/// Advance horizontal velocity toward `wish_dir * target_speed`.
///
/// `wish_dir` is expected normalised, or zero for no input. When it is zero
/// and the character is grounded, friction applies instead of acceleration.
pub fn step_horizontal(
    velocity_xz: Vec2,
    wish_dir: Vec2,
    target_speed: f32,
    grounded: bool,
    t: &Tuning,
    dt: f32,
) -> Vec2 {
    const EPS: f32 = 1e-6;

    // No input: on the ground friction brings you to a stop; in the air you
    // keep your momentum, which is what makes air control feel like steering
    // rather than braking.
    if wish_dir.length_squared() <= EPS {
        if !grounded {
            return velocity_xz;
        }
        let speed = velocity_xz.length();
        if speed <= EPS {
            return Vec2::ZERO;
        }
        // Subtract a fixed amount of speed, floored at zero, so friction
        // lands exactly on rest instead of oscillating around it.
        let new_speed = (speed - t.ground_friction * dt).max(0.0);
        return velocity_xz * (new_speed / speed);
    }

    let accel = if grounded {
        t.ground_accel
    } else {
        t.air_accel * t.air_control
    };

    let target = wish_dir * target_speed;
    let delta = target - velocity_xz;
    let max_step = accel * dt;
    if delta.length() <= max_step {
        // Close enough to land on the target exactly - no overshoot.
        target
    } else {
        velocity_xz + delta.normalize() * max_step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> Tuning {
        Tuning::default()
    }

    /// P1
    #[test]
    fn rising_and_falling_use_different_gravity() {
        let t = tuning();
        assert_eq!(gravity_for(5.0, &t), t.rise_gravity, "rising");
        assert_eq!(gravity_for(-5.0, &t), t.fall_gravity, "falling");
        assert_eq!(gravity_for(0.0, &t), t.fall_gravity, "apex counts as falling");
    }

    /// P1 again, through the integrator.
    #[test]
    fn step_vertical_applies_the_right_gravity() {
        let t = tuning();
        let dt = 1.0 / 60.0;
        let rising = step_vertical(5.0, &t, dt);
        assert!(
            (rising - (5.0 - t.rise_gravity * dt)).abs() < 1e-5,
            "rising: got {rising}"
        );
        let falling = step_vertical(-1.0, &t, dt);
        assert!(
            (falling - (-1.0 - t.fall_gravity * dt)).abs() < 1e-5,
            "falling: got {falling}"
        );
    }

    /// P2
    #[test]
    fn fall_speed_is_clamped_to_terminal_velocity() {
        let t = tuning();
        let mut v = 0.0;
        for _ in 0..600 {
            v = step_vertical(v, &t, 1.0 / 60.0);
        }
        assert!(
            v >= -t.max_fall_speed - 1e-4,
            "fell to {v}, past terminal {}",
            -t.max_fall_speed
        );
    }

    /// P3
    #[test]
    fn coyote_window_allows_a_late_jump() {
        let t = tuning();
        let just_left = t.coyote_time * 0.5;
        assert!(
            should_jump(just_left, t.jump_buffer_time, &t),
            "jump should register {just_left}s after leaving the ledge"
        );
    }

    /// P4
    #[test]
    fn jump_is_refused_after_the_coyote_window() {
        let t = tuning();
        let too_late = t.coyote_time * 2.0;
        assert!(
            !should_jump(too_late, t.jump_buffer_time, &t),
            "jump should not register {too_late}s after leaving the ledge"
        );
    }

    /// P5
    #[test]
    fn buffered_press_fires_but_expired_one_does_not() {
        let t = tuning();
        assert!(
            should_jump(0.0, 0.01, &t),
            "a press still in the buffer should fire on landing"
        );
        assert!(
            !should_jump(0.0, 0.0, &t),
            "an expired buffer should not fire"
        );
    }

    /// P6
    #[test]
    fn ground_acceleration_approaches_target_without_overshoot() {
        let t = tuning();
        let dt = 1.0 / 60.0;
        let mut v = Vec2::ZERO;
        for _ in 0..600 {
            v = step_horizontal(v, Vec2::X, t.walk_speed, true, &t, dt);
        }
        assert!(
            (v.x - t.walk_speed).abs() < 1e-3,
            "should settle at walk speed {}, got {}",
            t.walk_speed,
            v.x
        );
        assert!(
            v.x <= t.walk_speed + 1e-3,
            "must not overshoot target, got {}",
            v.x
        );
    }

    /// P7 - friction must land on zero, not oscillate around it.
    #[test]
    fn friction_decays_to_exactly_zero() {
        let t = tuning();
        let dt = 1.0 / 60.0;
        let mut v = Vec2::new(t.run_speed, 0.0);
        for _ in 0..600 {
            v = step_horizontal(v, Vec2::ZERO, 0.0, true, &t, dt);
        }
        assert!(v.length() < 1e-4, "should stop dead, got {v:?}");
        assert!(v.x >= 0.0, "friction must never reverse direction, got {}", v.x);
    }

    /// P8
    #[test]
    fn air_control_scales_acceleration() {
        let t = tuning();
        let dt = 1.0 / 60.0;
        let on_ground = step_horizontal(Vec2::ZERO, Vec2::X, t.walk_speed, true, &t, dt);
        let in_air = step_horizontal(Vec2::ZERO, Vec2::X, t.walk_speed, false, &t, dt);
        assert!(
            in_air.x < on_ground.x,
            "air acceleration ({}) should be weaker than ground ({})",
            in_air.x,
            on_ground.x
        );
        assert!(in_air.x > 0.0, "air control should still allow steering");
    }
}

// ---------------------------------------------------------------------------
// Bevy system
// ---------------------------------------------------------------------------

use crate::camera::CameraRig;
use crate::collision::{Aabb, move_and_slide};
use crate::scene::{Level, Player};

/// Read input, integrate velocity, and move the character.
///
/// Velocity is computed outright and handed to `move_and_slide`. Nothing
/// applies a force; collision only ever removes motion.
pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    tuning: Res<Tuning>,
    level: Res<Level>,
    rigs: Query<&CameraRig>,
    mut players: Query<(&mut Transform, &mut PlayerBody), With<Player>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok((mut transform, mut body)) = players.single_mut() else {
        return;
    };
    // Movement is relative to where the camera is looking.
    let yaw = rigs.single().map(|r| r.yaw).unwrap_or(0.0);

    let mut raw = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        raw.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        raw.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        raw.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        raw.x -= 1.0;
    }

    // Camera-space basis on the XZ plane. Bevy's forward is -Z.
    let (sin, cos) = yaw.sin_cos();
    let forward = Vec2::new(-sin, -cos);
    let right = Vec2::new(cos, -sin);
    let mut wish = forward * raw.y + right * raw.x;
    if wish.length_squared() > 1e-6 {
        wish = wish.normalize();
    } else {
        wish = Vec2::ZERO;
    }

    let target_speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        tuning.run_speed
    } else {
        tuning.walk_speed
    };

    // Jump buffering: remember a press made slightly too early.
    if keys.just_pressed(KeyCode::Space) {
        body.jump_buffer = tuning.jump_buffer_time;
    }
    body.jump_buffer = (body.jump_buffer - dt).max(0.0);

    // Horizontal.
    let planar = Vec2::new(body.velocity.x, body.velocity.z);
    let stepped = step_horizontal(planar, wish, target_speed, body.grounded, &tuning, dt);
    body.velocity.x = stepped.x;
    body.velocity.z = stepped.y;

    // Vertical: either a jump fires, or gravity applies.
    if should_jump(body.time_since_grounded, body.jump_buffer, &tuning) {
        body.velocity.y = tuning.jump_velocity();
        body.jump_buffer = 0.0;
        // Consume the coyote window so one ledge cannot yield two jumps.
        body.time_since_grounded = tuning.coyote_time + 1.0;
        body.grounded = false;
    } else {
        body.velocity.y = step_vertical(body.velocity.y, &tuning, dt);
    }

    // Move, then let geometry subtract whatever is blocked.
    let half = Vec3::new(
        tuning.player_radius,
        tuning.player_half_height(),
        tuning.player_radius,
    );
    let result = move_and_slide(
        Aabb::new(transform.translation, half),
        body.velocity * dt,
        &level.blockers,
    );
    transform.translation = result.position;

    if result.grounded || result.hit_ceiling {
        body.velocity.y = 0.0;
    }
    body.grounded = result.grounded;
    if result.grounded {
        body.time_since_grounded = 0.0;
    } else {
        body.time_since_grounded += dt;
    }

    // Face the direction of travel.
    if stepped.length_squared() > 0.01 {
        body.facing_yaw = (-stepped.x).atan2(-stepped.y);
    }
    transform.rotation = Quat::from_rotation_y(body.facing_yaw);
}

#[cfg(test)]
mod integration_tests {
    //! Drives the real `move_player` system with synthetic input, headless.
    //!
    //! These answer "does the capsule actually move?" repeatably, instead of
    //! relying on someone watching a window.
    //!
    //! - M1: holding forward accelerates to walk speed and travels forward.
    //! - M2: holding run reaches run speed, which is faster than walking.
    //! - M3: a jump leaves the ground, peaks, and lands again.
    //! - M4: standing still, the character never sinks through the floor.

    use super::*;
    use crate::camera::CameraRig;
    use crate::scene::{Level, Player, level_blockers, spawn_point};
    use std::time::Duration;

    const DT: f32 = 1.0 / 60.0;

    fn app() -> App {
        let t = Tuning::default();
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Tuning>()
            .insert_resource(Level {
                blockers: level_blockers(),
            })
            .add_systems(Update, move_player);
        app.world_mut().spawn(CameraRig::default());
        app.world_mut().spawn((
            Player,
            PlayerBody::default(),
            Transform::from_translation(spawn_point(&t)),
        ));
        app
    }

    fn step(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(DT));
        app.update();
        // The input plugin normally does this; without it, just_pressed would
        // stay true forever and a single tap would jump every frame.
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().clear();
    }

    fn player_state(app: &mut App) -> (Vec3, PlayerBody) {
        let (tf, body) = app
            .world_mut()
            .query::<(&Transform, &PlayerBody)>()
            .single(app.world())
            .expect("player exists");
        (tf.translation, body.clone())
    }

    /// M1
    #[test]
    fn holding_forward_moves_the_capsule() {
        let mut app = app();
        let (start, _) = player_state(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        for _ in 0..60 {
            step(&mut app);
        }
        let (end, body) = player_state(&mut app);
        let travelled = (end - start).length();
        assert!(
            travelled > 1.0,
            "capsule should have moved; travelled only {travelled:.3} m"
        );
        // Camera yaw 0 means forward is -Z.
        assert!(
            end.z < start.z - 1.0,
            "should travel forward (-Z): {} -> {}",
            start.z,
            end.z
        );
        let speed = Vec2::new(body.velocity.x, body.velocity.z).length();
        let t = Tuning::default();
        assert!(
            (speed - t.walk_speed).abs() < 0.1,
            "should settle at walk speed {}, got {speed:.3}",
            t.walk_speed
        );
    }

    /// M2
    #[test]
    fn running_is_faster_than_walking() {
        let t = Tuning::default();

        let mut walk = app();
        walk.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        for _ in 0..60 {
            step(&mut walk);
        }
        let (walk_end, _) = player_state(&mut walk);

        let mut run = app();
        {
            let mut input = run.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::KeyW);
            input.press(KeyCode::ShiftLeft);
        }
        for _ in 0..60 {
            // Re-press: clear() wipes held state too in this harness.
            let mut input = run.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::KeyW);
            input.press(KeyCode::ShiftLeft);
            drop(input);
            step(&mut run);
        }
        let (run_end, body) = player_state(&mut run);

        let speed = Vec2::new(body.velocity.x, body.velocity.z).length();
        assert!(
            (speed - t.run_speed).abs() < 0.2,
            "should settle at run speed {}, got {speed:.3}",
            t.run_speed
        );
        assert!(
            run_end.distance(Vec3::ZERO) > walk_end.distance(Vec3::ZERO),
            "running should cover more ground than walking"
        );
    }

    /// M3
    #[test]
    fn jump_leaves_the_ground_and_lands_again() {
        let mut app = app();
        let (start, body) = player_state(&mut app);
        assert!(body.grounded, "should start grounded");

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);

        let mut peak = start.y;
        let mut left_ground = false;
        for _ in 0..90 {
            step(&mut app);
            let (pos, b) = player_state(&mut app);
            peak = peak.max(pos.y);
            if !b.grounded {
                left_ground = true;
            }
        }
        let (end, body) = player_state(&mut app);

        assert!(left_ground, "jump should leave the ground");
        let t = Tuning::default();
        assert!(
            peak > start.y + t.jump_height * 0.7,
            "should rise near jump height {}; peaked at {:.3} above start",
            t.jump_height,
            peak - start.y
        );
        assert!(body.grounded, "should land again within 1.5 s");
        assert!(
            (end.y - start.y).abs() < 1e-3,
            "should land back at floor level, got {} vs {}",
            end.y,
            start.y
        );
    }

    /// M4
    #[test]
    fn standing_still_never_sinks() {
        let mut app = app();
        let (start, _) = player_state(&mut app);
        for _ in 0..300 {
            step(&mut app);
        }
        let (end, body) = player_state(&mut app);
        assert!(body.grounded, "should remain grounded");
        assert!(
            (end.y - start.y).abs() < 1e-4,
            "should not sink or drift: {} -> {}",
            start.y,
            end.y
        );
    }
}
