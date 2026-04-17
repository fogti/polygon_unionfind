// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod polygon;
mod polygon_unionfind;
mod union;
mod unionfind;

pub use polygon_unionfind::PolygonUnionFind;
#[cfg(feature = "undoredo")]
pub use polygon_unionfind::{
    PolygonUnionFindDelta, PolygonUnionFindHalfDelta, RecordingPolygonUnionFind,
};
pub use unionfind::UnionFind;
pub use polygon::{Polygon, PolygonWithWeight, Rings};

#[cfg(test)]
mod tests {
    use i_overlay::core::{fill_rule::FillRule, overlay_rule::OverlayRule};
    use i_overlay::float::single::SingleFloatOverlay;

    #[test]
    fn union_with_self_preserves_inner_rings() {
        let outer = vec![
            [0_f64, 0_f64],
            [10_f64, 0_f64],
            [10_f64, 10_f64],
            [0_f64, 10_f64],
        ];
        let hole = vec![
            [2_f64, 2_f64],
            [8_f64, 2_f64],
            [8_f64, 8_f64],
            [2_f64, 8_f64],
        ];
        let shape = vec![outer, hole];
        let merged = shape.overlay(&shape, OverlayRule::Union, FillRule::EvenOdd);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].len(), 2, "outer ring and one hole");
    }
}
