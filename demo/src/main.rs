// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Programming of this file was assisted by OpenAI Codex 5.1/5.2/5.3 and Cursor
// Composer 2.0 Fast.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use polygon_unionfind::{Polygon, PolygonUnionFindDelta, RecordingPolygonUnionFind};
use undoredo::UndoRedo;

/// Monotone-chain convex hull; returns vertices in counter-clockwise order.
fn convex_hull(points: &[[i32; 2]]) -> Vec<[i32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut pts: Vec<[i32; 2]> = points.to_vec();
    pts.sort_by(|a, b| a[0].cmp(&b[0]).then_with(|| a[1].cmp(&b[1])));

    fn cross(o: [i32; 2], a: [i32; 2], b: [i32; 2]) -> i64 {
        (a[0] as i64 - o[0] as i64) * (b[1] as i64 - o[1] as i64)
            - (a[1] as i64 - o[1] as i64) * (b[0] as i64 - o[0] as i64)
    }

    let mut lower = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }

    let mut upper = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn random_convex_polygon_at_point(center: [i32; 2], radius: i32, count: usize) -> Vec<[i32; 2]> {
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let angle = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(radius as f32 * 0.5, radius as f32);
        let x = center[0] + (r * angle.cos()) as i32;
        let y = center[1] + (r * angle.sin()) as i32;
        points.push([x, y]);
    }

    let mut hull = convex_hull(&points);
    if hull.len() < 3 {
        hull = vec![
            [center[0] - radius, center[1] - radius],
            [center[0] + radius, center[1] - radius],
            [center[0], center[1] + radius],
        ];
    }
    hull
}

fn polygon_from_ring_i32(ring: Vec<[i32; 2]>) -> Polygon<i64, ()> {
    Polygon {
        exterior: ring
            .into_iter()
            .map(|[x, y]| [i64::from(x), i64::from(y)])
            .collect(),
        interiors: vec![],
        weight: (),
    }
}

#[macroquad::main("Polygon Union-Find Viewer")]
async fn main() {
    let mut undoredo: UndoRedo<PolygonUnionFindDelta<i64>> = UndoRedo::new();
    let mut polygon_unionfind: RecordingPolygonUnionFind<i64> = RecordingPolygonUnionFind::new();

    let mut zoom = 1.0f32;
    let mut offset = vec2(0.0, 0.0);
    let mut last_mouse_pos: Option<Vec2> = None;
    let mut show_insert_hint = true;

    loop {
        let undo_button = Rect::new(20.0, 20.0, 100.0, 36.0);
        let redo_button = Rect::new(130.0, 20.0, 100.0, 36.0);
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        if show_insert_hint && left_pressed {
            show_insert_hint = false;
        }
        let undo_clicked = left_pressed && undo_button.contains(mouse);
        let redo_clicked = left_pressed && redo_button.contains(mouse);

        let center = vec2(screen_width() * 0.5, screen_height() * 0.5) + offset;

        if undo_clicked {
            undoredo.undo(&mut polygon_unionfind);
        } else if redo_clicked {
            undoredo.redo(&mut polygon_unionfind);
        } else if left_pressed {
            if !undo_button.contains(mouse) && !redo_button.contains(mouse) {
                let click_world = vec2((mx - center.x) / zoom, -(my - center.y) / zoom);
                let radius = (60.0 / zoom).max(10.0).round() as i32;
                let count = gen_range(3, 10) as usize;
                let ring = random_convex_polygon_at_point(
                    [click_world.x.round() as i32, click_world.y.round() as i32],
                    radius,
                    count,
                );

                polygon_unionfind.insert(polygon_from_ring_i32(ring));
                undoredo.commit(&mut polygon_unionfind);
            }
        }

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

        let undo_hover = undo_button.contains(mouse);
        let redo_hover = redo_button.contains(mouse);
        let undo_pressed = undo_hover && is_mouse_button_down(MouseButton::Left);
        let redo_pressed = redo_hover && is_mouse_button_down(MouseButton::Left);

        let undo_fill = if undo_pressed {
            LIGHTGRAY
        } else if undo_hover {
            GRAY
        } else {
            DARKGRAY
        };
        let redo_fill = if redo_pressed {
            LIGHTGRAY
        } else if redo_hover {
            GRAY
        } else {
            DARKGRAY
        };

        draw_rectangle(
            undo_button.x,
            undo_button.y,
            undo_button.w,
            undo_button.h,
            undo_fill,
        );
        draw_text(
            "undo",
            undo_button.x + 26.0,
            undo_button.y + 24.0,
            28.0,
            WHITE,
        );
        draw_rectangle(
            redo_button.x,
            redo_button.y,
            redo_button.w,
            redo_button.h,
            redo_fill,
        );
        draw_text(
            "redo",
            redo_button.x + 26.0,
            redo_button.y + 24.0,
            28.0,
            WHITE,
        );

        if show_insert_hint {
            let hint = "Click to insert a new polygon.";
            let hint_size = 22.0;
            let hint_dims = measure_text(hint, None, hint_size as u16, 1.0);
            draw_text(
                hint,
                (screen_width() - hint_dims.width) * 0.5,
                screen_height() - 32.0,
                hint_size,
                GRAY,
            );
        }

        for geom_with_data in polygon_unionfind.rtree().as_ref().iter() {
            let [bbox_min_x, bbox_min_y] = geom_with_data.geom().lower();
            let [bbox_max_x, bbox_max_y] = geom_with_data.geom().upper();

            let bbox_origin = center + vec2(bbox_min_x as f32, -bbox_max_y as f32) * zoom;
            let bbox_width = (bbox_max_x as f32 - bbox_min_x as f32) * zoom;
            let bbox_height = (bbox_max_y as f32 - bbox_min_y as f32) * zoom;
            draw_rectangle_lines(
                bbox_origin.x,
                bbox_origin.y,
                bbox_width,
                bbox_height,
                2.0,
                DARKGRAY,
            );
        }

        for (i, polygon) in polygon_unionfind.polygons().into_iter().enumerate() {
            let colors = [RED, GREEN, BLUE, SKYBLUE, MAGENTA, YELLOW];
            let color = colors[i % colors.len()];
            let rings: Vec<&[[i64; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring.iter().zip(ring.iter().cycle().skip(1)).take(ring.len()) {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 3.0, color);
                }
            }
        }

        next_frame().await;
    }
}
