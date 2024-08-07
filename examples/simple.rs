use amida::shape::Polygon;
use amida::*;
use macroquad::prelude::*;
use nalgebra::Vector2;

#[macroquad::main("Simple")]
async fn main() {
    let mut t = 0.0;
    loop {
        t += 0.01;
        t %= 10.0;
        clear_background(BLACK);

        draw_rectangle_lines(100.0, 100.0, 200.0, 200.0, 3.0, RED);
        draw_triangle_lines(
            Vec2::new(150.0, 150.0),
            Vec2::new(150.0, 250.0),
            Vec2::new(250.0, 200.0),
            3.0,
            RED,
        );
        let area = Polygon::new()
            .add_rect(Vector2::new(100.0, 100.0), Vector2::new(200.0, 200.0))
            .add_polygon(&[
                Vector2::new(150.0, 150.0),
                Vector2::new(150.0, 250.0),
                Vector2::new(250.0, 200.0),
            ]);
        for v in area.grid_points(5.0) {
            draw_circle(v.x, v.y, 5.0, Color::from_rgba(255, 255, 255, 255));
        }
        next_frame().await;
    }
}
