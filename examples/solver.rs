use macroquad::prelude::*;
use nalgebra::Vector2;
use prism::base::Sampler;
use prism::shape::Polygon;
use prism::solver::Solver;
use prism::Volume;

#[macroquad::main("Solver")]
async fn main() {
    let area = Polygon::new()
        .add_rect(Vector2::new(100.0, 100.0), Vector2::new(200.0, 200.0))
        .add_polygon(&[
            Vector2::new(150.0, 150.0),
            Vector2::new(150.0, 250.0),
            Vector2::new(250.0, 200.0),
        ])
        .pad(5.0);
    let mut sampler = Sampler::new(area, 10.0);
    let mut points = vec![]; //  (0..400).map(|_| sampler.sample_white()).collect::<Vec<_>>();
    sampler.generate_randomized_grid(1.2, |p| {
        points.push(p);
    });

    let mut solver = Solver::new(sampler.volume, points, 5.0);

    loop {
        clear_background(BLACK);

        draw_text(
            &format!("Max penetration: {}", solver.max_penetration),
            10.0,
            10.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("Boundary penetration: {}", solver.boundary_penetration),
            10.0,
            30.0,
            20.0,
            WHITE,
        );

        draw_rectangle_lines(100.0, 100.0, 200.0, 200.0, 3.0, RED);
        draw_triangle_lines(
            Vec2::new(150.0, 150.0),
            Vec2::new(150.0, 250.0),
            Vec2::new(250.0, 200.0),
            3.0,
            RED,
        );

        if is_mouse_button_pressed(MouseButton::Left) {
            solver.step_collisions(1.0);
        }
        if is_mouse_button_pressed(MouseButton::Middle) {
            println!("{:?}", solver.solve(1000, 0.1));
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            solver.step_boundary(1.0);
        }

        for point in solver.points.iter() {
            draw_circle(point.x, point.y, 5.0, Color::from_rgba(255, 255, 255, 255));
        }

        next_frame().await;
    }
}
