//! src/utils/math.rs
//! Mathematical utilities and geometric types
use bitflags::bitflags;
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
        Orientation::North
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
            Orientation::Custom(x, y, z, w) => glam::Mat4::from_quat(glam::Quat::from_xyzw(*x, *y, *z, *w)),
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
            
