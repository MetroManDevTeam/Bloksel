use glam::{Vec3, Vec4, Mat4};

pub fn calculate_distance(pos: Vec3, coord: &ChunkCoord, chunk_size: u32) -> f32 {
    let chunk_center = coord.to_world_center(chunk_size);
    pos.distance(chunk_center)
}

pub fn calculate_lod_level(distance: f32, view_distance: f32) -> u32 {
    if distance < view_distance * 0.3 {
        0
    } else if distance < view_distance * 0.6 {
        1
    } else {
        2
    }
}
