#![doc = include_str!("../README.md")]

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

pub use crate::unionfind::UnionFind;

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod unionfind;

#[derive(Clone, Debug)]
pub struct Polygon<K, W = ()> {
    /// Polygon exterior ring vertices.
    pub vertices: Vec<Point<K>>,
    /// User-defined payload for this polygon.
    pub weight: W,
}

/// A two-dimenstional point.
#[derive(Clone, Copy, Debug)]
pub struct Point<K> {
    pub x: K,
    pub y: K,
}

impl<K: Copy> From<[K; 2]> for Point<K> {
    #[inline]
    fn from(coords: [K; 2]) -> Self {
        Self {
            x: coords[0],
            y: coords[1],
        }
    }
}

impl<K: FromPrimitive + ToPrimitive> FloatPointCompatible<f64> for Point<K>
where
    Point<K>: Copy,
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
    W = (),
    PC: KeyedCollection = Vec<Polygon<K, W>>,
    PR: KeyedCollection = AsRefRTree<GeomWithData<Rectangle<[K; 2]>, usize>>,
    UFPC = Vec<usize>,
    UFRC = Vec<usize>,
> {
    polygons: PC,
    rtree: PR,
    unionfind: UnionFind<UFPC, UFRC>,
    scalar_marker: PhantomData<K>,
    polygon_weight_marker: PhantomData<W>,
}

impl<
    K: RTreeNum,
    W,
    PC: Default + KeyedCollection,
    PR: Default + KeyedCollection,
    UFPC: Default,
    UFRC: Default,
> PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
{
    /// Create an empty `PolygonUnionFind`.
    #[inline]
    pub fn new() -> Self {
        Self {
            polygons: Default::default(),
            rtree: Default::default(),
            unionfind: UnionFind::new(),
            scalar_marker: PhantomData,
            polygon_weight_marker: PhantomData,
        }
    }
}

impl<K: RTreeNum, W, PC: KeyedCollection, PR: KeyedCollection, UFPC, UFRC>
    PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
{
    /// Borrow the raw polygon collection backend.
    #[inline]
    pub fn raw_polygons(&self) -> &PC {
        &self.polygons
    }

    /// Borrow the spatial index backend.
    #[inline]
    pub fn rtree(&self) -> &PR {
        &self.rtree
    }

    /// Borrow the union-find backend.
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
    W,
    PC: Clone + IntoIter<usize> + Get<usize, Value = Polygon<K, W>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
where
    Point<K>: Copy,
{
    /// Return unique representative indices for currently merged polygons.
    #[inline]
    pub fn polygon_indices(&mut self) -> impl Iterator<Item = usize> {
        let mut deduplicating_set = BTreeSet::new();

        for (i, _polygon) in self.polygons.clone().into_iter() {
            deduplicating_set.insert(self.unionfind.find(i));
        }

        IntoIterator::into_iter(deduplicating_set)
    }

    /// Iterate representative polygons after all merges.
    ///
    /// If several inserted polygons have been merged into one component,
    /// only the representative polygon for that component is yielded.
    #[inline]
    pub fn polygons(&mut self) -> impl Iterator<Item = &PC::Value> {
        let mut deduplicating_set = BTreeSet::new();

        for (i, _polygon) in self.polygons.clone().into_iter() {
            deduplicating_set.insert(self.unionfind.find(i));
        }

        IntoIterator::into_iter(deduplicating_set).map(|i| self.polygons.get(&i).unwrap())
    }
}

impl<
    K: FromPrimitive + RTreeNum + ToPrimitive,
    W: Clone,
    PC: Get<usize, Value = Polygon<K, W>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
where
    Point<K>: Copy,
{
    /// Insert a polygon and merge it with any neighboring polygon.
    ///
    /// Returns the polygon's new id even if it was absorbed by another polygon.
    pub fn insert(&mut self, mut polygon: Polygon<K, W>) -> usize {
        let new_polygon_index = self.unionfind.new_set();

        let rectangle = Self::rectangle_from_polygon(&polygon);
        self.rtree
            .insert(GeomWithData::new(rectangle.clone(), new_polygon_index), ());

        let id = self.polygons.push(polygon.clone());

        for neighbor in self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rectangle.envelope())
        {
            let neighbor_representative = self.unionfind.find_compress(neighbor.data);

            let vertices: Vec<[f64; 2]> = polygon
                .vertices
                .iter()
                .map(|vertex| [vertex.x(), vertex.y()])
                .collect();
            let neighbor_vertices: Vec<[f64; 2]> = self
                .polygons
                .get(&neighbor_representative)
                .unwrap()
                .vertices
                .iter()
                .map(|vertex| [vertex.x(), vertex.y()])
                .collect();
            let union = vertices.overlay(&neighbor_vertices, OverlayRule::Union, FillRule::EvenOdd);

            if union.len() >= 2 {
                continue;
            }

            if let Some(union) = union.get(&0) {
                self.unionfind
                    .union(neighbor_representative, new_polygon_index);

                //let representative = self.unionfind.find_compress(new_polygon_index);
                let representative = self.unionfind.find(new_polygon_index);
                polygon = Polygon {
                    vertices: union[0]
                        .iter()
                        .map(|vertex| Point {
                            x: K::from_f64(vertex[0]).unwrap(),
                            y: K::from_f64(vertex[1]).unwrap(),
                        })
                        .collect(),
                    weight: self.polygons.get(&representative).unwrap().weight.clone(),
                };
                self.polygons.set(representative, polygon.clone());
            }
        }

        id
    }

    /// Replace the weight associated the polygon under `id`.
    ///
    /// This does not change the polygon's shape anyhow.
    pub fn set_weight(&mut self, id: usize, weight: W) {
        let polygon = self.polygons.get(&id).unwrap();
        self.polygons.set(
            id,
            Polygon {
                vertices: polygon.vertices.clone(),
                weight,
            },
        );
    }

    /// Find the representative polygon for `index` without path compression.
    ///
    /// If you want to path compression to be performed, use [`find_compress()`]
    /// instead.
    #[inline]
    pub fn find(&self, index: usize) -> &Polygon<K, W> {
        self.polygons.get(&self.unionfind.find(index)).unwrap()
    }

    /// Find the representative polygon for `index`, applying path compression
    /// along the way.
    ///
    /// [https://cp-algorithms.com/data_structures/disjoint_set_union.html#path-compression-optimization](Path compression)
    /// is an optimization that speeds up finding an element by flattening the
    /// tree formed by all the connected nodes.
    pub fn find_compress(&mut self, index: usize) -> &Polygon<K, W> {
        self.polygons
            .get(&self.unionfind.find_compress(index))
            .unwrap()
    }

    fn rectangle_from_polygon(polygon: &Polygon<K, W>) -> Rectangle<[K; 2]> {
        Rectangle::from_aabb(
            polygon
                .vertices
                .iter()
                .fold(AABB::new_empty(), |aabb, vertex| {
                    aabb.merged(&AABB::from_point([vertex.x, vertex.y]))
                }),
        )
    }
}

impl<K: RTreeNum, W, PC: Clear, PR: Clear, UFPC: Clear, UFRC: Clear>
    PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
{
    /// Remove all polygons and reset indexing/union state.
    pub fn clear(&mut self) {
        self.polygons.clear();
        self.rtree.clear();
        self.unionfind.clear();
    }
}

#[cfg(feature = "undoredo")]
/// `PolygonUnionFind` with Undo/Redo.
pub type RecordingPolygonUnionFind<K, W = ()> = PolygonUnionFind<
    K,
    W,
    Recorder<Vec<Polygon<K, W>>, BTreeMap<usize, Polygon<K, W>>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<Vec<usize>>,
    Recorder<Vec<usize>>,
>;

#[cfg(feature = "undoredo")]
/// Delta-serializable `PolygonUnionFind` representation for undo/redo snapshots.
pub type PolygonUnionFindDelta<K, W = ()> = PolygonUnionFind<
    K,
    W,
    BTreeMap<usize, Polygon<K, W>>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, usize>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    W: Clone,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + Clone + ApplyDelta<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + Clone + ApplyDelta<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: Clone + ApplyDelta<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: Clone + ApplyDelta<UFRCE>,
> ApplyDelta<PolygonUnionFind<K, W, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
{
    fn apply_delta(&mut self, delta: &Delta<PolygonUnionFind<K, W, PCE, PRE, UFPCE, UFRCE>>) {
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
    W: Clone,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + FlushDelta<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + FlushDelta<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: FlushDelta<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: FlushDelta<UFRCE>,
> FlushDelta<PolygonUnionFind<K, W, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
{
    fn flush_delta(&mut self) -> Delta<PolygonUnionFind<K, W, PCE, PRE, UFPCE, UFRCE>> {
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();
        let (removed_rtree, inserted_rtree) = self.rtree.flush_delta().dissolve();
        let (removed_unionfind, inserted_unionfind) = self.unionfind.flush_delta().dissolve();

        Delta::with_removed_inserted(
            PolygonUnionFind {
                polygons: removed_polygons,
                rtree: removed_rtree,
                unionfind: removed_unionfind,
                scalar_marker: PhantomData,
                polygon_weight_marker: PhantomData,
            },
            PolygonUnionFind {
                polygons: inserted_polygons,
                rtree: inserted_rtree,
                unionfind: inserted_unionfind,
                scalar_marker: PhantomData,
                polygon_weight_marker: PhantomData,
            },
        )
    }
}
