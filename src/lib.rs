// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod bool_ops;
mod polygon;
mod polygon_set;
mod polygon_unionfind;
mod unionfind;

pub use polygon::{Polygon, PolygonId, PolygonWithWeight, Rings};
pub use polygon_set::PolygonSet;
#[cfg(feature = "undoredo")]
pub use polygon_set::{PolygonSetDelta, PolygonSetHalfDelta, RecordingPolygonSet};
pub use polygon_unionfind::PolygonUnionFind;
#[cfg(feature = "undoredo")]
pub use polygon_unionfind::{
    PolygonUnionFindDelta, PolygonUnionFindHalfDelta, RecordingPolygonUnionFind,
};
pub use unionfind::UnionFind;

pub trait Add<P> {
    fn add(&mut self, polygon: P) -> PolygonId;
}

pub trait Subtract<P> {
    fn subtract(&mut self, polygon: P) -> Vec<PolygonId>;
}
