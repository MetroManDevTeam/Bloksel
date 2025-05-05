//! src/utils/math.rs
//! Mathematical utilities and geometric types

use glam::{Vec3, Vec4, Mat4};

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

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x &&
        self.max.x >= other.min.x &&
        self.min.y <= other.max.y &&
        self.max.y >= other.min.y &&
        self.min.z <= other.max.z &&
        self.max.z >= other.min.z
    }
}

/// View frustum for culling
pub struct ViewFrustum {
    pub planes: [Plane; 6],
}

impl ViewFrustum {
    pub fn from_matrices(view: &Mat4, proj: &Mat4) -> Self {
        let vp = proj * view;
        let mut planes = [Plane::default(); 6];
        
        // Plane extraction logic...
        planes
    }
}

/// Geometric plane
#[derive(Default, Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    pub fn normalize(&mut self) {
        let length = self.normal.length();
        self.normal /= length;
        self.distance /= length;
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
            Self { origin, direction: direction.normalize() }
        }

        pub fn intersect_aabb(&self, aabb: &super::AABB) -> Option<f32> {
            // Ray-AABB intersection implementation...
            None
        }
    }
}
