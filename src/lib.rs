#![doc = include_str!("../README.md")]

// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{collections::BTreeSet, marker::PhantomData};

use maplike::{Clear, Get, Insert, IntoIter, KeyedCollection, Push, Remove, Set};
use rstar::{
    AABB, Envelope, RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;
#[cfg(feature = "undoredo")]
use std::collections::BTreeMap;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::union::Union;
pub use crate::unionfind::UnionFind;

#[cfg(feature = "std")]
extern crate std;

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod union;
mod unionfind;

#[derive(Clone, Debug)]
pub struct Polygon<K, W = ()> {
    /// Polygon boundary.
    pub exterior: Vec<[K; 2]>,
    /// Polygon interior rings.
    pub interiors: Vec<Vec<[K; 2]>>,
    /// User-defined payload for this polygon.
    pub weight: W,
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
    K: RTreeNum,
    W,
    PC: Clone + IntoIter<usize> + Get<usize, Value = Polygon<K, W>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
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
    W: Clone,
    PC: Get<usize, Value = Polygon<K, W>> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, usize>, Value = ()>
        + Remove<GeomWithData<Rectangle<[K; 2]>, usize>>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, W, PC, PR, UFPC, UFRC>
where
    K: Copy,
    Polygon<K, W>: Union<Polygon<K, W>>,
{
    /// Insert a polygon and merge it with any neighboring polygon.
    ///
    /// Returns the polygon's new id even if it was immediately absorbed by
    /// another polygon.
    pub fn insert(&mut self, mut polygon: Polygon<K, W>) -> usize {
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
            let Some(merged) =
                <Polygon<K, W> as Union<Polygon<K, W>>>::union(polygon.clone(), neighbor)
            else {
                continue;
            };

            let root_of_neighbor = neighbor_representative;
            self.unionfind.union(root_of_neighbor, root_of_inserted);

            let representative = self.unionfind.find_compress(new_polygon_index);
            let merged_weight = self.polygons.get(&representative).unwrap().weight.clone();
            polygon = Polygon {
                exterior: merged.exterior,
                interiors: merged.interiors,
                weight: merged_weight,
            };
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

        id
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

    fn reinsert_polygon_in_rtree(&mut self, polygon_id: usize, polygon: &Polygon<K, W>) {
        self.remove_polygon_from_rtree(polygon_id);
        let rect = Self::rectangle_from_polygon(polygon);
        self.rtree.insert(GeomWithData::new(rect, polygon_id), ());
    }

    /// Set the weight (payload) associated with the polygon under `index`.
    ///
    /// This does not change the polygon's shape anyhow.
    pub fn set_weight(&mut self, index: usize, weight: W) {
        let polygon = self.polygons.get(&index).unwrap();
        self.polygons.set(
            index,
            Polygon {
                exterior: polygon.exterior.clone(),
                interiors: polygon.interiors.clone(),
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
                .exterior
                .iter()
                .fold(AABB::new_empty(), |aabb, vertex| {
                    aabb.merged(&AABB::from_point([vertex[0], vertex[1]]))
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
