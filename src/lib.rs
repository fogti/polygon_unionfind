// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{collections::BTreeSet, marker::PhantomData};

use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    i_float::float::compatible::FloatPointCompatible,
};

use maplike::{Clear, Get, Insert, IntoIter, KeyedCollection, Push, Set};
use num_traits::{FromPrimitive, ToPrimitive};
use rstar::{
    AABB, Envelope, RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;
#[cfg(feature = "undoredo")]
use std::collections::BTreeMap;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::unionfind::UnionFind;

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

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
            let neighbor_representative = self.unionfind.find_compress(neighbor.data);

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

impl<K: RTreeNum, PC: Clear, PR: Clear, UFPC: Clear, UFRC: Clear>
    PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    pub fn clear(&mut self) {
        self.polygons.clear();
        self.rtree.clear();
        self.unionfind.clear();
    }
}

#[cfg(feature = "undoredo")]
pub type RecordingPolygonUnionFind<K> = PolygonUnionFind<
    K,
    Recorder<Vec<Vec<Point2<K>>>, BTreeMap<usize, Vec<Point2<K>>>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<Vec<usize>>,
    Recorder<Vec<usize>>,
>;

#[cfg(feature = "undoredo")]
pub type PolygonUnionFindDelta<K> = PolygonUnionFind<
    K,
    BTreeMap<usize, Vec<Point2<K>>>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, usize>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + Clone + ApplyDelta<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + Clone + ApplyDelta<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: Clone + ApplyDelta<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: Clone + ApplyDelta<UFRCE>,
> ApplyDelta<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn apply_delta(&mut self, delta: &Delta<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>) {
        let (removed, inserted) = delta.clone().dissolve();

        let polygons_delta = Delta::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_delta(&polygons_delta);

        let rtree_delta = Delta::with_removed_inserted(removed.rtree, inserted.rtree);
        self.rtree.apply_delta(&rtree_delta);

        let unionfind_delta = Delta::with_removed_inserted(removed.unionfind, inserted.unionfind);
        self.unionfind.apply_delta(&unionfind_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + FlushDelta<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + FlushDelta<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: FlushDelta<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: FlushDelta<UFRCE>,
> FlushDelta<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn flush_delta(&mut self) -> Delta<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>> {
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();
        let (removed_rtree, inserted_rtree) = self.rtree.flush_delta().dissolve();
        let (removed_unionfind, inserted_unionfind) = self.unionfind.flush_delta().dissolve();

        Delta::with_removed_inserted(
            PolygonUnionFind {
                polygons: removed_polygons,
                rtree: removed_rtree,
                unionfind: removed_unionfind,
                scalar_marker: PhantomData,
            },
            PolygonUnionFind {
                polygons: inserted_polygons,
                rtree: inserted_rtree,
                unionfind: inserted_unionfind,
                scalar_marker: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    // TODO: tests.
}
