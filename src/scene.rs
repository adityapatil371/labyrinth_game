//! Graybox level: three boxy rooms joined by short wide passages.
//!
//! Level geometry is deliberately *not* in [`crate::tuning::Tuning`]. Tuning
//! holds numbers you change while playing to chase a feel; room dimensions
//! are fixed layout that would require respawning geometry to change. They
//! are named constants here rather than literals scattered through the
//! spawn code.
//!
//! Collision and visuals are generated from the *same* list of boxes, so
//! they cannot drift apart: what you see is exactly what you collide with.
//!
//! # SPEC
//!
//! - S1: the level has blockers, and every blocker has positive extents.
//! - S2: there are exactly three platforms, at three distinct heights.
//! - S3: the spawn point puts the player's feet exactly on the floor.
//! - S4: the three rooms are actually connected - a straight walk at player
//!   height from the first room to the last passes through no blocker.

use crate::collision::Aabb;
use crate::tuning::Tuning;
use bevy::prelude::*;

// ---- level dimensions, in metres ----

/// Half-width of a room. Rooms are 40x40 - large enough to build up run
/// speed and land a jump without immediately hitting a wall.
pub const ROOM_HALF: f32 = 20.0;
/// Interior wall height.
pub const WALL_HEIGHT: f32 = 8.0;
/// Wall and floor slab thickness.
pub const SLAB: f32 = 1.0;
/// Half-width of a connecting passage. Wide enough to run through without
/// catching on the sides.
pub const PASSAGE_HALF_WIDTH: f32 = 4.0;
/// Length of a connecting passage - short, per the brief.
pub const PASSAGE_LENGTH: f32 = 12.0;
/// Distance between adjacent room centres.
pub const ROOM_SPACING: f32 = 2.0 * ROOM_HALF + PASSAGE_LENGTH;
/// Number of rooms.
pub const ROOM_COUNT: usize = 3;

/// One raised platform per room, each a different height so vertical
/// movement is testable. All must be reachable - see tuning test T3.
pub const PLATFORM_HEIGHTS: [f32; ROOM_COUNT] = [0.5, 1.0, 1.5];
/// The tallest platform. The default jump must clear this.
pub const TALLEST_PLATFORM_HEIGHT: f32 = 1.5;
/// Half-extent of a platform in X and Z.
pub const PLATFORM_HALF_XZ: f32 = 3.0;

/// Marker for the player entity.
#[derive(Component, Debug)]
pub struct Player;

/// Marker for the small nub showing which way the player faces.
#[derive(Component, Debug)]
pub struct FacingMarker;

/// All static collision boxes in the level.
#[derive(Resource, Debug, Default)]
pub struct Level {
    pub blockers: Vec<Aabb>,
}

/// Centre of room `i` on the X axis.
pub fn room_center_x(i: usize) -> f32 {
    i as f32 * ROOM_SPACING
}

/// Every static box in the level: floors, walls and platforms.
pub fn level_blockers() -> Vec<Aabb> {
    let mut out = Vec::new();
    let half_slab = SLAB * 0.5;

    for i in 0..ROOM_COUNT {
        let cx = room_center_x(i);

        // Floor: top surface at y = 0.
        out.push(Aabb::from_base(
            Vec3::new(cx, -SLAB, 0.0),
            Vec3::new(ROOM_HALF, half_slab, ROOM_HALF),
        ));

        // North and south walls, full width, overhanging into the corners.
        for sz in [-1.0f32, 1.0] {
            out.push(Aabb::from_base(
                Vec3::new(cx, 0.0, sz * (ROOM_HALF + half_slab)),
                Vec3::new(ROOM_HALF + SLAB, WALL_HEIGHT * 0.5, half_slab),
            ));
        }

        // East and west walls. Where a passage connects, the wall is split
        // into two segments leaving a gap the width of the passage.
        for sx in [-1.0f32, 1.0] {
            let has_gap = if sx < 0.0 { i > 0 } else { i + 1 < ROOM_COUNT };
            let wall_x = cx + sx * (ROOM_HALF + half_slab);
            if has_gap {
                let seg_len = (ROOM_HALF + SLAB) - PASSAGE_HALF_WIDTH;
                let seg_half = seg_len * 0.5;
                for sz in [-1.0f32, 1.0] {
                    out.push(Aabb::from_base(
                        Vec3::new(wall_x, 0.0, sz * (PASSAGE_HALF_WIDTH + seg_half)),
                        Vec3::new(half_slab, WALL_HEIGHT * 0.5, seg_half),
                    ));
                }
            } else {
                out.push(Aabb::from_base(
                    Vec3::new(wall_x, 0.0, 0.0),
                    Vec3::new(half_slab, WALL_HEIGHT * 0.5, ROOM_HALF + SLAB),
                ));
            }
        }

        // One raised platform per room, offset off the central corridor line
        // so it never blocks the straight path between rooms (see S4).
        let h = PLATFORM_HEIGHTS[i];
        out.push(Aabb::from_base(
            Vec3::new(cx + 6.0, 0.0, 8.0),
            Vec3::new(PLATFORM_HALF_XZ, h * 0.5, PLATFORM_HALF_XZ),
        ));
    }

    // Passages between adjacent rooms: floor plus two side walls.
    for i in 0..ROOM_COUNT - 1 {
        let mid_x = room_center_x(i) + ROOM_HALF + PASSAGE_LENGTH * 0.5;
        let half_len = PASSAGE_LENGTH * 0.5;

        out.push(Aabb::from_base(
            Vec3::new(mid_x, -SLAB, 0.0),
            Vec3::new(half_len, half_slab, PASSAGE_HALF_WIDTH),
        ));

        for sz in [-1.0f32, 1.0] {
            out.push(Aabb::from_base(
                Vec3::new(mid_x, 0.0, sz * (PASSAGE_HALF_WIDTH + half_slab)),
                Vec3::new(half_len, WALL_HEIGHT * 0.5, half_slab),
            ));
        }
    }

    out
}

/// Where the player starts: centre of the first room, feet on the floor.
pub fn spawn_point(tuning: &Tuning) -> Vec3 {
    Vec3::new(room_center_x(0), tuning.player_half_height(), 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S1
    #[test]
    fn level_has_blockers_with_positive_extents() {
        let blockers = level_blockers();
        assert!(!blockers.is_empty(), "level must contain geometry");
        for (i, b) in blockers.iter().enumerate() {
            assert!(
                b.half.x > 0.0 && b.half.y > 0.0 && b.half.z > 0.0,
                "blocker {i} has non-positive half-extents: {:?}",
                b.half
            );
        }
    }

    /// S2
    #[test]
    fn three_platforms_at_three_distinct_heights() {
        let mut heights = PLATFORM_HEIGHTS.to_vec();
        heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
        heights.dedup();
        assert_eq!(
            heights.len(),
            ROOM_COUNT,
            "expected {ROOM_COUNT} distinct platform heights, got {heights:?}"
        );
        // Every platform must exist as a blocker at its stated height.
        let blockers = level_blockers();
        for h in PLATFORM_HEIGHTS {
            let found = blockers
                .iter()
                .any(|b| (b.max().y - h).abs() < 1e-4 && b.half.x <= PLATFORM_HALF_XZ + 1e-4);
            assert!(found, "no platform blocker with top surface at height {h}");
        }
    }

    /// S3
    #[test]
    fn player_spawns_with_feet_on_the_floor() {
        let t = Tuning::default();
        let p = spawn_point(&t);
        assert!(
            (p.y - t.player_half_height()).abs() < 1e-6,
            "feet should rest on y=0; centre expected at {}, got {}",
            t.player_half_height(),
            p.y
        );
        // And the spawn point must not be inside geometry.
        let body = Aabb::new(p, Vec3::new(t.player_radius, t.player_half_height(), t.player_radius));
        for b in level_blockers() {
            assert!(
                !body.intersects(&b),
                "spawn point {p:?} is inside blocker {b:?}"
            );
        }
    }

    /// Build a headless app and run the spawn systems: no window, no GPU.
    fn spawned_app() -> App {
        use bevy::asset::{AssetApp, AssetPlugin};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_resource::<Tuning>()
            .insert_resource(Level {
                blockers: level_blockers(),
            })
            .add_systems(Startup, (spawn_level, crate::camera::spawn_camera));
        app.update();
        app
    }

    /// S5
    #[test]
    fn spawns_exactly_one_player_and_one_camera() {
        let mut app = spawned_app();
        let players = app.world_mut().query::<&Player>().iter(app.world()).len();
        assert_eq!(players, 1, "expected exactly one player");
        let cams = app
            .world_mut()
            .query::<&crate::camera::CameraRig>()
            .iter(app.world())
            .len();
        assert_eq!(cams, 1, "expected exactly one camera rig");
    }

    /// S6 - the facing marker is the only cue for which way the capsule points.
    #[test]
    fn player_has_a_facing_marker() {
        let mut app = spawned_app();
        let markers = app
            .world_mut()
            .query::<&FacingMarker>()
            .iter(app.world())
            .len();
        assert_eq!(markers, 1, "expected exactly one facing marker");
    }

    /// S7
    #[test]
    fn spawns_one_directional_light() {
        let mut app = spawned_app();
        let lights = app
            .world_mut()
            .query::<&DirectionalLight>()
            .iter(app.world())
            .len();
        assert_eq!(lights, 1, "expected exactly one directional light");
    }

    /// S8 - visuals must match collision exactly, one mesh per blocker plus
    /// the player capsule and its marker.
    #[test]
    fn every_blocker_gets_a_mesh() {
        let mut app = spawned_app();
        let meshes = app.world_mut().query::<&Mesh3d>().iter(app.world()).len();
        let expected = level_blockers().len() + 2; // + player capsule + marker
        assert_eq!(
            meshes, expected,
            "expected {expected} meshes (blockers + player + marker), got {meshes}"
        );
    }

    /// S9 - the player must start standing on the floor, not inside or above it.
    #[test]
    fn player_starts_on_the_ground() {
        let mut app = spawned_app();
        let t = Tuning::default();
        let transform = *app
            .world_mut()
            .query_filtered::<&Transform, With<Player>>()
            .single(app.world())
            .expect("player should exist");
        assert!(
            (transform.translation.y - t.player_half_height()).abs() < 1e-5,
            "player should start with feet on y=0, centre at {}, got {}",
            t.player_half_height(),
            transform.translation.y
        );

        // And one simulated step of gravity must not move it: it is grounded.
        let half = Vec3::new(t.player_radius, t.player_half_height(), t.player_radius);
        let body = Aabb::new(transform.translation, half);
        let r = crate::collision::move_and_slide(
            body,
            Vec3::new(0.0, -t.fall_gravity * (1.0 / 60.0) * (1.0 / 60.0), 0.0),
            &level_blockers(),
        );
        assert!(r.grounded, "player should be grounded on the first step");
        assert!(
            (r.position.y - transform.translation.y).abs() < 1e-5,
            "grounded player should not sink"
        );
    }

    /// S4 - catches the classic bug where a wall seals a passage.
    #[test]
    fn rooms_are_connected_end_to_end() {
        let t = Tuning::default();
        let blockers = level_blockers();
        let half = Vec3::new(t.player_radius, t.player_half_height(), t.player_radius);
        let y = t.player_half_height();

        let start_x = room_center_x(0);
        let end_x = room_center_x(ROOM_COUNT - 1);
        let steps = 600;
        for s in 0..=steps {
            let x = start_x + (end_x - start_x) * (s as f32 / steps as f32);
            let body = Aabb::new(Vec3::new(x, y, 0.0), half);
            for b in &blockers {
                assert!(
                    !body.intersects(b),
                    "path blocked at x={x:.2} by {b:?} - rooms are not connected"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Bevy systems
// ---------------------------------------------------------------------------

/// Which shade of grey a blocker should render as.
///
/// Derived from geometry rather than stored, so the collision list stays the
/// single source of truth and visuals cannot drift from it.
fn blocker_shade(b: &Aabb) -> f32 {
    if b.max().y <= 0.0 + 1e-4 {
        0.26 // floor
    } else if b.half.x <= PLATFORM_HALF_XZ + 1e-4 && b.half.z <= PLATFORM_HALF_XZ + 1e-4 {
        0.55 // platform - lighter so it reads as standable
    } else {
        0.38 // wall
    }
}

/// Spawn the level, the player, and the lighting.
///
/// Every visual box comes from the same `Level::blockers` list the collision
/// code uses, so what you see is exactly what you collide with.
pub fn spawn_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tuning: Res<Tuning>,
    level: Res<Level>,
) {
    let mut grey = |v: f32| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(v, v, v),
            perceptual_roughness: 1.0,
            ..default()
        })
    };
    let floor_mat = grey(0.26);
    let wall_mat = grey(0.38);
    let platform_mat = grey(0.55);
    let player_mat = grey(0.75);
    let marker_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.35, 0.2),
        perceptual_roughness: 1.0,
        ..default()
    });

    for b in &level.blockers {
        let shade = blocker_shade(b);
        let mat = if shade < 0.3 {
            floor_mat.clone()
        } else if shade > 0.5 {
            platform_mat.clone()
        } else {
            wall_mat.clone()
        };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(
                b.half.x * 2.0,
                b.half.y * 2.0,
                b.half.z * 2.0,
            ))),
            MeshMaterial3d(mat),
            Transform::from_translation(b.center),
        ));
    }

    // Player capsule, with a small nub on its front face so facing direction
    // is readable at a glance. Bevy's forward is -Z.
    let nose_z = -(tuning.player_radius + 0.12);
    commands.spawn((
        Player,
        crate::player::PlayerBody::default(),
        Mesh3d(meshes.add(Capsule3d::new(
            tuning.player_radius,
            tuning.player_half_length,
        ))),
        MeshMaterial3d(player_mat),
        Transform::from_translation(spawn_point(&tuning)),
        children![(
            FacingMarker,
            Mesh3d(meshes.add(Cuboid::new(0.18, 0.18, 0.3))),
            MeshMaterial3d(marker_mat),
            Transform::from_xyz(0.0, 0.35, nose_z),
        )],
    ));

    // Simple lighting: one directional key light plus enough ambient to keep
    // the unlit sides of the graybox readable.
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // In bevy 0.19 the global ambient resource is GlobalAmbientLight;
    // AmbientLight is a per-camera component.
    commands.insert_resource(GlobalAmbientLight {
        brightness: 260.0,
        ..default()
    });
}

/// Cursor capture.
///
/// Locked and hidden while playing. Escape releases it, clicking re-captures
/// it, and opening the tuning panel releases it too - sliders are unusable
/// with the cursor locked to camera look.
pub fn cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    panel: Res<crate::debug_ui::DebugPanel>,
    mut released: Local<bool>,
    mut cursors: Query<&mut bevy::window::CursorOptions>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        *released = true;
    }
    if mouse.just_pressed(MouseButton::Left) && !panel.open {
        *released = false;
    }
    let want_locked = !panel.open && !*released;

    for mut c in &mut cursors {
        let mode = if want_locked {
            bevy::window::CursorGrabMode::Locked
        } else {
            bevy::window::CursorGrabMode::None
        };
        if c.grab_mode != mode {
            c.grab_mode = mode;
            c.visible = !want_locked;
        }
    }
}
