use crate::{
    config::EngineConfig,
    utils::math::{AABB as MathAABB, Plane as MathPlane, ViewFrustum as MathViewFrustum},
    world::chunk_coord::ChunkCoord,
};
use glam::{Mat4, Vec3};
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct SpatialPartition {
    quadtree: QuadTree,
    lod_state: HashMap<ChunkCoord, u32>,
    spatial_index: BTreeMap<ChunkCoord, u32>,
    last_player_pos: Vec3,
}

impl SpatialPartition {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            quadtree: QuadTree::new(
                MathAABB {
                    min: Vec3::new(
                        -(config.render_distance as i32) as f32 * 32.0,
                        0.0,
                        -(config.render_distance as i32) as f32 * 32.0,
                    ),
                    max: Vec3::new(
                        config.render_distance as f32 * 32.0,
                        256.0,
                        config.render_distance as f32 * 32.0,
                    ),
                },
                4,
            ),
            lod_state: HashMap::new(),
            spatial_index: BTreeMap::new(),
            last_player_pos: Vec3::ZERO,
        }
    }

    fn update(&mut self, player_pos: Vec3, view_frustum: &MathViewFrustum, config: &EngineConfig) {
        if player_pos.distance(self.last_player_pos) > config.chunk_size as f32 * 0.5 {
            self.rebuild_quadtree(player_pos, config);
            self.last_player_pos = player_pos;
        }

        let visible = self.quadtree.query(view_frustum);
        self.update_lod(player_pos, &visible, config);
    }

    fn rebuild_quadtree(&mut self, center: Vec3, config: &EngineConfig) {
        self.quadtree = QuadTree::new(
            MathAABB {
                min: Vec3::new(
                    -(config.render_distance as i32) as f32 * 32.0,
                    0.0,
                    -(config.render_distance as i32) as f32 * 32.0,
                ),
                max: Vec3::new(
                    config.render_distance as f32 * 32.0,
                    256.0,
                    config.render_distance as f32 * 32.0,
                ),
            },
            4,
        );
        self.spatial_index.clear();

        let radius = config.render_distance as i32;
        let center_chunk = ChunkCoord::from_world_pos(center, config.chunk_size);

        for x in -radius..=radius {
            for z in -radius..=radius {
                let coord = ChunkCoord {
                    x: center_chunk.x + x,
                    y: center_chunk.y,
                    z: center_chunk.z + z,
                };
                let key = self.spatial_key(coord);
                self.spatial_index.insert(coord, key);
                self.quadtree.add_chunk(coord);
            }
        }
    }

    fn update_lod(&mut self, center: Vec3, visible: &[ChunkCoord], config: &EngineConfig) {
        for coord in visible {
            let distance = self.calculate_distance(center, coord, config.chunk_size);
            let lod = self.calculate_lod_level(distance, config);
            self.lod_state.insert(*coord, lod);
        }
    }

    fn calculate_distance(&self, pos: Vec3, coord: &ChunkCoord, chunk_size: u32) -> f32 {
        let chunk_center = coord.to_world_center(chunk_size);
        pos.distance(chunk_center)
    }

    fn calculate_lod_level(&self, distance: f32, config: &EngineConfig) -> u32 {
        if distance < config.view_distance * 0.3 {
            0
        } else if distance < config.view_distance * 0.6 {
            1
        } else {
            2
        }
    }

    fn spatial_key(&self, coord: ChunkCoord) -> u32 {
        ((coord.x as u32) << 16) | (coord.z as u32)
    }

    fn get_visible_chunks(&self) -> Vec<ChunkCoord> {
        self.spatial_index.keys().cloned().collect()
    }

    fn get_loading_priority(&self, player_pos: Vec3, chunk_size: u32) -> Vec<ChunkCoord> {
        let mut chunks: Vec<_> = self.spatial_index.keys().cloned().collect();
        chunks.sort_by_key(|c| {
            let center = c.to_world_center(chunk_size);
            (player_pos.distance(center) * 1000.0) as u32
        });
        chunks
    }
}

pub struct QuadTree {
    bounds: MathAABB,
    children: Option<Box<[QuadTree; 4]>>,
    depth: u32,
    chunks: Vec<ChunkCoord>,
}

impl QuadTree {
    pub fn new(bounds: MathAABB, max_depth: u32) -> Self {
        Self {
            bounds,
            children: None,
            depth: max_depth,
            chunks: Vec::new(),
        }
    }

    pub fn subdivide(&mut self) {
        if self.depth == 0 {
            return;
        }

        let center = (self.bounds.min + self.bounds.max) * 0.5;
        let mut children = Vec::with_capacity(4);

        for i in 0..4 {
            children.push(QuadTree::new(self.get_quadrant_bounds(i), self.depth - 1));
        }

        self.children = Some(Box::new([
            children.remove(0),
            children.remove(0),
            children.remove(0),
            children.remove(0),
        ]));
    }

    fn get_quadrant_bounds(&self, quadrant: usize) -> MathAABB {
        let center = (self.bounds.min + self.bounds.max) * 0.5;
        match quadrant {
            0 => MathAABB {
                min: self.bounds.min,
                max: center,
            },
            1 => MathAABB {
                min: Vec3::new(center.x, self.bounds.min.y, self.bounds.min.z),
                max: Vec3::new(self.bounds.max.x, center.y, center.z),
            },
            2 => MathAABB {
                min: Vec3::new(self.bounds.min.x, self.bounds.min.y, center.z),
                max: Vec3::new(center.x, center.y, self.bounds.max.z),
            },
            3 => MathAABB {
                min: center,
                max: self.bounds.max,
            },
            _ => panic!("Invalid quadrant index"),
        }
    }

    pub fn query(&self, frustum: &MathViewFrustum) -> Vec<ChunkCoord> {
        let mut result = Vec::new();
        if !self.bounds.intersects_frustum(frustum) {
            return result;
        }

        result.extend(self.chunks.iter().cloned());

        if let Some(children) = &self.children {
            for child in children.iter() {
                result.extend(child.query(frustum));
            }
        }

        result
    }

    pub fn add_chunk(&mut self, coord: ChunkCoord) {
        self.chunks.push(coord);
    }

    pub fn get_chunks(&self) -> &[ChunkCoord] {
        &self.chunks
    }
}

impl MathAABB {
    fn intersects_frustum(&self, frustum: &MathViewFrustum) -> bool {
        for plane in &frustum.planes {
            let mut min_point = Vec3::new(self.min.x, self.min.y, self.min.z);
            let mut max_point = Vec3::new(self.max.x, self.max.y, self.max.z);

            if plane.normal.x > 0.0 {
                min_point.x = self.max.x;
                max_point.x = self.min.x;
            }
            if plane.normal.y > 0.0 {
                min_point.y = self.max.y;
                max_point.y = self.min.y;
            }
            if plane.normal.z > 0.0 {
                min_point.z = self.max.z;
                max_point.z = self.min.z;
            }

            if plane.normal.dot(min_point) + plane.distance < 0.0 {
                return false;
            }
        }
        true
    }
}

pub struct ViewFrustum {
    pub planes: [MathPlane; 6],
}

impl ViewFrustum {
    pub fn new() -> Self {
        Self {
            planes: [MathPlane::default(); 6],
        }
    }

    pub fn from_matrices(view: &Mat4, proj: &Mat4) -> Self {
        let view_proj = proj * view;
        let mut frustum = Self::new();

        // Left plane
        frustum.planes[0] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w + view_proj.x_axis.x,
                view_proj.y_axis.w + view_proj.y_axis.x,
                view_proj.z_axis.w + view_proj.z_axis.x,
            ),
            distance: view_proj.w_axis.w + view_proj.w_axis.x,
        };

        // Right plane
        frustum.planes[1] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w - view_proj.x_axis.x,
                view_proj.y_axis.w - view_proj.y_axis.x,
                view_proj.z_axis.w - view_proj.z_axis.x,
            ),
            distance: view_proj.w_axis.w - view_proj.w_axis.x,
        };

        // Bottom plane
        frustum.planes[2] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w + view_proj.x_axis.y,
                view_proj.y_axis.w + view_proj.y_axis.y,
                view_proj.z_axis.w + view_proj.z_axis.y,
            ),
            distance: view_proj.w_axis.w + view_proj.w_axis.y,
        };

        // Top plane
        frustum.planes[3] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w - view_proj.x_axis.y,
                view_proj.y_axis.w - view_proj.y_axis.y,
                view_proj.z_axis.w - view_proj.z_axis.y,
            ),
            distance: view_proj.w_axis.w - view_proj.w_axis.y,
        };

        // Near plane
        frustum.planes[4] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w + view_proj.x_axis.z,
                view_proj.y_axis.w + view_proj.y_axis.z,
                view_proj.z_axis.w + view_proj.z_axis.z,
            ),
            distance: view_proj.w_axis.w + view_proj.w_axis.z,
        };

        // Far plane
        frustum.planes[5] = MathPlane {
            normal: Vec3::new(
                view_proj.x_axis.w - view_proj.x_axis.z,
                view_proj.y_axis.w - view_proj.y_axis.z,
                view_proj.z_axis.w - view_proj.z_axis.z,
            ),
            distance: view_proj.w_axis.w - view_proj.w_axis.z,
        };

        // Normalize all planes
        for plane in &mut frustum.planes {
            let length = plane.normal.length();
            plane.normal /= length;
            plane.distance /= length;
        }

        frustum
    }

    pub fn intersects_frustum(&self, frustum: &MathViewFrustum) -> bool {
        for plane in &self.planes {
            if !frustum.intersects_plane(plane) {
                return false;
            }
        }
        true
    }
}

pub struct SpatialIndex {
    chunks: Vec<ChunkCoord>,
    center: Vec3,
    radius: f32,
    chunk_size: u32,
}

impl SpatialIndex {
    pub fn new(center: Vec3, radius: f32, chunk_size: u32) -> Self {
        let chunk_size = chunk_size as i32;
        let center_chunk = ChunkCoord::from_world_pos(center, chunk_size);
        let radius_chunks = (radius / chunk_size as f32).ceil() as i32;

        let mut chunks = Vec::new();
        for x in -radius_chunks..=radius_chunks {
            for y in -radius_chunks..=radius_chunks {
                for z in -radius_chunks..=radius_chunks {
                    let coord = ChunkCoord::new(
                        center_chunk.x() + x,
                        center_chunk.y() + y,
                        center_chunk.z() + z,
                    );
                    let chunk_center = coord.to_world_center(chunk_size);
                    let distance = (chunk_center - center).length();
                    if distance <= radius {
                        chunks.push(coord);
                    }
                }
            }
        }

        Self {
            chunks,
            center,
            radius,
            chunk_size: chunk_size as u32,
        }
    }

    pub fn get_chunk_key(&self, coord: &ChunkCoord) -> u32 {
        ((coord.x() as u32) << 16) | (coord.z() as u32)
    }

    pub fn get_chunks_in_frustum(&self, frustum: &ViewFrustum) -> Vec<ChunkCoord> {
        self.chunks
            .iter()
            .filter(|c| {
                let center = c.to_world_center(self.chunk_size as i32);
                let half_size = self.chunk_size as f32 * 0.5;
                let aabb = MathAABB {
                    min: center - Vec3::splat(half_size),
                    max: center + Vec3::splat(half_size),
                };
                frustum.intersects_aabb(&aabb)
            })
            .copied()
            .collect()
    }
}
