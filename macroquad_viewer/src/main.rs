// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use ::rand::Rng;
use ::rand::thread_rng;
use i_overlay::i_float::float::compatible::FloatPointCompatible;
use macroquad::prelude::*;
use polygon_unionfind::PolygonUnionFind;
use std::f64::consts::PI;

/*fn generate_random_polygons(
    count: usize,
    min_vertices: usize,
    max_vertices: usize,
    min_coord: i64,
    max_coord: i64,
) -> Vec<Vec<[i64; 2]>> {
    let mut rng = thread_rng();
    let mut polygons = Vec::with_capacity(count);

    for _ in 0..count {
        let num_vertices = rng.gen_range(min_vertices..=max_vertices);
        let mut vertices = Vec::with_capacity(num_vertices);

        for _ in 0..num_vertices {
            vertices.push([
                rng.gen_range(min_coord..=max_coord),
                rng.gen_range(min_coord..=max_coord),
            ]);
        }

        polygons.push(vertices);
    }

    polygons
}*/

fn generate_random_radial_polygons(
    count: usize,
    min_vertices: usize,
    max_vertices: usize,
    min_coord: i64,
    max_coord: i64,
) -> Vec<Vec<[i64; 2]>> {
    let mut rng = thread_rng();
    let mut polygons = Vec::with_capacity(count);

    for _ in 0..count {
        let num_vertices = rng.gen_range(min_vertices..=max_vertices);

        // Pick a random center point for the polygon
        let center_x = rng.gen_range(min_coord..=max_coord) as f64;
        let center_y = rng.gen_range(min_coord..=max_coord) as f64;

        // Maximum radius from the center
        let max_radius = ((max_coord - min_coord) / 2) as f64;

        // Generate vertices around the center
        let mut angles: Vec<f64> = (0..num_vertices)
            .map(|_| rng.gen_range(0.0..2.0 * PI))
            .collect();

        // Sort angles to ensure a proper clockwise or counter-clockwise polygon
        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut vertices = Vec::with_capacity(num_vertices);
        for angle in angles {
            let radius = rng.gen_range(0.1 * max_radius..max_radius);
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            vertices.push([x.round() as i64, y.round() as i64]);
        }

        polygons.push(vertices);
    }

    polygons
}

#[macroquad::main("Polygon Union-Find Viewer")]
async fn main() {
    let mut polygon_unionfind: PolygonUnionFind<i64> = PolygonUnionFind::new();

    for polygon in generate_random_radial_polygons(6, 3, 8, -200, 200) {
        polygon_unionfind.insert(polygon);
    }

    let mut zoom = 1.0f32;
    let mut offset = vec2(0.0, 0.0);
    let mut last_mouse_pos: Option<Vec2> = None;

    loop {
        let (_, scroll_y) = mouse_wheel();
        if scroll_y != 0.0 {
            zoom *= 1.0 + scroll_y * 0.1;
            zoom = zoom.clamp(0.1, 20.0);
        }

        if is_mouse_button_down(MouseButton::Middle) {
            let (mx, my) = mouse_position();
            let current = vec2(mx, my);
            if let Some(previous) = last_mouse_pos {
                offset += current - previous;
            }
            last_mouse_pos = Some(current);
        } else {
            last_mouse_pos = None;
        }

        clear_background(BLACK);

        let center = vec2(screen_width() * 0.5, screen_height() * 0.5) + offset;

        for polygon in polygon_unionfind.polygons() {
            for window in polygon
                .iter()
                .zip(polygon.iter().cycle().skip(1))
                .take(polygon.len())
            {
                let (from, to) = window;
                let start = center + vec2(from.x() as f32, -from.y() as f32) * zoom;
                let end = center + vec2(to.x() as f32, -to.y() as f32) * zoom;
                draw_line(start.x, start.y, end.x, end.y, 3.0, WHITE);
            }
        }

        next_frame().await;
    }
}
