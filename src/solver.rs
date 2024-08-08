use super::*;

#[derive(Debug, Clone)]
pub struct Solver<V: VolumeCore<N>, const N: usize> {
    pub volume: VolumeGrid<V, N>,
    pub point_grid: HashMap<Vector<i32, N>, SmallVec<[u32; 4]>>,
    pub points: Vec<Vector<f32, N>>,
    pub radius: f32,
    pub max_penetration: f32,
    pub boundary_penetration: f32,
}

impl<V: VolumeCore<N>, const N: usize> Solver<V, N> {
    pub fn new(volume: VolumeGrid<V, N>, points: Vec<Vector<f32, N>>, radius: f32) -> Self {
        Self {
            volume,
            point_grid: HashMap::new(),
            points,
            radius,
            max_penetration: f32::INFINITY,
            boundary_penetration: f32::INFINITY,
        }
    }
    pub fn update_grid(&mut self) {
        self.point_grid.clear();
        for (i, point) in self.points.iter().enumerate() {
            let cell = self.volume.containing_cell(*point);
            self.point_grid.entry(cell).or_default().push(i as u32);
        }
    }
    pub fn neighbors(&self, point_index: usize, mut f: impl FnMut(u32, Vector<f32, N>)) {
        let point = self.points[point_index];
        let cell = self.volume.containing_cell(point);
        for i in 0..3_usize.pow(N as u32) {
            let offset =
                from_linear(i, Vector::<_, N>::repeat(3)).cast::<i32>() - Vector::repeat(1);
            if let Some(adj) = self.point_grid.get(&(cell + offset)) {
                for &adj in adj {
                    if adj != point_index as u32 {
                        f(adj, self.points[adj as usize]);
                    }
                }
            }
        }
    }
    pub fn step_collisions(&mut self, delta_factor: f32) {
        self.update_grid();
        let mut max_penetration: f32 = 0.0;
        let deltas = self
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let mut delta = Vector::repeat(0.0);
                self.neighbors(i, |_, q| {
                    let dist = (p - q).norm();
                    let penetration = self.radius * 2.0 - dist;
                    if penetration > 0.0 {
                        let normal = (p - q) / dist;
                        delta += normal * penetration / 2.0;
                    }
                    max_penetration = max_penetration.max(penetration);
                });
                delta
            })
            .collect::<Vec<_>>();
        for (point, delta) in self.points.iter_mut().zip(deltas) {
            *point += delta * delta_factor;
        }
        self.max_penetration = max_penetration;
    }
    pub fn step_boundary(&mut self, delta_factor: f32) {
        self.boundary_penetration = 0.0;
        for point in &mut self.points {
            let dist = self.volume.distance(*point);
            if dist > 0.0 {
                *point -= self.volume.gradient(*point) * dist * delta_factor;
                self.boundary_penetration =
                    self.boundary_penetration.max(self.volume.distance(*point));
            }
        }
    }
    pub fn solve(&mut self, max_iters: usize, cutoff: f32) -> usize {
        let mut iters = 0;
        while (self.max_penetration > cutoff * self.radius
            || self.boundary_penetration > 0.0001 * self.radius)
            && iters < max_iters
        {
            self.step_collisions(2.0);
            self.step_boundary(1.0);
            iters += 1;
        }
        iters
    }
}
