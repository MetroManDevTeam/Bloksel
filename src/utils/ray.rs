use glam::Vec3;

#[derive(Debug, Clone)]
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

    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> Option<f32> {
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;

        for i in 0..3 {
            let t1 = (min[i] - self.origin[i]) / self.direction[i];
            let t2 = (max[i] - self.origin[i]) / self.direction[i];

            tmin = tmin.max(t1.min(t2));
            tmax = tmax.min(t1.max(t2));
        }

        if tmax >= tmin && tmax > 0.0 {
            Some(tmin.max(0.0))
        } else {
            None
        }
    }
} 