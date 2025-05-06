use glam::{Vec3, Mat4, Vec2, Vec4, IVec3};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    pub fn new(normal: Vec3, point: Vec3) -> Self {
        Self {
            normal: normal.normalize(),
            distance: normal.dot(point),
        }
    }

    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) - self.distance
    }
}

#[derive(Debug, Clone)]
pub struct ViewFrustum {
    pub planes: [Plane; 6],
}

impl ViewFrustum {
    pub fn new(view_proj: Mat4) -> Self {
        let mut planes = [
            Plane { normal: Vec3::ZERO, distance: 0.0 },
            Plane { normal: Vec3::ZERO, distance: 0.0 },
            Plane { normal: Vec3::ZERO, distance: 0.0 },
            Plane { normal: Vec3::ZERO, distance: 0.0 },
            Plane { normal: Vec3::ZERO, distance: 0.0 },
            Plane { normal: Vec3::ZERO, distance: 0.0 },
        ];

        // Extract planes from view-projection matrix
        for i in 0..6 {
            let row = match i {
                0 => view_proj.row(3) + view_proj.row(0), // Right
                1 => view_proj.row(3) - view_proj.row(0), // Left
                2 => view_proj.row(3) + view_proj.row(1), // Top
                3 => view_proj.row(3) - view_proj.row(1), // Bottom
                4 => view_proj.row(3) + view_proj.row(2), // Far
                5 => view_proj.row(3) - view_proj.row(2), // Near
                _ => unreachable!(),
            };

            let normal = Vec3::new(row.x, row.y, row.z);
            let length = normal.length();
            
            planes[i] = Plane {
                normal: normal / length,
                distance: row.w / length,
            };
        }

        Self { planes }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false;
            }
        }
        true
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
}

impl Default for Orientation {
    fn default() -> Self {
        Self::North
    }
}

impl Orientation {
    pub fn to_vec3(&self) -> Vec3 {
        match self {
            Self::North => Vec3::new(0.0, 0.0, -1.0),
            Self::South => Vec3::new(0.0, 0.0, 1.0),
            Self::East => Vec3::new(1.0, 0.0, 0.0),
            Self::West => Vec3::new(-1.0, 0.0, 0.0),
            Self::Up => Vec3::new(0.0, 1.0, 0.0),
            Self::Down => Vec3::new(0.0, -1.0, 0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
}

pub trait VoxelPosition {
    fn to_chunk_coord(&self, chunk_size: u32) -> IVec3;
    fn to_block_index(&self, chunk_size: u32) -> IVec3;
}

impl VoxelPosition for Vec3 {
    fn to_chunk_coord(&self, chunk_size: u32) -> IVec3 {
        IVec3::new(
            (self.x / chunk_size as f32).floor() as i32,
            (self.y / chunk_size as f32).floor() as i32,
            (self.z / chunk_size as f32).floor() as i32,
        )
    }

    fn to_block_index(&self, chunk_size: u32) -> IVec3 {
        IVec3::new(
            self.x.rem_euclid(chunk_size as f32) as i32,
            self.y.rem_euclid(chunk_size as f32) as i32,
            self.z.rem_euclid(chunk_size as f32) as i32,
        )
    }
}
