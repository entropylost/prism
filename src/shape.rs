use super::*;

#[derive(Debug, Clone)]
pub struct Cuboid<const N: usize> {
    min: Vector<f32, N>,
    max: Vector<f32, N>,
}
impl<const N: usize> Cuboid<N> {
    pub fn new(min: Vector<f32, N>, max: Vector<f32, N>) -> Self {
        Cuboid { min, max }
    }
}
impl<const N: usize> Volume<N> for Cuboid<N> {
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
impl<const N: usize> Domain<N> for Cuboid<N> {
    fn distance(&self, point: Vector<f32, N>) -> f32 {
        let inside = self.contains(point);
        let nearest = point.zip_zip_map(&self.min, &self.max, |x, a, b| x.clamp(a, b));
        if !inside {
            return (point - nearest).norm();
        }
        let mut dist = f32::MAX;
        for i in 0..N {
            let mut nearest = nearest;
            nearest[i] = self.min[i];
            dist = dist.min((point - nearest).norm());
            nearest[i] = self.max[i];
            dist = dist.min((point - nearest).norm());
        }
        -dist
    }
}

#[derive(Debug, Clone)]
pub struct Ball<const N: usize> {
    center: Vector<f32, N>,
    radius: f32,
}
impl<const N: usize> Ball<N> {
    pub fn new(center: Vector<f32, N>, radius: f32) -> Self {
        Ball { center, radius }
    }
}
impl<const N: usize> Volume<N> for Ball<N> {
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
impl<const N: usize> Domain<N> for Ball<N> {
    fn distance(&self, point: Vector<f32, N>) -> f32 {
        let dist = (point - self.center).norm();
        dist - self.radius
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
impl Volume<2> for Polygon<2> {
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
impl Domain<2> for Polygon<2> {
    fn distance(&self, point: Vector2<f32>) -> f32 {
        let inside = self.contains(point);
        let mut dist = f32::MAX;
        for polygon in &self.polygons {
            let mut b = polygon.last().unwrap();
            for a in polygon {
                dist = dist.min(distance_to_line(*a, *b, point));
                b = a;
            }
        }

        if inside {
            -dist
        } else {
            dist
        }
    }
}
