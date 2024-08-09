use super::*;

pub trait VolumeCore<const N: usize>: Sized + Sync {
    fn distance(&self, point: Vector<f32, N>) -> f32;
    // The gradient of the sdf. Should be normalized.
    // Note: This may be 0 if there isn't a single direction to move in.
    fn gradient(&self, point: Vector<f32, N>) -> Vector<f32, N>;

    fn contains(&self, point: Vector<f32, N>) -> bool;
    fn min_bound(&self) -> Vector<f32, N>;
    fn max_bound(&self) -> Vector<f32, N>;

    fn create_grid(self, cell_size: f32) -> VolumeGrid<Self, N> {
        let offset = self.min_bound().map(|x| (x / cell_size).floor() as i32);
        let size = (self.max_bound().map(|x| (x / cell_size).ceil() as i32) - offset)
            .try_cast::<u32>()
            .unwrap();
        let mut inside_cells = vec![];
        let mut border_cells = vec![];
        let cells = Array::from_fn(size, |pos| {
            let pos = pos.cast::<i32>() + offset;
            let point = (pos.cast::<f32>() + Vector::repeat(0.5)) * cell_size;
            let dist = self.distance(point) * std::f32::consts::SQRT_2 / cell_size;
            let ty = match dist {
                ..-1.0 => Cell::Inside,
                -1.0..=1.0 => Cell::Border,
                _ => Cell::Outside,
            };
            if ty == Cell::Inside {
                inside_cells.push(pos);
            } else if ty == Cell::Border {
                border_cells.push(pos);
            }
            ty
        });
        VolumeGrid {
            volume: self,
            cell_size,
            offset,
            cells,
            inside_cells,
            border_cells,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Array<T, const N: usize> {
    shape: Vector<u32, N>,
    data: Box<[T]>,
}

impl<T, const N: usize> Array<T, N> {
    pub fn steps(&self) -> Vector<u32, N> {
        let mut steps = Vector::repeat(1);
        for (i, x) in self.shape.iter().enumerate() {
            if i + 1 != self.shape.len() {
                steps[i + 1] = x * steps[i];
            }
        }
        steps
    }

    pub fn from_fn(shape: Vector<u32, N>, mut f: impl FnMut(Vector<u32, N>) -> T) -> Self {
        let size = shape.cast::<usize>().product();
        let data = (0..size)
            .map(|index| f(from_linear(index, shape)))
            .collect();
        Self { shape, data }
    }
    pub fn repeat(shape: Vector<u32, N>, t: T) -> Self
    where
        T: Clone,
    {
        Self::from_fn(shape, |_| t.clone())
    }
    pub fn contains(&self, index: Vector<u32, N>) -> bool {
        index.zip_fold(&self.shape, true, |lt, i, s| lt && i < s)
    }
}
impl<T, const N: usize> Index<Vector<u32, N>> for Array<T, N> {
    type Output = T;
    fn index(&self, index: Vector<u32, N>) -> &T {
        assert!(self.contains(index));
        &self.data[to_linear(index, self.shape)]
    }
}
impl<T, const N: usize> IndexMut<Vector<u32, N>> for Array<T, N> {
    fn index_mut(&mut self, index: Vector<u32, N>) -> &mut T {
        assert!(self.contains(index));
        &mut self.data[to_linear(index, self.shape)]
    }
}

#[derive(Debug, Clone)]
pub struct Sampler<V: VolumeCore<N>, const N: usize, R: Rng> {
    pub volume: VolumeGrid<V, N>,
    pub rng: R,
}
impl<V: VolumeCore<N>, const N: usize> Sampler<V, N, Pcg64Mcg> {
    pub fn new(volume: V, cell_size: f32) -> Self {
        Self {
            volume: volume.create_grid(cell_size),
            rng: Pcg64Mcg::from_entropy(),
        }
    }
}
impl<V: VolumeCore<N>, const N: usize, R: Rng> Sampler<V, N, R> {
    pub fn with_rng(volume: V, cell_size: f32, rng: R) -> Self {
        Self {
            volume: volume.create_grid(cell_size),
            rng,
        }
    }
    pub fn contains(&self, point: Vector<f32, N>) -> bool {
        match self.volume[point] {
            Cell::Inside => true,
            Cell::Outside => false,
            Cell::Border => self.volume.contains(point),
        }
    }
    pub fn sample_white(&mut self) -> Vector<f32, N> {
        let allowed_cells = self.volume.inside_cells.len() + self.volume.border_cells.len();
        loop {
            let cell = self.rng.gen_range(0..allowed_cells);
            let point = Vector::from_fn(|_, _| self.rng.gen_range(0.0..self.volume.cell_size));
            if cell < self.volume.inside_cells.len() {
                return point
                    + self.volume.inside_cells[cell].cast::<f32>() * self.volume.cell_size;
            } else {
                let point = point
                    + self.volume.border_cells[cell - self.volume.inside_cells.len()].cast::<f32>()
                        * self.volume.cell_size;
                if self.volume.contains(point) {
                    return point;
                }
            }
        }
    }
    pub fn generate_randomized_grid(
        &mut self,
        samples_per_cell: f32,
        mut f: impl FnMut(Vector<f32, N>),
    ) {
        for cell in &self.volume.inside_cells {
            let num_samples = samples_per_cell.floor() as u32
                + self.rng.gen_bool(samples_per_cell.fract() as f64) as u32;
            for _ in 0..num_samples {
                f(
                    Vector::from_fn(|_, _| self.rng.gen_range(0.0..self.volume.cell_size))
                        + cell.cast::<f32>() * self.volume.cell_size,
                );
            }
        }
        for cell in &self.volume.border_cells {
            let num_samples = samples_per_cell.floor() as u32
                + self.rng.gen_bool(samples_per_cell.fract() as f64) as u32;
            for _ in 0..num_samples {
                let point = Vector::from_fn(|_, _| self.rng.gen_range(0.0..self.volume.cell_size))
                    + cell.cast::<f32>() * self.volume.cell_size;
                if self.volume.contains(point) {
                    f(point);
                }
            }
        }
    }
    // TODO: Move out since it doesn't require a RNG?
    pub fn generate_grid(
        &self,
        size: Vector<f32, N>,
        offset: Vector<f32, N>,
        mut f: impl FnMut(Vector<f32, N>),
    ) {
        for cell in &self.volume.inside_cells {
            foreach_grid_in_rect(
                offset,
                size,
                cell.cast::<f32>() * self.volume.cell_size,
                Vector::repeat(self.volume.cell_size),
                &mut f,
            );
        }
        for cell in &self.volume.border_cells {
            foreach_grid_in_rect(
                offset,
                size,
                cell.cast::<f32>() * self.volume.cell_size,
                Vector::repeat(self.volume.cell_size),
                |point| {
                    if self.volume.contains(point) {
                        f(point);
                    }
                },
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeGrid<V: VolumeCore<N>, const N: usize> {
    pub volume: V,
    pub cell_size: f32,
    pub offset: Vector<i32, N>,
    pub cells: Array<Cell, N>,
    pub inside_cells: Vec<Vector<i32, N>>,
    pub border_cells: Vec<Vector<i32, N>>,
}
impl<V: VolumeCore<N>, const N: usize> Deref for VolumeGrid<V, N> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.volume
    }
}
impl<V: VolumeCore<N>, const N: usize> VolumeGrid<V, N> {
    pub fn containing_cell(&self, point: Vector<f32, N>) -> Vector<i32, N> {
        (point / self.cell_size).map(|x| x.floor() as i32)
    }
}

impl<V: VolumeCore<N>, const N: usize> Index<Vector<f32, N>> for VolumeGrid<V, N> {
    type Output = Cell;
    fn index(&self, index: Vector<f32, N>) -> &Self::Output {
        &self.cells[(self.containing_cell(index) - self.offset)
            .try_cast::<u32>()
            .unwrap()]
    }
}
impl<V: VolumeCore<N>, const N: usize> Index<Vector<i32, N>> for VolumeGrid<V, N> {
    type Output = Cell;
    fn index(&self, index: Vector<i32, N>) -> &Self::Output {
        &self.cells[(index - self.offset).try_cast::<u32>().unwrap()]
    }
}
impl<V: VolumeCore<N>, const N: usize> IndexMut<Vector<i32, N>> for VolumeGrid<V, N> {
    fn index_mut(&mut self, index: Vector<i32, N>) -> &mut Self::Output {
        &mut self.cells[(index - self.offset).try_cast::<u32>().unwrap()]
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cell {
    Inside,
    Outside,
    Border,
}
