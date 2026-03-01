// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{
    collections::BTreeSet,
    marker::PhantomData,
};

use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    i_float::float::compatible::FloatPointCompatible,
};

use maplike::{Get, Insert, IntoIter, KeyedCollection, Push, Set};
use num_traits::{FromPrimitive, ToPrimitive};
use rstar::{
    AABB, Envelope, RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;

use crate::unionfind::UnionFind;

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

#[cfg(feature = "undoredo")]
mod undoredo;
#[cfg(feature = "undoredo")]
pub use undoredo::RecordingPolygonUnionFind;

mod unionfind;

#[derive(Clone, Copy, Debug)]
pub struct Point2<K> {
    pub x: K,
    pub y: K,
}

impl<K: Copy> From<[K; 2]> for Point2<K> {
    #[inline]
    fn from(coords: [K; 2]) -> Self {
        Self {
            x: coords[0],
            y: coords[1],
        }
    }
}

impl<K: FromPrimitive + ToPrimitive> FloatPointCompatible<f64> for Point2<K>
where
    Point2<K>: Copy,
{
    #[inline]
    fn from_xy(x: f64, y: f64) -> Self {
        Self {
            x: K::from_f64(x).unwrap(),
            y: K::from_f64(y).unwrap(),
        }
    }

    #[inline]
    fn x(&self) -> f64 {
        self.x.to_f64().unwrap()
    }

    #[inline]
    fn y(&self) -> f64 {
        self.y.to_f64().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct PolygonUnionFind<
    K: RTreeNum,
    PC: KeyedCollection = Vec<Vec<Point2<K>>>,
    PR: KeyedCollection = AsRefRTree<GeomWithData<Rectangle<[K; 2]>, usize>>,
    UFPC = Vec<usize>,
    UFRC = Vec<usize>,
> {
    polygons: PC,
    rtree: PR,
    unionfind: UnionFind<UFPC, UFRC>,
    scalar_marker: PhantomData<K>,
}

impl<
    K: RTreeNum,
    PC: Default + KeyedCollection,
    PR: Default + KeyedCollection,
    UFPC: Default,
    UFRC: Default,
> PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    #[inline]
    pub fn new() -> Self {
        Self {
            polygons: Default::default(),
            rtree: Default::default(),
            unionfind: UnionFind::new(),
            scalar_marker: PhantomData,
        }
    }
}

impl<K: RTreeNum, PC: KeyedCollection, PR: KeyedCollection, UFPC, UFRC>
    PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    #[inline]
    pub fn raw_polygons(&self) -> &PC {
        &self.polygons
    }

    #[inline]
    pub fn rtree(&self) -> &PR {
        &self.rtree
    }

    #[inline]
    pub fn unionfind(&self) -> &UnionFind<UFPC, UFRC> {
        &self.unionfind
    }

    /// Dissolve the polygon-union-find, returning its internal polygons,
    /// R-tree, and union-find, ceding ownership over them.
    #[inline]
    pub fn dissolve(self) -> (PC, PR, UnionFind<UFPC, UFRC>) {
        (self.polygons, self.rtree, self.unionfind)
    }
}

impl<
    K: FromPrimitive + RTreeNum + ToPrimitive,
    PC: Clone + IntoIter<usize> + Get<usize, Value = Vec<Point2<K>>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, PC, PR, UFPC, UFRC>
where
    Point2<K>: Copy,
{
    #[inline]
    pub fn polygons(&mut self) -> impl Iterator<Item = PC::Value> {
        let mut deduplicating_set = BTreeSet::new();

        for (i, _polygon) in self.polygons.clone().into_iter() {
            deduplicating_set.insert(self.unionfind.find(i));
        }

        IntoIterator::into_iter(deduplicating_set).map(|i| self.polygons.get(&i).unwrap().clone())
    }
}

impl<
    K: FromPrimitive + RTreeNum + ToPrimitive,
    PC: Get<usize, Value = Vec<Point2<K>>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, PC, PR, UFPC, UFRC>
where
    Point2<K>: Copy,
{
    pub fn insert(&mut self, polygon: impl IntoIterator<Item = [K; 2]>) {
        let new_polygon_index = self.unionfind.new_set();

        let mut polygon: Vec<Point2<K>> = polygon.into_iter().map(Into::into).collect();
        let rectangle = Self::rectangle_from_polygon(&polygon);
        self.rtree
            .insert(GeomWithData::new(rectangle.clone(), new_polygon_index), ());

        self.polygons.push(polygon.clone());

        for neighbor in self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rectangle.envelope())
        {
            //let neighbor_representative = self.unionfind.find_compress(neighbor.data);
            let neighbor_representative = self.unionfind.find(neighbor.data);

            let union = polygon.overlay(
                self.polygons.get(&neighbor_representative).unwrap(),
                OverlayRule::Union,
                FillRule::EvenOdd,
            );

            if union.len() >= 2 {
                continue;
            }

            if let Some(union) = union.get(&0) {
                self.unionfind
                    .union(neighbor_representative, new_polygon_index);

                //let representative = self.unionfind.find_compress(new_polygon_index);
                let representative = self.unionfind.find(new_polygon_index);
                self.polygons.set(representative, union[0].clone());
                polygon = union[0].clone();
            }
        }
    }

    fn rectangle_from_polygon(vertices: &[Point2<K>]) -> Rectangle<[K; 2]> {
        Rectangle::from_aabb(
            vertices
                .into_iter()
                .fold(AABB::new_empty(), |aabb, vertex| {
                    aabb.merged(&AABB::from_point([vertex.x, vertex.y]))
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: tests.
}
