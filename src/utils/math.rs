//! src/utils/math.rs
//! Mathematical utilities and geometric types
use bitflags::bitflags;
use glam::{IVec3, Mat4, Quat, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        self.contains(point)
    }

    pub fn transform(&self, transform: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for corner in corners {
            let transformed = transform.transform_point3(corner);
            min = min.min(transformed);
            max = max.max(transformed);
        }

        Self { min, max }
    }
}

/// View frustum for culling
#[derive(Debug, Clone)]
pub struct ViewFrustum {
    pub planes: [Plane; 6],
}

impl ViewFrustum {
    pub fn new() -> Self {
        Self {
            planes: [
                Plane::default(),
                Plane::default(),
                Plane::default(),
                Plane::default(),
                Plane::default(),
                Plane::default(),
            ],
        }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.signed_distance(point) >= 0.0)
    }

    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if plane.normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if plane.normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if plane.signed_distance(p) < 0.0 {
                return false;
            }
        }
        true
    }
}

/// Geometric plane
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub fn normalize(&mut self) {
        let length = self.normal.length();
        if length > 0.0 {
            self.normal /= length;
            self.distance /= length;
        }
    }

    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }

    pub fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self {
            normal: Vec3::ZERO,
            distance: 0.0,
        }
    }
}

/// Raycasting utilities
pub mod raycast {
    use glam::Vec3;

    pub struct Ray {
        pub origin: Vec3,
        pub direction: Vec3,
    }

    impl Ray {
        pub fn new(origin: Vec3, direction: Vec3) -> Self {
            Self {
                origin,
                direction: direction.normalize(),
            }
        }

        pub fn intersect_aabb(&self, aabb: &super::AABB) -> Option<f32> {
            let mut tmin = f32::MIN;
            let mut tmax = f32::MAX;

            for i in 0..3 {
                if self.direction[i].abs() < f32::EPSILON {
                    // Ray is parallel to slab. No hit if origin not within slab
                    if self.origin[i] < aabb.min[i] || self.origin[i] > aabb.max[i] {
                        return None;
                    }
                } else {
                    let inv_d = 1.0 / self.direction[i];
                    let mut t1 = (aabb.min[i] - self.origin[i]) * inv_d;
                    let mut t2 = (aabb.max[i] - self.origin[i]) * inv_d;

                    if t1 > t2 {
                        std::mem::swap(&mut t1, &mut t2);
                    }

                    tmin = tmin.max(t1);
                    tmax = tmax.min(t2);

                    if tmin > tmax {
                        return None;
                    }
                }
            }

            Some(tmin)
        }

        pub fn intersect_plane(&self, plane: &super::Plane) -> Option<f32> {
            let denom = plane.normal.dot(self.direction);
            if denom.abs() > f32::EPSILON {
                let t = (-plane.distance - plane.normal.dot(self.origin)) / denom;
                if t >= 0.0 {
                    return Some(t);
                }
            }
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Orientation {
    North,
    South,
    East,
    West,
    Up,
    Down,
    Custom(f32, f32, f32, f32),
    None,
}

impl Default for Orientation {
    fn default() -> Self {
        Self::North
    }
}

impl Orientation {
    pub fn to_matrix(&self) -> glam::Mat4 {
        match self {
            Orientation::North => glam::Mat4::IDENTITY,
            Orientation::South => glam::Mat4::from_rotation_y(std::f32::consts::PI),
            Orientation::East => glam::Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Orientation::West => glam::Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            Orientation::Up => glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            Orientation::Down => glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
            Orientation::Custom(x, y, z, w) => {
                glam::Mat4::from_quat(glam::Quat::from_xyzw(*x, *y, *z, *w))
            }
            Orientation::None => glam::Mat4::IDENTITY,
        }
    }

    pub fn facing(&self) -> glam::Vec3 {
        match self {
            Orientation::North => glam::Vec3::NEG_Z,
            Orientation::South => glam::Vec3::Z,
            Orientation::East => glam::Vec3::X,
            Orientation::West => glam::Vec3::NEG_X,
            Orientation::Up => glam::Vec3::Y,
            Orientation::Down => glam::Vec3::NEG_Y,
            Orientation::Custom(x, y, z, w) => {
                let quat = glam::Quat::from_xyzw(*x, *y, *z, *w);
                quat.mul_vec3(glam::Vec3::NEG_Z)
            }
            Orientation::None => glam::Vec3::ZERO,
        }
    }

    pub fn from_quat(quat: Quat) -> Self {
        // Compare with standard orientations first
        let angles = quat.to_euler(glam::EulerRot::YXZ);

        // Check for cardinal directions
        if (angles.0 - 0.0).abs() < f32::EPSILON
            && (angles.1 - 0.0).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::North;
        }

        if (angles.0 - std::f32::consts::PI).abs() < f32::EPSILON
            && (angles.1 - 0.0).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::South;
        }

        if (angles.0 - std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON
            && (angles.1 - 0.0).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::East;
        }

        if (angles.0 - (-std::f32::consts::FRAC_PI_2)).abs() < f32::EPSILON
            && (angles.1 - 0.0).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::West;
        }

        if (angles.0 - 0.0).abs() < f32::EPSILON
            && (angles.1 - (-std::f32::consts::FRAC_PI_2)).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::Up;
        }

        if (angles.0 - 0.0).abs() < f32::EPSILON
            && (angles.1 - std::f32::consts::FRAC_PI_2).abs() < f32::EPSILON
            && (angles.2 - 0.0).abs() < f32::EPSILON
        {
            return Orientation::Down;
        }

        // If not a standard orientation, return custom
        Orientation::Custom(quat.x, quat.y, quat.z, quat.w)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct ConnectedDirections: u8 {
        const NORTH = 0b00000001;
        const SOUTH = 0b00000010;
        const EAST = 0b00000100;
        const WEST = 0b00001000;
        const UP = 0b00010000;
        const DOWN = 0b00100000;
    }
}

impl ConnectedDirections {
    pub fn to_direction(&self) -> Vec3 {
        let mut result = Vec3::ZERO;

        if self.contains(ConnectedDirections::NORTH) {
            result += Vec3::NEG_Z;
        }
        if self.contains(ConnectedDirections::SOUTH) {
            result += Vec3::Z;
        }
        if self.contains(ConnectedDirections::EAST) {
            result += Vec3::X;
        }
        if self.contains(ConnectedDirections::WEST) {
            result += Vec3::NEG_X;
        }
        if self.contains(ConnectedDirections::UP) {
            result += Vec3::Y;
        }
        if self.contains(ConnectedDirections::DOWN) {
            result += Vec3::NEG_Y;
        }

        result.normalize_or_zero()
    }
}
