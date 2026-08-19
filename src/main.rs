//! Graybox seed for the labyrinth game.
//!
//! Opens a window with a 3D camera, a flat ground plane, one cube, and one
//! directional light. Untextured grey materials throughout. This is a
//! rendering smoke test and a starting point — not a game.
//!
//! API verified against bevy 0.19.1 (see CLAUDE.md: Bevy is pre-1.0 and its
//! API drifts between releases, so confirm against the installed version).
//!
//! # SPEC
//!
//! Every line below is checked by a test in the `tests` module.
//!
//! - S1: `setup` spawns exactly one `DirectionalLight`.
//! - S2: `setup` spawns exactly one `Camera3d`.
//! - S3: `setup` spawns exactly two `Mesh3d` entities (the ground and the cube).
//! - S4: the ground sits at y = 0.0 and the cube's centre sits at
//!   y = `CUBE_SIZE` / 2.0, i.e. the cube rests on the plane rather than
//!   intersecting it or floating above it.

use bevy::prelude::*;

/// Ground plane edge length, in world units (metres).
const GROUND_SIZE: f32 = 20.0;

/// Cube edge length, in world units (metres).
const CUBE_SIZE: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Flat, untextured greys. perceptual_roughness is set to 1.0 explicitly so
    // surfaces read as matte graybox rather than picking up specular highlights.
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.30),
        perceptual_roughness: 1.0,
        ..default()
    });
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.60, 0.60, 0.60),
        perceptual_roughness: 1.0,
        ..default()
    });

    // Ground plane, centred on the origin, facing +Y. (S3, S4)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE))),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // One cube, resting on the plane: its origin is at its centre, so it is
    // lifted by half its height. (S3, S4)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE))),
        MeshMaterial3d(cube_material),
        Transform::from_xyz(0.0, CUBE_SIZE / 2.0, 0.0),
    ));

    // Directional light. Only the rotation matters for a directional light —
    // the translation just makes `looking_at` readable. (S1)
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Camera, looking down at the origin. (S2)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-4.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetApp, AssetPlugin};

    /// Headless app with just enough registered to run `setup`:
    /// no window, no renderer, but `Assets<Mesh>` and
    /// `Assets<StandardMaterial>` exist so the system's params resolve.
    fn run_setup() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_systems(Startup, setup);
        app.update();
        app
    }

    /// S1
    #[test]
    fn spawns_exactly_one_directional_light() {
        let mut app = run_setup();
        let count = app
            .world_mut()
            .query::<&DirectionalLight>()
            .iter(app.world())
            .len();
        assert_eq!(count, 1, "expected exactly one directional light");
    }

    /// S2
    #[test]
    fn spawns_exactly_one_camera() {
        let mut app = run_setup();
        let count = app.world_mut().query::<&Camera3d>().iter(app.world()).len();
        assert_eq!(count, 1, "expected exactly one 3D camera");
    }

    /// S3
    #[test]
    fn spawns_ground_and_cube_meshes() {
        let mut app = run_setup();
        let count = app.world_mut().query::<&Mesh3d>().iter(app.world()).len();
        assert_eq!(count, 2, "expected exactly two meshes: ground and cube");
    }

    /// S4 — the only claim in this file that could silently be wrong.
    #[test]
    fn cube_rests_on_ground_plane() {
        let mut app = run_setup();
        let mut heights: Vec<f32> = app
            .world_mut()
            .query::<(&Mesh3d, &Transform)>()
            .iter(app.world())
            .map(|(_, transform)| transform.translation.y)
            .collect();
        heights.sort_by(|a, b| a.partial_cmp(b).expect("no NaN heights"));

        assert_eq!(heights.len(), 2, "expected two mesh entities");
        assert!(
            (heights[0] - 0.0).abs() < f32::EPSILON,
            "ground should sit at y = 0.0, got {}",
            heights[0]
        );
        assert!(
            (heights[1] - CUBE_SIZE / 2.0).abs() < f32::EPSILON,
            "cube centre should sit at y = {}, got {}",
            CUBE_SIZE / 2.0,
            heights[1]
        );
    }
}
