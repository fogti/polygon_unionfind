// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::marker::PhantomData;

use alloc::collections::BTreeMap;
use maplike::KeyedCollection;
use rstar::{
    RTree, RTreeNum,
    primitives::{GeomWithData, Rectangle},
};
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::{Point2, PolygonUnionFind, UnionFind};

pub type RecordingPolygonUnionFind<K> = PolygonUnionFind<
    K,
    Recorder<Vec<Vec<Point2<K>>>, BTreeMap<usize, Vec<Point2<K>>>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<Vec<usize>>,
    Recorder<Vec<usize>>,
>;

pub type PolygonUnionFindDelta<K> = PolygonUnionFind<
    K,
    BTreeMap<usize, Vec<Point2<K>>>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, usize>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

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

        let polygons_delta =
            undoredo::Delta::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_delta(&polygons_delta);

        let rtree_delta = undoredo::Delta::with_removed_inserted(removed.rtree, inserted.rtree);
        self.rtree.apply_delta(&rtree_delta);

        let unionfind_delta =
            undoredo::Delta::with_removed_inserted(removed.unionfind, inserted.unionfind);
        self.unionfind.apply_delta(&unionfind_delta);
    }
}

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

        undoredo::Delta::with_removed_inserted(
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

impl<
    PCE: Clone + KeyedCollection,
    PC: Clone + ApplyDelta<PCE>,
    RCE: Clone + KeyedCollection,
    RC: Clone + ApplyDelta<RCE>,
> ApplyDelta<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn apply_delta(&mut self, delta: &Delta<UnionFind<PCE, RCE>>) {
        let (removed, inserted) = delta.clone().dissolve();

        let parents_delta =
            undoredo::Delta::with_removed_inserted(removed.parents, inserted.parents);
        self.parents.apply_delta(&parents_delta);

        let ranks_delta = undoredo::Delta::with_removed_inserted(removed.ranks, inserted.ranks);
        self.ranks.apply_delta(&ranks_delta);
    }
}

impl<PCE: KeyedCollection, PC: FlushDelta<PCE>, RCE: KeyedCollection, RC: FlushDelta<RCE>>
    FlushDelta<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn flush_delta(&mut self) -> Delta<UnionFind<PCE, RCE>> {
        let (removed_parents, inserted_parents) = self.parents.flush_delta().dissolve();
        let (removed_ranks, inserted_ranks) = self.ranks.flush_delta().dissolve();

        undoredo::Delta::with_removed_inserted(
            UnionFind::from_parents_ranks(removed_parents, removed_ranks),
            UnionFind::from_parents_ranks(inserted_parents, inserted_ranks),
        )
    }
}
