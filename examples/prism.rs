use macroquad::prelude::*;
use nalgebra::Vector2;
use prism::shape::Polygon;
use prism::Volume;

#[macroquad::main("Packed")]
async fn main() {
    let area = Polygon::new().add_polygon(&[
        Vector2::new(300.0, 200.0),
        Vector2::new(400.0, 400.0),
        Vector2::new(200.0, 400.0),
    ]);
    let mut points = area.clone().packed_points(5.0);
    loop {
        clear_background(BLACK);

        draw_rectangle_lines(100.0, 100.0, 200.0, 200.0, 3.0, RED);
        draw_triangle_lines(
            Vec2::new(150.0, 150.0),
            Vec2::new(150.0, 250.0),
            Vec2::new(250.0, 200.0),
            3.0,
            RED,
        );

        if is_mouse_button_pressed(MouseButton::Left) {
            points = area.clone().packed_points(5.0);
        }

        draw_text(
            &format!("Solver Iters: {}", points.iters),
            10.0,
            10.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("Penetration: {}", points.max_penetration),
            10.0,
            30.0,
            20.0,
            WHITE,
        );

        for v in points.iter() {
            draw_circle(v.x, v.y, 5.0, Color::from_rgba(255, 255, 255, 255));
        }
        next_frame().await;
    }
}
