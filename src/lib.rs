// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod bool_ops;
mod combinators;
mod inflate;
mod polygon;
mod polygon_set;
mod polygon_unionfind;
mod unionfind;

pub use combinators::{Inflated, Negated, Paralleled};
pub use inflate::Inflate;
pub use polygon::{Polygon, PolygonId, PolygonWithData, Rings};
pub use polygon_set::PolygonSet;
#[cfg(feature = "undoredo")]
pub use polygon_set::{PolygonSetDelta, PolygonSetHalfDelta, RecordingPolygonSet};
pub use polygon_unionfind::PolygonUnionFind;
#[cfg(feature = "undoredo")]
pub use polygon_unionfind::{
    PolygonUnionFindDelta, PolygonUnionFindHalfDelta, RecordingPolygonUnionFind,
};
pub use unionfind::UnionFind;

pub trait Include<P> {
    type Output;

    fn include(&mut self, polygon: P) -> Self::Output;
}

pub trait Exclude<P> {
    type Output;

    fn exclude(&mut self, polygon: P) -> Self::Output;
}

pub trait Clip<P> {
    type Output;

    fn clip(&mut self, polygon: P) -> Self::Output;
}
