use std::ops::{Add, Sub};

use nalgebra::Vector3;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct Cuboid<const N: usize> {
    min: Vector<f32, N>,
    max: Vector<f32, N>,
}
impl<const N: usize> Cuboid<N> {
    pub fn from_bounds(min: Vector<f32, N>, max: Vector<f32, N>) -> Self {
        Cuboid { min, max }
    }
    pub fn new(half_size: Vector<f32, N>) -> Self {
        Cuboid {
            min: -half_size,
            max: half_size,
        }
    }
    pub fn offset(self, offset: Vector<f32, N>) -> Self {
        Cuboid {
            min: self.min + offset,
            max: self.max + offset,
        }
    }
}
impl<const N: usize> Add<Vector<f32, N>> for Cuboid<N> {
    type Output = Self;
    fn add(self, rhs: Vector<f32, N>) -> Self::Output {
        self.offset(rhs)
    }
}
impl<const N: usize> Sub<Vector<f32, N>> for Cuboid<N> {
    type Output = Self;
    fn sub(self, rhs: Vector<f32, N>) -> Self::Output {
        self.offset(-rhs)
    }
}
impl<const N: usize> VolumeCore<N> for Cuboid<N> {
    fn nearest_surface_point(&self, point: Vector<f32, N>) -> (Vector<f32, N>, bool) {
        let inside = self.contains(point);
        let inner_nearest = point.zip_zip_map(&self.min, &self.max, |x, a, b| x.clamp(a, b));
        if !inside {
            return (inner_nearest, inside);
        }
        let mut dist = f32::MAX;
        let mut nearest = Vector::repeat(0.0);
        for i in 0..N {
            let mut side_nearest = inner_nearest;
            side_nearest[i] = self.min[i];
            if (point - side_nearest).norm() < dist {
                dist = (point - side_nearest).norm();
                nearest = side_nearest;
            }
            side_nearest[i] = self.max[i];
            if (point - side_nearest).norm() < dist {
                dist = (point - side_nearest).norm();
                nearest = side_nearest;
            }
        }

        (nearest, inside)
    }
    fn contains(&self, point: Vector<f32, N>) -> bool {
        point.zip_fold(&self.min, true, |acc, a, b| acc && (a >= b))
            && point.zip_fold(&self.max, true, |acc, a, b| acc && (a <= b))
    }
    fn min_bound(&self) -> Vector<f32, N> {
        self.min
    }
    fn max_bound(&self) -> Vector<f32, N> {
        self.max
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ball<const N: usize> {
    center: Vector<f32, N>,
    radius: f32,
}
impl<const N: usize> Ball<N> {
    pub fn new(radius: f32) -> Self {
        Ball {
            center: Vector::zeros(),
            radius,
        }
    }
    pub fn from_center(center: Vector<f32, N>, radius: f32) -> Self {
        Ball { center, radius }
    }
    pub fn offset(self, offset: Vector<f32, N>) -> Self {
        Ball {
            center: self.center + offset,
            radius: self.radius,
        }
    }
}
impl<const N: usize> Add<Vector<f32, N>> for Ball<N> {
    type Output = Self;
    fn add(self, rhs: Vector<f32, N>) -> Self::Output {
        self.offset(rhs)
    }
}
impl<const N: usize> Sub<Vector<f32, N>> for Ball<N> {
    type Output = Self;
    fn sub(self, rhs: Vector<f32, N>) -> Self::Output {
        self.offset(-rhs)
    }
}
impl<const N: usize> VolumeCore<N> for Ball<N> {
    fn nearest_surface_point(&self, point: Vector<f32, N>) -> (Vector<f32, N>, bool) {
        let delta = point - self.center;
        let norm = delta.norm();
        let dir = if norm <= 1e-6 {
            let mut v = Vector::repeat(0.0);
            v[0] = 1.0;
            v
        } else {
            delta / norm
        };
        (self.center + dir * self.radius, norm <= self.radius)
    }
    fn contains(&self, point: Vector<f32, N>) -> bool {
        (point - self.center).norm_squared() <= self.radius * self.radius
    }
    fn min_bound(&self) -> Vector<f32, N> {
        self.center - Vector::repeat(self.radius)
    }
    fn max_bound(&self) -> Vector<f32, N> {
        self.center + Vector::repeat(self.radius)
    }
}

// TODO: Make this work on 3D? Or only use triangles.
#[derive(Debug, Clone)]
pub struct Polygon<const N: usize> {
    polygons: Vec<Vec<Vector<f32, N>>>,
    min: Vector<f32, N>,
    max: Vector<f32, N>,
}
impl<const N: usize> Default for Polygon<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize> Polygon<N> {
    pub fn new() -> Self {
        Polygon {
            polygons: Vec::new(),
            min: Vector::repeat(f32::INFINITY),
            max: Vector::repeat(f32::NEG_INFINITY),
        }
    }
    pub fn add_polygon(self, polygon: &[Vector<f32, N>]) -> Self {
        let min = self.min.inf(
            &polygon
                .iter()
                .fold(Vector::repeat(f32::INFINITY), |x, y| y.inf(&x)),
        );
        let max = self.max.sup(
            &polygon
                .iter()
                .fold(Vector::repeat(f32::NEG_INFINITY), |x, y| y.sup(&x)),
        );
        let mut polygons = self.polygons;
        polygons.push(polygon.to_vec());
        Polygon { polygons, min, max }
    }
}
impl Polygon<2> {
    pub fn add_rect(self, half_size: Vector2<f32>, center: Vector2<f32>) -> Self {
        self.add_polygon(&[
            center - half_size,
            center + Vector2::new(half_size.x, -half_size.y),
            center + half_size,
            center + Vector2::new(-half_size.x, half_size.y),
        ])
    }
}
impl VolumeCore<2> for Polygon<2> {
    fn nearest_surface_point(&self, point: Vector<f32, 2>) -> (Vector<f32, 2>, bool) {
        let mut dist = f32::MAX;
        let mut nearest_point = Vector2::repeat(0.0);

        for polygon in &self.polygons {
            let mut b = polygon.last().unwrap();
            for a in polygon {
                let proj = project_line(*a, *b, point);
                if (proj - point).norm() <= dist {
                    dist = (proj - point).norm();
                    nearest_point = proj;
                }
                b = a;
            }
        }
        (nearest_point, self.contains(point))
    }
    fn contains(&self, point: Vector<f32, 2>) -> bool {
        if point.zip_fold(&self.min, false, |acc, a, b| acc | (a < b))
            || point.zip_fold(&self.max, false, |acc, a, b| acc | (a > b))
        {
            return false;
        }
        // https://web.archive.org/web/20200313050359/https://wrf.ecse.rpi.edu/Research/Short_Notes/pnpoly.html
        // Also: https://wrfranklin.org/Research/Short_Notes/pnpoly.html
        /*
        Copyright (c) 1970-2003, Wm. Randolph Franklin

        Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

            Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimers.
            Redistributions in binary form must reproduce the above copyright notice in the documentation and/or other materials provided with the distribution.
            The name of W. Randolph Franklin may not be used to endorse or promote products derived from this Software without specific prior written permission.

        THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
        */
        let mut interior = false;
        for polygon in &self.polygons {
            let mut b = polygon.last().unwrap();
            for a in polygon {
                if (a.y > point.y) != (b.y > point.y)
                    && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
                {
                    interior = !interior;
                }
                b = a;
            }
        }
        interior
    }
    fn min_bound(&self) -> Vector<f32, 2> {
        self.min
    }
    fn max_bound(&self) -> Vector<f32, 2> {
        self.max
    }
}

// This isn't generic due to lack of generic_const_exprs.
#[derive(Debug, Clone, Copy)]
pub struct Extrude3<V: VolumeCore<2>> {
    pub base: V,
    // TODO: Use a Range when new_range_api is stable.
    pub interval: (f32, f32),
}

impl<V: VolumeCore<2>> VolumeCore<3> for Extrude3<V> {
    fn nearest_surface_point(&self, point: Vector<f32, 3>) -> (Vector<f32, 3>, bool) {
        let (base_nearest, base_inside) = self
            .base
            .nearest_surface_point(point.fixed_view::<2, 1>(0, 0).into_owned());
        let interval_inside = point.z >= self.interval.0 && point.z <= self.interval.1;
        let closest_interval =
            if (point.z - self.interval.0).abs() < (point.z - self.interval.1).abs() {
                self.interval.0
            } else {
                self.interval.1
            };
        let inside = base_inside && interval_inside;
        let surface = if inside {
            let base_candidate = base_nearest.push(point.z);
            let interval_candidate = Vector3::new(point.x, point.y, closest_interval);
            if (base_candidate - point).norm() < (interval_candidate - point).norm() {
                base_candidate
            } else {
                interval_candidate
            }
        } else if base_inside {
            Vector3::new(point.x, point.y, closest_interval)
        } else if interval_inside {
            base_nearest.push(point.z)
        } else {
            base_nearest.push(closest_interval)
        };
        (surface, inside)
    }
    fn contains(&self, point: Vector<f32, 3>) -> bool {
        point.z >= self.interval.0
            && point.z <= self.interval.1
            && self
                .base
                .contains(point.fixed_view::<2, 1>(0, 0).into_owned())
    }
    fn min_bound(&self) -> Vector<f32, 3> {
        self.base.min_bound().push(self.interval.0)
    }
    fn max_bound(&self) -> Vector<f32, 3> {
        self.base.max_bound().push(self.interval.1)
    }
}
