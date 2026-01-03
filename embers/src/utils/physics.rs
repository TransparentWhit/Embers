use avian3d::math::{Quaternion, Vector};
use avian3d::prelude::*;
use std::f32::consts::PI;

pub fn section(mut arc_rad: f32, radius: f32, height: f32) -> Option<Collider> {
    arc_rad = arc_rad.abs();
    if arc_rad >= 2. * PI {
        return Some(Collider::cylinder(radius, height));
    }
    if arc_rad > PI {
        let half = section(0.5 * arc_rad, radius, height)?;
        return Some(Collider::compound(vec![
            (
                Position(Vector::ZERO),
                Rotation(Quaternion::from_rotation_y(0.25 * arc_rad)),
                half.clone(),
            ),
            (
                Position(Vector::ZERO),
                Rotation(Quaternion::from_rotation_y(-0.25 * arc_rad)),
                half,
            ),
        ]));
    }
    let segments = (arc_rad / (1f32 - 1. / 64.).acos() + 1.).ceil() as usize;
    let mut points = Vec::with_capacity(segments + 1);
    points.push(Vector::new(0., -0.5 * height, 0.));
    points.push(Vector::new(0., 0.5 * height, 0.));
    for segment in 0..segments {
        let angle = arc_rad * (segment as f32 / segments as f32 - 0.5);
        points.push(Vector::new(
            angle.cos() * radius,
            -0.5 * height,
            angle.sin() * radius,
        ));
        points.push(Vector::new(
            angle.cos() * radius,
            0.5 * height,
            angle.sin() * radius,
        ));
    }
    Collider::convex_hull(points)
}
