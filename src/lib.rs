use std::ops::{Index, IndexMut};

use nalgebra::{SVector as Vector, Vector2};
use rand::{Rng, SeedableRng};

pub mod shape;
pub mod utils;
use rand_pcg::Pcg64Mcg;
use utils::*;

pub trait Domain<const N: usize>: Volume<N> {
    fn distance(&self, point: Vector<f32, N>) -> f32;

    fn adjust(self, offset: f32) -> AdjustedDistanceField<Self, N> {
        AdjustedDistanceField {
            offset,
            field: self,
        }
    }
    fn create_grid(&self, cell_size: f32) -> Grid<N> {
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
        Grid {
            cell_size,
            offset,
            cells,
            inside_cells,
            border_cells,
        }
    }

    fn grid_points(self, settings: impl Into<GridSettings<N>>) -> Vec<Vector<f32, N>> {
        let settings = settings.into();
        if settings.border_adjust_radius != 0.0 {
            let this = self.adjust(settings.border_adjust_radius);
            let offset = settings.grid_offset.unwrap_or_else(|| this.min_bound());
            let cell_size = settings
                .cell_size
                .unwrap_or_else(|| settings.grid_size.fold(0.0, |x, y| x.max(y)));
            let sampler = Sampler::new(this, cell_size);
            let mut points = vec![];
            sampler.generate_grid(settings.grid_size, offset, |point| {
                points.push(point);
            });
            points
        } else {
            let offset = settings.grid_offset.unwrap_or_else(|| self.min_bound());
            let cell_size = settings
                .cell_size
                .unwrap_or_else(|| settings.grid_size.fold(0.0, |x, y| x.max(y)));
            let sampler = Sampler::new(self, cell_size);
            let mut points = vec![];
            sampler.generate_grid(settings.grid_size, offset, |point| {
                points.push(point);
            });
            points
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AdjustedDistanceField<D: Domain<N>, const N: usize> {
    offset: f32,
    field: D,
}
impl<D: Domain<N>, const N: usize> Domain<N> for AdjustedDistanceField<D, N> {
    fn distance(&self, point: Vector<f32, N>) -> f32 {
        self.field.distance(point) + self.offset
    }
}
impl<D: Domain<N>, const N: usize> Volume<N> for AdjustedDistanceField<D, N> {
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

pub trait Volume<const N: usize>: Sized {
    fn contains(&self, point: Vector<f32, N>) -> bool;
    fn min_bound(&self) -> Vector<f32, N>;
    fn max_bound(&self) -> Vector<f32, N>;
}

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

pub struct Sampler<D: Domain<N>, const N: usize, R: Rng> {
    pub grid: Grid<N>,
    pub domain: D,
    pub rng: R,
}
impl<D: Domain<N>, const N: usize> Sampler<D, N, Pcg64Mcg> {
    pub fn new(domain: D, cell_size: f32) -> Self {
        Self {
            grid: domain.create_grid(cell_size),
            domain,
            rng: Pcg64Mcg::from_entropy(),
        }
    }
}
impl<D: Domain<N>, const N: usize, R: Rng> Sampler<D, N, R> {
    pub fn with_rng(domain: D, cell_size: f32, rng: R) -> Self {
        Self {
            grid: domain.create_grid(cell_size),
            domain,
            rng,
        }
    }
    pub fn contains(&self, point: Vector<f32, N>) -> bool {
        match self.grid[point] {
            Cell::Inside => true,
            Cell::Outside => false,
            Cell::Border => self.domain.contains(point),
        }
    }
    pub fn sample_white(&mut self) -> Vector<f32, N> {
        let allowed_cells = self.grid.inside_cells.len() + self.grid.border_cells.len();
        loop {
            let cell = self.rng.gen_range(0..allowed_cells);
            let point = Vector::from_fn(|_, _| self.rng.gen_range(0.0..self.grid.cell_size));
            if cell < self.grid.inside_cells.len() {
                return point + self.grid.inside_cells[cell].cast::<f32>() * self.grid.cell_size;
            } else {
                let point = point
                    + self.grid.border_cells[cell - self.grid.inside_cells.len()].cast::<f32>()
                        * self.grid.cell_size;
                if self.domain.contains(point) {
                    return point;
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
        for cell in &self.grid.inside_cells {
            foreach_grid_in_rect(
                offset,
                size,
                cell.cast::<f32>() * self.grid.cell_size,
                Vector::repeat(self.grid.cell_size),
                &mut f,
            );
        }
        for cell in &self.grid.border_cells {
            foreach_grid_in_rect(
                offset,
                size,
                cell.cast::<f32>() * self.grid.cell_size,
                Vector::repeat(self.grid.cell_size),
                |point| {
                    if self.domain.contains(point) {
                        f(point);
                    }
                },
            );
        }
    }
}

pub struct Grid<const N: usize> {
    pub cell_size: f32,
    pub offset: Vector<i32, N>,
    pub cells: Array<Cell, N>,
    pub inside_cells: Vec<Vector<i32, N>>,
    pub border_cells: Vec<Vector<i32, N>>,
}
impl<const N: usize> Grid<N> {
    fn containing_cell(&self, point: Vector<f32, N>) -> Vector<i32, N> {
        (point / self.cell_size).map(|x| x.floor() as i32)
    }
}

impl<const N: usize> Index<Vector<f32, N>> for Grid<N> {
    type Output = Cell;
    fn index(&self, index: Vector<f32, N>) -> &Self::Output {
        &self.cells[(self.containing_cell(index) - self.offset)
            .try_cast::<u32>()
            .unwrap()]
    }
}
impl<const N: usize> Index<Vector<i32, N>> for Grid<N> {
    type Output = Cell;
    fn index(&self, index: Vector<i32, N>) -> &Self::Output {
        &self.cells[(index - self.offset).try_cast::<u32>().unwrap()]
    }
}
impl<const N: usize> IndexMut<Vector<i32, N>> for Grid<N> {
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

impl From<f32> for ParticleSettings {
    fn from(min_radius: f32) -> Self {
        Self {
            min_radius,
            exclude_border: false,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ParticleSettings {
    pub min_radius: f32,
    pub exclude_border: bool,
}
pub struct GridSettings<const N: usize> {
    pub border_adjust_radius: f32,
    pub grid_size: Vector<f32, N>,
    pub cell_size: Option<f32>,
    // If None, uses grid_size / 2 + min_bound of shape.
    pub grid_offset: Option<Vector<f32, N>>,
}

impl<X, const N: usize> From<X> for GridSettings<N>
where
    ParticleSettings: From<X>,
{
    fn from(x: X) -> Self {
        let settings = ParticleSettings::from(x);
        Self {
            border_adjust_radius: if settings.exclude_border {
                0.0
            } else {
                settings.min_radius - 0.00001
            },
            grid_size: Vector::repeat(settings.min_radius * 2.0),
            cell_size: None,
            grid_offset: None,
        }
    }
}
