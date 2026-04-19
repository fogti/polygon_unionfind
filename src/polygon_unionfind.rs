// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{collections::BTreeSet, marker::PhantomData};

use maplike::{Clear, Container, Get, Insert, IntoIter, Push, Remove, Set};
use rstar::{
    AABB, Envelope, RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;
#[cfg(feature = "undoredo")]
use std::collections::BTreeMap;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

#[cfg(feature = "undoredo")]
use crate::PolygonWithWeight;
use crate::Rings;
use crate::boolops::Union;
use crate::unionfind::UnionFind;
use crate::{Polygon, polygon::PolygonId};

#[derive(Clone, Debug)]
pub struct PolygonUnionFind<
    K: RTreeNum,
    P = Polygon<K>,
    PC: Container = Vec<P>,
    PR: Container = AsRefRTree<GeomWithData<Rectangle<[K; 2]>, usize>>,
    UFPC = Vec<usize>,
    UFRC = Vec<usize>,
> {
    polygons: PC,
    rtree: PR,
    unionfind: UnionFind<UFPC, UFRC>,
    scalar_marker: PhantomData<K>,
    polygon_marker: PhantomData<P>,
}

impl<K: RTreeNum, P, PC: Default + Container, PR: Default + Container, UFPC: Default, UFRC: Default>
    PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Create an empty `PolygonUnionFind`.
    #[inline]
    pub fn new() -> Self {
        Self {
            polygons: Default::default(),
            rtree: Default::default(),
            unionfind: UnionFind::new(),
            scalar_marker: PhantomData,
            polygon_marker: PhantomData,
        }
    }
}

impl<K: RTreeNum, P, PC: Container, PR: Container, UFPC, UFRC>
    PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Returns a reference to underlying raw polygon collection.
    #[inline]
    pub fn raw_polygons(&self) -> &PC {
        &self.polygons
    }

    /// Returns a reference to the underlying R-tree.
    #[inline]
    pub fn rtree(&self) -> &PR {
        &self.rtree
    }

    /// Returns a reference to the underlying union-find.
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
    K: RTreeNum,
    P,
    PC: Clone + IntoIter<usize> + Get<usize, Value = P> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
where
    K: Copy,
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
    K: RTreeNum,
    P: Clone + Rings<K> + Union<P>,
    PC: Get<usize, Value = P> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>
        + Remove<GeomWithData<Rectangle<[K; 2]>, usize>>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
where
    K: Copy,
{
    /// Insert a polygon and merge it with any neighboring polygon.
    ///
    /// Returns the polygon's new id even if it was immediately absorbed by
    /// another polygon.
    pub fn insert(&mut self, mut polygon: P) -> PolygonId {
        let new_polygon_index = self.unionfind.new_set();

        let rectangle = Self::rectangle_from_polygon(&polygon);
        self.rtree
            .insert(GeomWithData::new(rectangle.clone(), new_polygon_index), ());

        let id = self.polygons.push(polygon.clone());

        let neighbor_ids: Vec<usize> = self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rectangle.envelope())
            .map(|neighbor| neighbor.data)
            .collect();

        for neighbor_id in neighbor_ids {
            let neighbor_representative = self.unionfind.find_compress(neighbor_id);
            let root_of_inserted = self.unionfind.find_compress(new_polygon_index);

            if neighbor_representative == root_of_inserted {
                continue;
            }

            let neighbor = self.polygons.get(&neighbor_representative).unwrap().clone();
            let Some(merged) = <P as Union<P>>::union(polygon.clone(), neighbor) else {
                continue;
            };

            let root_of_neighbor = neighbor_representative;
            self.unionfind.union(root_of_neighbor, root_of_inserted);

            let representative = self.unionfind.find_compress(new_polygon_index);
            polygon = merged;
            self.polygons.set(representative, polygon.clone());

            let absorbed = if representative == root_of_neighbor {
                root_of_inserted
            } else {
                root_of_neighbor
            };

            // Remove the absorbed polygon from R-tree to shorten query
            // times.
            self.remove_polygon_from_rtree(absorbed);

            // The bbox changed, so we need to reinsert the polygon in R-tree.
            self.reinsert_polygon_in_rtree(representative, &polygon);
        }

        PolygonId::new(id)
    }

    fn remove_polygon_from_rtree(&mut self, polygon_id: usize) {
        let item = self
            .rtree
            .as_ref()
            .iter()
            .find(|g| g.data == polygon_id)
            .cloned();
        if let Some(item) = item {
            self.rtree.remove(&item);
        }
    }

    fn reinsert_polygon_in_rtree(&mut self, polygon_id: usize, polygon: &P) {
        self.remove_polygon_from_rtree(polygon_id);
        let rect = Self::rectangle_from_polygon(polygon);
        self.rtree.insert(GeomWithData::new(rect, polygon_id), ());
    }

    /// Find the representative polygon for `index` without path compression.
    ///
    /// If you want to path compression to be performed, use [`find_compress()`]
    /// instead.
    #[inline]
    pub fn find(&self, index: usize) -> &P {
        self.polygons.get(&self.unionfind.find(index)).unwrap()
    }

    /// Find the representative polygon for `index`, applying path compression
    /// along the way.
    ///
    /// [https://cp-algorithms.com/data_structures/disjoint_set_union.html#path-compression-optimization](Path compression)
    /// is an optimization that speeds up finding an element by flattening the
    /// tree formed by all the connected nodes.
    pub fn find_compress(&mut self, index: usize) -> &P {
        self.polygons
            .get(&self.unionfind.find_compress(index))
            .unwrap()
    }

    fn rectangle_from_polygon(polygon: &P) -> Rectangle<[K; 2]> {
        Rectangle::from_aabb(
            polygon
                .exterior()
                .iter()
                .fold(AABB::new_empty(), |aabb, vertex| {
                    aabb.merged(&AABB::from_point([vertex[0], vertex[1]]))
                }),
        )
    }
}

impl<K: RTreeNum, P, PC: Clear, PR: Clear, UFPC: Clear, UFRC: Clear>
    PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Remove all polygons and their relations.
    ///
    /// This resets the strucutre's state to as if it was returned freshly from
    /// [`new()`].
    pub fn clear(&mut self) {
        self.polygons.clear();
        self.rtree.clear();
        self.unionfind.clear();
    }
}

#[cfg(feature = "undoredo")]
/// `PolygonUnionFind` that records changes for delta-based Undo/Redo.
pub type RecordingPolygonUnionFind<K, P = PolygonWithWeight<K>> = PolygonUnionFind<
    K,
    P,
    Recorder<Vec<P>, BTreeMap<usize, P>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<Vec<usize>>,
    Recorder<Vec<usize>>,
>;

#[cfg(feature = "undoredo")]
/// Half-delta of `PolygonUnionFind`.
pub type PolygonUnionFindHalfDelta<K, P = PolygonWithWeight<K>> = PolygonUnionFind<
    K,
    P,
    BTreeMap<usize, P>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, usize>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

#[cfg(feature = "undoredo")]
/// Delta of `PolygonUnionFind` for delta-based Undo/Redo.
pub type PolygonUnionFindDelta<K, P = PolygonWithWeight<K>> =
    Delta<PolygonUnionFindHalfDelta<K, P>>;

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P: Clone,
    PE: Clone + Container<Value = P>,
    PC: Container<Value = P> + Clone + ApplyDelta<PE>,
    PRE: Clone + Container,
    PR: Container + Clone + ApplyDelta<PRE>,
    UFPCE: Clone + Container,
    UFPC: Clone + ApplyDelta<UFPCE>,
    UFRCE: Clone + Container,
    UFRC: Clone + ApplyDelta<UFRCE>,
> ApplyDelta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    fn apply_delta(&mut self, delta: Delta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>) {
        let (removed, inserted) = delta.dissolve();

        let polygons_delta = Delta::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_delta(polygons_delta);

        let rtree_delta = Delta::with_removed_inserted(removed.rtree, inserted.rtree);
        self.rtree.apply_delta(rtree_delta);

        let unionfind_delta = Delta::with_removed_inserted(removed.unionfind, inserted.unionfind);
        self.unionfind.apply_delta(unionfind_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P: Clone,
    PE: Clone + Container<Value = P>,
    PC: Container<Value = P> + FlushDelta<PE>,
    PRE: Clone + Container,
    PR: Container + FlushDelta<PRE>,
    UFPCE: Clone + Container,
    UFPC: FlushDelta<UFPCE>,
    UFRCE: Clone + Container,
    UFRC: FlushDelta<UFRCE>,
> FlushDelta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    fn flush_delta(&mut self) -> Delta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>> {
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();
        let (removed_rtree, inserted_rtree) = self.rtree.flush_delta().dissolve();
        let (removed_unionfind, inserted_unionfind) = self.unionfind.flush_delta().dissolve();

        Delta::with_removed_inserted(
            PolygonUnionFind {
                polygons: removed_polygons,
                rtree: removed_rtree,
                unionfind: removed_unionfind,
                scalar_marker: PhantomData,
                polygon_marker: PhantomData,
            },
            PolygonUnionFind {
                polygons: inserted_polygons,
                rtree: inserted_rtree,
                unionfind: inserted_unionfind,
                scalar_marker: PhantomData,
                polygon_marker: PhantomData,
            },
        )
    }
}
