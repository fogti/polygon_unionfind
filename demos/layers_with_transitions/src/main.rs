// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use polygon_unionfind::{
    Inflated, LayersWithTransitionsDelta, Negated, Paralleled, PolygonSetHalfDelta,
    PolygonUnionFindHalfDelta, PolygonWithData, RecordingInflated, RecordingLayersWithTransitions,
    RecordingNegated, RecordingPolygonSet,
};
use undoredo::UndoRedo;

type DemoPolygon = PolygonWithData<i64, ()>;
type DemoNegated = RecordingNegated<i64, DemoPolygon>;
type DemoInnerLayer = Paralleled<DemoNegated>;
type DemoLayer = Paralleled<DemoInnerLayer>;
type DemoTransition = Paralleled<RecordingPolygonSet<i64, DemoPolygon>>;
type DemoLayersWithTransitions = RecordingLayersWithTransitions<i64, DemoPolygon>;
type DemoLayersDelta = LayersWithTransitionsDelta<
    i64,
    DemoPolygon,
    Paralleled<
        Paralleled<
            Negated<
                PolygonSetHalfDelta<i64, DemoPolygon>,
                Inflated<
                    PolygonUnionFindHalfDelta<i64, DemoPolygon>,
                    core::marker::PhantomData<i64>,
                >,
            >,
        >,
    >,
    Paralleled<PolygonSetHalfDelta<i64, DemoPolygon>>,
>;
const PRIMARY_INFLATION: i64 = 0;
const PARALLEL_INFLATION: i64 = 50;

fn frame_polygon() -> DemoPolygon {
    DemoPolygon {
        exterior: vec![[-2000, -2000], [2000, -2000], [2000, 2000], [-2000, 2000]],
        interiors: vec![],
        weight: (),
    }
}

fn new_negated_layer(offset: i64) -> DemoNegated {
    let mut minuend: RecordingPolygonSet<i64, DemoPolygon> = RecordingPolygonSet::new();
    let _ = minuend.add(frame_polygon());
    RecordingNegated::new(
        minuend,
        RecordingInflated::<i64, DemoPolygon>::new(offset),
    )
}

fn new_layer_stack() -> DemoLayer {
    // Match combinators demo pattern: two polygon-set stacks in parallel.
    let inner = Paralleled::new(
        new_negated_layer(PRIMARY_INFLATION),
        vec![new_negated_layer(PARALLEL_INFLATION)],
    );
    Paralleled::new(inner, vec![])
}

fn new_transition_layer() -> DemoTransition {
    let mut transition_set = RecordingPolygonSet::new();
    let _ = transition_set.add(frame_polygon());
    DemoTransition::new(transition_set, vec![])
}

fn new_layers_with_transitions() -> DemoLayersWithTransitions {
    DemoLayersWithTransitions::new(
        vec![new_layer_stack(), new_layer_stack(), new_layer_stack()],
        vec![new_transition_layer(), new_transition_layer()],
    )
}

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

fn polygon_from_ring_i32(ring: Vec<[i32; 2]>) -> DemoPolygon {
    DemoPolygon {
        exterior: ring
            .into_iter()
            .map(|[x, y]| [i64::from(x), i64::from(y)])
            .collect(),
        interiors: vec![],
        weight: (),
    }
}

fn random_polygon_at_screen_click(center: Vec2, zoom: f32, mx: f32, my: f32) -> DemoPolygon {
    let click_world = vec2((mx - center.x) / zoom, -(my - center.y) / zoom);
    let radius = (60.0 / zoom).max(10.0).round() as i32;
    let count = gen_range(3, 10) as usize;
    let ring = random_convex_polygon_at_point(
        [click_world.x.round() as i32, click_world.y.round() as i32],
        radius,
        count,
    );
    polygon_from_ring_i32(ring)
}

fn draw_polygon_set_lines(
    polygon_set: &RecordingPolygonSet<i64, DemoPolygon>,
    center: Vec2,
    zoom: f32,
    color: Color,
) {
    for (_index, polygon) in polygon_set.polygons().as_ref().iter() {
        let rings: Vec<&[[i64; 2]]> = std::iter::once(polygon.exterior.as_slice())
            .chain(polygon.interiors.iter().map(Vec::as_slice))
            .collect();
        for ring in rings.iter().copied() {
            for (from, to) in ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
            {
                let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                draw_line(start.x, start.y, end.x, end.y, 2.5, color);
            }
        }
    }
}

fn darken(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

#[macroquad::main("Layers with transitions")]
async fn main() {
    let mut undoredo: UndoRedo<DemoLayersDelta> = UndoRedo::new();
    let mut model = new_layers_with_transitions();
    let mut zoom = 1.0f32;
    let mut offset = vec2(0.0, 0.0);
    let mut show_hint = true;
    let mut last_middle_mouse: Option<Vec2> = None;
    let mut middle_press_start: Option<Vec2> = None;
    let mut middle_panned = false;

    loop {
        let undo_button = Rect::new(20.0, 20.0, 100.0, 36.0);
        let redo_button = Rect::new(130.0, 20.0, 100.0, 36.0);
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let screen_center = vec2(screen_width() * 0.5, screen_height() * 0.5);
        let mut center = screen_center + offset;

        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let middle_pressed = is_mouse_button_pressed(MouseButton::Middle);
        let right_pressed = is_mouse_button_pressed(MouseButton::Right);
        if show_hint && (left_pressed || middle_pressed || right_pressed) {
            show_hint = false;
        }

        let undo_clicked = left_pressed && undo_button.contains(mouse);
        let redo_clicked = left_pressed && redo_button.contains(mouse);

        if undo_clicked {
            undoredo.undo(&mut model);
        } else if redo_clicked {
            undoredo.redo(&mut model);
        } else if left_pressed {
            model.add_into_layer(0, random_polygon_at_screen_click(center, zoom, mx, my));
            undoredo.commit(&mut model);
        } else if right_pressed {
            model.add_into_layer(2, random_polygon_at_screen_click(center, zoom, mx, my));
            undoredo.commit(&mut model);
        }

        if middle_pressed {
            middle_press_start = Some(mouse);
            middle_panned = false;
            last_middle_mouse = Some(mouse);
        }
        if is_mouse_button_down(MouseButton::Middle) {
            if let Some(previous) = last_middle_mouse {
                let delta = mouse - previous;
                if delta.length_squared() > 0.0 {
                    offset += delta;
                    middle_panned = true;
                }
            }
            last_middle_mouse = Some(mouse);
        } else {
            last_middle_mouse = None;
        }
        if is_mouse_button_released(MouseButton::Middle) {
            if !middle_panned {
                if let Some(click_pos) = middle_press_start {
                    model.add_into_layer(
                        1,
                        random_polygon_at_screen_click(center, zoom, click_pos.x, click_pos.y),
                    );
                    undoredo.commit(&mut model);
                }
            }
            middle_press_start = None;
            middle_panned = false;
        }

        let (_, scroll_y) = mouse_wheel();
        if scroll_y != 0.0 {
            center = screen_center + offset;
            let old_zoom = zoom;
            let world_before = vec2((mx - center.x) / old_zoom, -(my - center.y) / old_zoom);
            zoom = (old_zoom * (1.0 + scroll_y * 0.1)).clamp(0.1, 20.0);
            let new_center = vec2(mx - world_before.x * zoom, my + world_before.y * zoom);
            offset = new_center - screen_center;
        }
        center = screen_center + offset;

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

        if show_hint {
            let hint = "Left: TOP, middle: MIDDLE, right: BOTTOM. Scroll to zoom.";
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

        // Draw order keeps bottom under middle under top.
        for (layer_i, base_color) in [(2usize, SKYBLUE), (1usize, ORANGE), (0usize, RED)] {
            let primary = model.layers()[layer_i].primary().primary().minuend();
            draw_polygon_set_lines(primary, center, zoom, base_color);

            let parallel = model.layers()[layer_i]
                .primary()
                .parallels()
                .first()
                .expect("each layer has one parallel polygon-set")
                .minuend();
            draw_polygon_set_lines(parallel, center, zoom, darken(base_color, 0.55));
        }

        // Transition between top and middle is visually topmost.
        let topmost_transition = model.transitions()[0].primary();
        draw_polygon_set_lines(topmost_transition, center, zoom, YELLOW);

        let other_transition = model.transitions()[1].primary();
        draw_polygon_set_lines(other_transition, center, zoom, MAGENTA);

        next_frame().await;
    }
}
