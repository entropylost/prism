use super::*;

pub fn distance_to_line<const N: usize>(
    a: Vector<f32, N>,
    b: Vector<f32, N>,
    x: Vector<f32, N>,
) -> f32 {
    (x - project_line(a, b, x)).norm()
}

pub fn project_line<const N: usize>(
    a: Vector<f32, N>,
    b: Vector<f32, N>,
    x: Vector<f32, N>,
) -> Vector<f32, N> {
    let c = b - a;
    let l2 = c.norm_squared();
    let proj = (x - a).dot(&c) / l2;
    let proj = proj.clamp(0.0, 1.0);
    a + proj * c
}

// Also, https://github.com/RenderKit/embree/blob/master/tutorials/common/math/closest_point.h

pub fn foreach_grid_in_rect<const N: usize>(
    offset: Vector<f32, N>,
    size: Vector<f32, N>,
    rect_offset: Vector<f32, N>,
    rect_size: Vector<f32, N>,
    mut f: impl FnMut(Vector<f32, N>),
) {
    let offset = (offset - rect_offset).zip_map(&size, |x, s| x.rem_euclid(s));
    let shape = (rect_size - offset)
        .component_div(&size)
        .map(|x| x.ceil() as u32);
    let total_size = shape.cast::<usize>().product();
    for i in 0..total_size {
        let point = rect_offset + offset + from_linear(i, shape).cast::<f32>().component_mul(&size);
        f(point);
    }
}

pub fn from_linear<const N: usize>(mut index: usize, shape: Vector<u32, N>) -> Vector<u32, N> {
    Vector::from_fn(|i, _| {
        let si = shape[i] as usize;
        let res = index % si;
        index /= si;
        res as u32
    })
}
pub fn to_linear<const N: usize>(index: Vector<u32, N>, shape: Vector<u32, N>) -> usize {
    index
        .zip_fold(&shape, (1, 0), |(step, res), ix, s| {
            (step * s as usize, res + step * ix as usize)
        })
        .1
}
