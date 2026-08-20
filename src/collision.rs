//! Hand-rolled axis-aligned collision.
//!
//! The level is boxes, so AABB-vs-AABB is exact here and a physics engine
//! would add indirection without adding accuracy. Just as importantly:
//! nothing in this module can push the player. Movement is whatever the
//! caller asks for, minus whatever geometry blocks. That makes
//! "physics never drives the character" structural rather than a setting.
//!
//! The player renders as a capsule but collides as its bounding box. For
//! boxy rooms the difference only shows at box edges.
//!
//! # SPEC
//!
//! - C1: with no obstacles, the full delta is applied.
//! - C2: falling onto a floor leaves the body's feet exactly on the floor's
//!   top surface, and reports grounded.
//! - C3: moving into a wall stops flush against it, while motion along the
//!   other axes still applies (slide, not stick).
//! - C4: moving up into a ceiling stops with the head just below it, and
//!   reports a ceiling hit.
//! - C5: moving horizontally along a floor stays grounded and does not sink.

use bevy::prelude::*;

/// An axis-aligned box, stored as centre plus half-extents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub center: Vec3,
    pub half: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, half: Vec3) -> Self {
        Self { center, half }
    }

    /// Build from a floor-standing box: `base` is the centre of its bottom
    /// face, which is how level geometry is naturally described.
    pub fn from_base(base: Vec3, half: Vec3) -> Self {
        Self {
            center: Vec3::new(base.x, base.y + half.y, base.z),
            half,
        }
    }

    pub fn min(&self) -> Vec3 {
        self.center - self.half
    }

    pub fn max(&self) -> Vec3 {
        self.center + self.half
    }

    /// Strict overlap on all three axes. Touching faces do not count as
    /// overlapping, so a body resting exactly on a floor is not "inside" it.
    pub fn intersects(&self, other: &Aabb) -> bool {
        let (a_min, a_max) = (self.min(), self.max());
        let (b_min, b_max) = (other.min(), other.max());
        a_min.x < b_max.x
            && a_max.x > b_min.x
            && a_min.y < b_max.y
            && a_max.y > b_min.y
            && a_min.z < b_max.z
            && a_max.z > b_min.z
    }
}

/// Outcome of a movement attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveResult {
    /// Final centre position after resolution.
    pub position: Vec3,
    /// True if downward motion was stopped by geometry this step.
    pub grounded: bool,
    /// True if upward motion was stopped by geometry this step.
    pub hit_ceiling: bool,
    /// True if motion on X or Z was stopped by geometry this step.
    pub hit_wall: bool,
}

/// Move `body` by `delta`, resolving against static `blockers`.
///
/// Axes resolve independently and in the order Y, X, Z. Resolving Y first
/// means the body lands before it tries to move along the surface, which
/// avoids catching on the lip of the floor it is standing on.
pub fn move_and_slide(body: Aabb, delta: Vec3, blockers: &[Aabb]) -> MoveResult {
    let mut body = body;

    // Y first: land before sliding along the surface, so the body does not
    // catch on the lip of the floor it is standing on.
    let (blocked_y, _) = sweep(&mut body, delta.y, 1, blockers);
    let grounded = blocked_y && delta.y < 0.0;
    let hit_ceiling = blocked_y && delta.y > 0.0;

    let (blocked_x, _) = sweep(&mut body, delta.x, 0, blockers);
    let (blocked_z, _) = sweep(&mut body, delta.z, 2, blockers);

    MoveResult {
        position: body.center,
        grounded,
        hit_ceiling,
        hit_wall: blocked_x || blocked_z,
    }
}

fn axis_get(v: Vec3, a: usize) -> f32 {
    match a {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn axis_set(v: &mut Vec3, a: usize, val: f32) {
    match a {
        0 => v.x = val,
        1 => v.y = val,
        _ => v.z = val,
    }
}

/// Do the body and blocker overlap on the two axes that are *not* `axis`?
/// Only then can motion along `axis` be blocked by it.
fn overlaps_other_axes(body: &Aabb, b: &Aabb, axis: usize) -> bool {
    let (bmin, bmax) = (body.min(), body.max());
    let (omin, omax) = (b.min(), b.max());
    for i in 0..3 {
        if i == axis {
            continue;
        }
        if !(axis_get(bmin, i) < axis_get(omax, i) && axis_get(bmax, i) > axis_get(omin, i)) {
            return false;
        }
    }
    true
}

/// Move `body` along one axis by `d`, stopping at the first surface crossed.
///
/// This is a swept test, not a move-then-push-out: a body falling fast enough
/// to pass entirely through a thin floor in one step must still land on it.
/// Returns whether motion was blocked.
fn sweep(body: &mut Aabb, d: f32, axis: usize, blockers: &[Aabb]) -> (bool, f32) {
    if d == 0.0 {
        return (false, 0.0);
    }
    const EPS: f32 = 1e-6;
    let old_min = axis_get(body.min(), axis);
    let old_max = axis_get(body.max(), axis);
    let half = axis_get(body.half, axis);

    let mut surface: Option<f32> = None;
    if d < 0.0 {
        let new_min = old_min + d;
        for b in blockers {
            if !overlaps_other_axes(body, b, axis) {
                continue;
            }
            // Top face of the blocker, approached from above.
            let s = axis_get(b.max(), axis);
            if s <= old_min + EPS && s > new_min {
                surface = Some(surface.map_or(s, |cur: f32| cur.max(s)));
            }
        }
        if let Some(s) = surface {
            let mut c = body.center;
            axis_set(&mut c, axis, s + half);
            body.center = c;
            return (true, s);
        }
    } else {
        let new_max = old_max + d;
        for b in blockers {
            if !overlaps_other_axes(body, b, axis) {
                continue;
            }
            // Bottom face of the blocker, approached from below.
            let s = axis_get(b.min(), axis);
            if s >= old_max - EPS && s < new_max {
                surface = Some(surface.map_or(s, |cur: f32| cur.min(s)));
            }
        }
        if let Some(s) = surface {
            let mut c = body.center;
            axis_set(&mut c, axis, s - half);
            body.center = c;
            return (true, s);
        }
    }

    let mut c = body.center;
    let moved = axis_get(c, axis) + d;
    axis_set(&mut c, axis, moved);
    body.center = c;
    (false, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1x1x1 body (half-extents 0.5) centred at `at`.
    fn body_at(at: Vec3) -> Aabb {
        Aabb::new(at, Vec3::splat(0.5))
    }

    /// A wide floor slab whose top surface sits at y = 0.
    fn floor() -> Aabb {
        Aabb::from_base(Vec3::new(0.0, -1.0, 0.0), Vec3::new(50.0, 0.5, 50.0))
    }

    /// C1
    #[test]
    fn unobstructed_move_applies_full_delta() {
        let start = body_at(Vec3::new(0.0, 5.0, 0.0));
        let delta = Vec3::new(1.0, -2.0, 3.0);
        let r = move_and_slide(start, delta, &[]);
        assert!(
            (r.position - (start.center + delta)).length() < 1e-5,
            "expected {:?}, got {:?}",
            start.center + delta,
            r.position
        );
        assert!(!r.grounded, "nothing to stand on, should not be grounded");
    }

    /// C2 - the load-bearing one. Feet must land exactly on the surface.
    #[test]
    fn falling_lands_exactly_on_floor() {
        let start = body_at(Vec3::new(0.0, 3.0, 0.0));
        // Try to fall far below the floor.
        let r = move_and_slide(start, Vec3::new(0.0, -10.0, 0.0), &[floor()]);
        assert!(r.grounded, "should report grounded after landing");
        // Body half-height 0.5, floor top at y=0, so centre must rest at 0.5.
        assert!(
            (r.position.y - 0.5).abs() < 1e-5,
            "expected centre y=0.5 (feet on floor), got {}",
            r.position.y
        );
    }

    /// C3 - stopping on one axis must not cancel the others.
    #[test]
    fn wall_blocks_one_axis_and_slides_on_the_other() {
        // Wall occupying x in [2, 3], tall and long.
        let wall = Aabb::from_base(Vec3::new(2.5, 0.0, 0.0), Vec3::new(0.5, 5.0, 50.0));
        let start = body_at(Vec3::new(0.0, 0.5, 0.0));
        // Push hard into the wall while also moving along z.
        let r = move_and_slide(start, Vec3::new(5.0, 0.0, 2.0), &[wall]);
        assert!(r.hit_wall, "should report a wall hit");
        assert!(
            (r.position.x - 1.5).abs() < 1e-5,
            "expected to stop flush at x=1.5 (wall min 2.0 minus half 0.5), got {}",
            r.position.x
        );
        assert!(
            (r.position.z - 2.0).abs() < 1e-5,
            "z motion should still apply while x is blocked, got {}",
            r.position.z
        );
    }

    /// C4
    #[test]
    fn ceiling_stops_upward_motion() {
        // Ceiling slab occupying y in [4, 5].
        let ceiling = Aabb::from_base(Vec3::new(0.0, 4.0, 0.0), Vec3::new(50.0, 0.5, 50.0));
        let start = body_at(Vec3::new(0.0, 1.0, 0.0));
        let r = move_and_slide(start, Vec3::new(0.0, 10.0, 0.0), &[ceiling]);
        assert!(r.hit_ceiling, "should report a ceiling hit");
        assert!(
            (r.position.y - 3.5).abs() < 1e-5,
            "expected centre y=3.5 (ceiling min 4.0 minus half 0.5), got {}",
            r.position.y
        );
    }

    /// C5 - walking must not sink into the surface being walked on.
    #[test]
    fn walking_along_floor_stays_grounded() {
        // Resting exactly on the floor, with the small downward probe that
        // gravity applies every frame.
        let start = body_at(Vec3::new(0.0, 0.5, 0.0));
        let r = move_and_slide(start, Vec3::new(3.0, -0.01, 0.0), &[floor()]);
        assert!(r.grounded, "should stay grounded while walking");
        assert!(
            (r.position.y - 0.5).abs() < 1e-5,
            "should not sink; expected y=0.5, got {}",
            r.position.y
        );
        assert!(
            (r.position.x - 3.0).abs() < 1e-5,
            "horizontal motion should apply fully, got {}",
            r.position.x
        );
    }
}
