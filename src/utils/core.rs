//! src/utils/mod.rs
//! Core utilities used throughout the engine

use crate::utils::core::error::CoreError;
use crate::utils::math;
use glam::{IVec3, Mat4, Vec3, Vec4};
use std::time::{Duration, Instant};
use thiserror::Error;

pub mod error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Resource loading error: {0}")]
    ResourceError(String),

    #[error("Threading error: {0}")]
    ThreadError(String),
}

/// Frame timing statistics
#[derive(Debug, Clone)]
pub struct FrameTiming {
    pub delta_time: f32,
    pub fps: f32,
    pub frame_count: u64,
}

/// Performance profiler
pub struct Profiler {
    start_time: Instant,
    last_frame: Instant,
    frame_count: u64,
    frustum: math::ViewFrustum,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
            frustum: math::ViewFrustum::from_matrices(&Mat4::IDENTITY, &Mat4::IDENTITY),
        }
    }

    pub fn begin_frame(&mut self) -> Duration {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;
        self.frame_count += 1;
        delta
    }

    pub fn get_timing(&self) -> FrameTiming {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        FrameTiming {
            delta_time,
            fps: 1.0 / delta_time,
            frame_count: self.frame_count,
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(45.0f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y)
    }

    pub fn update_frustum(&mut self) {
        let view_proj = self.projection_matrix() * self.view_matrix();
        self.frustum = math::ViewFrustum::from_matrices(&view_proj, &Mat4::IDENTITY);
    }
}

/// Coordinate system conversions
pub trait CoordinateExtensions {
    fn to_chunk_coord(&self, chunk_size: u32) -> IVec3;
    fn to_block_index(&self, chunk_size: u32) -> IVec3;
}

impl CoordinateExtensions for Vec3 {
    fn to_chunk_coord(&self, chunk_size: u32) -> IVec3 {
        IVec3::new(
            (self.x / chunk_size as f32).floor() as i32,
            (self.y / chunk_size as f32).floor() as i32,
            (self.z / chunk_size as f32).floor() as i32,
        )
    }

    fn to_block_index(&self, chunk_size: u32) -> IVec3 {
        IVec3::new(
            (self.x % chunk_size as f32) as i32,
            (self.y % chunk_size as f32) as i32,
            (self.z % chunk_size as f32) as i32,
        )
    }
}

/// Extension methods for matrices
pub trait MatrixExtensions {
    fn to_view_frustum(&self) -> math::ViewFrustum;
}

impl MatrixExtensions for Mat4 {
    fn to_view_frustum(&self) -> math::ViewFrustum {
        math::ViewFrustum::from_matrices(self, &Mat4::IDENTITY)
    }
}

/// Thread-safe atomic counter
#[derive(Default)]
pub struct AtomicCounter {
    count: std::sync::atomic::AtomicU64,
}

impl AtomicCounter {
    pub fn increment(&self) -> u64 {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get(&self) -> u64 {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy)]
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

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}
