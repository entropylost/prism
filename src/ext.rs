use super::*;

pub trait Volume<const N: usize>: VolumeCore<N> {
    fn pad(self, offset: f32) -> PaddedVolume<Self, N> {
        PaddedVolume {
            offset,
            field: self,
        }
    }
    fn grid_points(self, settings: impl Into<GridSettings<N>>) -> Vec<Vector<f32, N>> {
        let settings = settings.into();
        if settings.border_adjust_radius != 0.0 {
            let domain = self.pad(settings.border_adjust_radius);
            grid_points_impl(domain, settings)
        } else {
            grid_points_impl(self, settings)
        }
    }

    fn random_points(self, count: usize) -> Vec<Vector<f32, N>> {
        let cell_size =
            (self.max_bound() - self.min_bound()).fold(f32::INFINITY, |x, y| x.min(y)) / 9.99;
        self.random_points_with_rng(count, cell_size, Pcg64Mcg::from_entropy())
    }

    fn random_points_with_rng(
        self,
        count: usize,
        cell_size: f32,
        rng: impl RngCore,
    ) -> Vec<Vector<f32, N>> {
        let mut sampler = Sampler::with_rng(self, cell_size, rng);
        (0..count).map(|_| sampler.sample_white()).collect()
    }

    fn packed_points(self, settings: impl Into<PackedSettings>) -> PackedPoints<N> {
        self.packed_points_with_rng(settings, Pcg64Mcg::from_entropy())
    }
    fn packed_points_with_rng(
        self,
        settings: impl Into<PackedSettings>,
        rng: impl RngCore,
    ) -> PackedPoints<N> {
        let settings = settings.into();
        if settings.particle_settings.pad_border {
            let domain = self.pad(settings.particle_settings.radius);
            packed_points_impl(domain, settings, rng)
        } else {
            packed_points_impl(self, settings, rng)
        }
    }
}
impl<const N: usize, X> Volume<N> for X where X: VolumeCore<N> {}

#[derive(Debug, Clone, Copy)]
pub struct ParticleSettings {
    pub radius: f32,
    pub pad_border: bool,
}
impl Default for ParticleSettings {
    fn default() -> Self {
        Self {
            radius: 1.0,
            pad_border: true,
        }
    }
}
impl From<f32> for ParticleSettings {
    fn from(radius: f32) -> Self {
        Self {
            radius,
            ..Default::default()
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct GridSettings<const N: usize> {
    pub border_adjust_radius: f32,
    pub grid_size: Vector<f32, N>,
    pub cell_size: Option<f32>,
    // If None, uses min_bound of shape.
    pub grid_offset: Option<Vector<f32, N>>,
}

impl<X, const N: usize> From<X> for GridSettings<N>
where
    ParticleSettings: From<X>,
{
    fn from(x: X) -> Self {
        let settings = ParticleSettings::from(x);
        Self {
            border_adjust_radius: if settings.pad_border {
                settings.radius * 0.9999
            } else {
                0.0
            },
            grid_size: Vector::repeat(settings.radius * 2.0),
            cell_size: None,
            grid_offset: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PackedSettings {
    pub particle_settings: ParticleSettings,
    pub max_iters: usize,
    pub cutoff: f32,
    pub density: f32,
}
impl Default for PackedSettings {
    fn default() -> Self {
        Self {
            particle_settings: Default::default(),
            max_iters: 500,
            cutoff: 0.1,
            density: 0.0,
        }
    }
}
impl<X> From<X> for PackedSettings
where
    ParticleSettings: From<X>,
{
    fn from(x: X) -> Self {
        let settings = x.into();
        Self {
            particle_settings: settings,
            ..Default::default()
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PaddedVolume<V: VolumeCore<N>, const N: usize> {
    offset: f32,
    field: V,
}
impl<V: VolumeCore<N>, const N: usize> VolumeCore<N> for PaddedVolume<V, N> {
    fn distance(&self, point: Vector<f32, N>) -> f32 {
        self.field.distance(point) + self.offset
    }
    fn gradient(&self, point: Vector<f32, N>) -> Vector<f32, N> {
        self.field.gradient(point)
    }
    fn contains(&self, point: Vector<f32, N>) -> bool {
        self.distance(point) <= 0.0
    }
    fn min_bound(&self) -> Vector<f32, N> {
        self.field.min_bound() + Vector::repeat(self.offset)
    }
    fn max_bound(&self) -> Vector<f32, N> {
        self.field.max_bound() - Vector::repeat(self.offset)
    }
}

pub struct PackedPoints<const N: usize> {
    pub points: Vec<Vector<f32, N>>,
    pub iters: usize,
    pub max_penetration: f32,
}
impl<const N: usize> Deref for PackedPoints<N> {
    type Target = Vec<Vector<f32, N>>;
    fn deref(&self) -> &Self::Target {
        &self.points
    }
}

fn grid_points_impl<const N: usize>(
    domain: impl VolumeCore<N>,
    settings: GridSettings<N>,
) -> Vec<Vector<f32, N>> {
    let offset = settings
        .grid_offset
        .unwrap_or_else(|| domain.min_bound().map(|x| x + 0.0001));
    let cell_size = settings
        .cell_size
        .unwrap_or_else(|| settings.grid_size.fold(0.0, |x, y| x.max(y)));
    let sampler = Sampler::new(domain, cell_size);
    let mut points = vec![];
    sampler.generate_grid(settings.grid_size, offset, |point| {
        points.push(point);
    });
    points
}

pub fn default_packed_density<const N: usize>() -> f32 {
    match N {
        1 => 1.0,
        2 => 1.0, // Max 1.2
        3 => 1.5,
        _ => 1.0,
    }
}

fn packed_points_impl<const N: usize>(
    domain: impl VolumeCore<N>,
    settings: PackedSettings,
    rng: impl RngCore,
) -> PackedPoints<N> {
    let mut sampler = Sampler::with_rng(domain, settings.particle_settings.radius * 2.0, rng);
    let mut points = vec![];
    sampler.generate_randomized_grid(
        if settings.density <= 0.0 {
            default_packed_density::<N>()
        } else {
            settings.density
        },
        |p| {
            points.push(p);
        },
    );
    let mut solver = Solver::new(sampler.volume, points, settings.particle_settings.radius);
    let iters = solver.solve(settings.max_iters, settings.cutoff);
    PackedPoints {
        points: solver.points,
        iters,
        max_penetration: solver.max_penetration,
    }
}
