use glam::{Vec3, Mat4};
use crate::{
    config::EngineConfig,
    utils::math::{ViewFrustum, AABB},
    world::chunk::ChunkCoord
};



struct ViewFrustum {
    planes: [Plane; 6],
}



#[derive(Default, Copy, Clone)]
struct Plane {
    normal: Vec3,
    distance: f32,
}

struct AABB {
    min: Vec3,
    max: Vec3,
}

struct SpatialPartition {
    quadtree: QuadTree,
    lod_state: HashMap<ChunkCoord, u32>,
    spatial_index: BTreeMap<u64, ChunkCoord>,
    last_player_pos: Vec3,
}

impl SpatialPartition {
    fn new(config: &EngineConfig) -> Self {
        Self {
            quadtree: QuadTree::new(config.render_distance),
            lod_state: HashMap::new(),
            spatial_index: BTreeMap::new(),
            last_player_pos: Vec3::ZERO,
        }
    }

    fn update(&mut self, player_pos: Vec3, view_frustum: &ViewFrustum, config: &EngineConfig) {
        if player_pos.distance(self.last_player_pos) > config.chunk_size as f32 * 0.5 {
            self.rebuild_quadtree(player_pos, config);
            self.last_player_pos = player_pos;
        }

        let visible = self.quadtree.query(view_frustum);
        self.update_lod(player_pos, &visible, config);
    }

    fn rebuild_quadtree(&mut self, center: Vec3, config: &EngineConfig) {
        self.quadtree = QuadTree::new(config.render_distance);
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
                self.spatial_index.insert(key, coord);
                self.quadtree.insert(coord);
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

    fn spatial_key(&self, coord: ChunkCoord) -> u64 {
        ((coord.x as u64) << 32) | (coord.z as u64)
    }

    fn get_visible_chunks(&self) -> Vec<ChunkCoord> {
        self.spatial_index.values().cloned().collect()
    }

    fn get_loading_priority(&self, player_pos: Vec3, chunk_size: u32) -> Vec<ChunkCoord> {
        let mut chunks: Vec<_> = self.spatial_index.values().cloned().collect();
        chunks.sort_by_key(|c| {
            let center = c.to_world_center(chunk_size);
            (player_pos.distance(center) * 1000.0) as u32
        });
        chunks
    }
}

struct QuadTree {
    nodes: [Option<Box<QuadTree>>; 4],
    chunks: Vec<ChunkCoord>,
    bounds: AABB,
    depth: u8,
}

impl QuadTree {
    pub fn new(render_distance: u32) -> Self {
        let size = render_distance as f32 * 32.0; // Assuming 32 blocks per chunk
        Self {
            nodes: [None, None, None, None],
            chunks: Vec::new(),
            bounds: AABB {
                min: Vec3::new(-size, 0.0, -size),
                max: Vec3::new(size, 256.0, size),
            },
            depth: 0,
        }
    }

    pub fn insert(&mut self, coord: ChunkCoord) {
        if !self.bounds.contains(coord) {
            return;
        }

        if self.depth < 4 {
            let quadrant = self.get_quadrant(coord);
            match &mut self.nodes[quadrant] {
                Some(node) => node.insert(coord),
                None => {
                    let mut new_node = QuadTree {
                        nodes: [None, None, None, None],
                        chunks: Vec::new(),
                        bounds: self.get_quadrant_bounds(quadrant),
                        depth: self.depth + 1,
                    };
                    new_node.insert(coord);
                    self.nodes[quadrant] = Some(Box::new(new_node));
                }
            }
        } else {
            self.chunks.push(coord);
        }
    }

    fn get_quadrant(&self, coord: ChunkCoord) -> usize {
        let center = self.bounds.center();
        ((coord.x as f32 >= center.x) as usize) + 
        (((coord.z as f32 >= center.z) as usize) * 2)
    }

    fn get_quadrant_bounds(&self, quadrant: usize) -> AABB {
        let center = self.bounds.center();
        match quadrant {
            0 => AABB { min: self.bounds.min, max: center },
            1 => AABB { min: Vec3::new(center.x, self.bounds.min.y, self.bounds.min.z), max: Vec3::new(self.bounds.max.x, center.y, center.z) },
            2 => AABB { min: Vec3::new(self.bounds.min.x, self.bounds.min.y, center.z), max: Vec3::new(center.x, center.y, self.bounds.max.z) },
            3 => AABB { min: center, max: self.bounds.max },
            _ => panic!("Invalid quadrant"),
        }
    }

    pub fn query(&self, frustum: &ViewFrustum) -> Vec<ChunkCoord> {
        let mut visible_chunks = Vec::new();
        
        if !self.bounds.intersects_frustum(frustum) {
            return visible_chunks;
        }

        visible_chunks.extend(&self.chunks);

        for node in &self.nodes {
            if let Some(child) = node {
                visible_chunks.extend(child.query(frustum));
            }
        }

        visible_chunks
    }
}

// Supporting AABB intersection implementation
impl AABB {
    fn intersects_frustum(&self, frustum: &ViewFrustum) -> bool {
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


// Supporting ViewFrustum plane extraction
impl ViewFrustum {
    fn from_matrices(view: &Mat4, proj: &Mat4) -> Self {
        let vp = proj * view;
        let mut planes = [Plane::default(); 6];
        
        // Left plane
        planes[0].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[0],
                                    vp.y_axis[3] + vp.y_axis[0],
                                    vp.z_axis[3] + vp.z_axis[0]);
        planes[0].distance = vp.w_axis[3] + vp.w_axis[0];
        
        // Right plane
        planes[1].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[0],
                                    vp.y_axis[3] - vp.y_axis[0],
                                    vp.z_axis[3] - vp.z_axis[0]);
        planes[1].distance = vp.w_axis[3] - vp.w_axis[0];
        
        // Bottom plane
        planes[2].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[1],
                                    vp.y_axis[3] + vp.y_axis[1],
                                    vp.z_axis[3] + vp.z_axis[1]);
        planes[2].distance = vp.w_axis[3] + vp.w_axis[1];
        
        // Top plane
        planes[3].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[1],
                                    vp.y_axis[3] - vp.y_axis[1],
                                    vp.z_axis[3] - vp.z_axis[1]);
        planes[3].distance = vp.w_axis[3] - vp.w_axis[1];
        
        // Near plane
        planes[4].normal = Vec3::new(vp.x_axis[3] + vp.x_axis[2],
                                    vp.y_axis[3] + vp.y_axis[2],
                                    vp.z_axis[3] + vp.z_axis[2]);
        planes[4].distance = vp.w_axis[3] + vp.w_axis[2];
        
        // Far plane
        planes[5].normal = Vec3::new(vp.x_axis[3] - vp.x_axis[2],
                                    vp.y_axis[3] - vp.y_axis[2],
                                    vp.z_axis[3] - vp.z_axis[2]);
        planes[5].distance = vp.w_axis[3] - vp.w_axis[2];
        
        // Normalize all planes
        for plane in &mut planes {
            let length = plane.normal.length();
            plane.normal /= length;
            plane.distance /= length;
        }
        
        Self { planes }
    }
}