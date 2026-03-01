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
use stable_vec::StableVec;
use undoredo::{ApplyEdit, Edit, FlushEdit, Recorder};

use crate::{Point2, PolygonUnionFind, UnionFind};

pub type RecordingPolygonUnionFind<K> = PolygonUnionFind<
    K,
    Recorder<Vec<Vec<Point2<K>>>, BTreeMap<usize, Vec<Point2<K>>>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<StableVec<usize>>,
    Recorder<StableVec<usize>>,
>;

pub type PolygonUnionFindEdit<K> = PolygonUnionFind<
    K,
    BTreeMap<usize, Vec<Point2<K>>>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, usize>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

impl<
    K: RTreeNum,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + Clone + ApplyEdit<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + Clone + ApplyEdit<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: Clone + ApplyEdit<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: Clone + ApplyEdit<UFRCE>,
> ApplyEdit<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn apply_edit(&mut self, edit: &Edit<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>) {
        let (removed, inserted) = edit.clone().dissolve();

        let polygons_edit =
            undoredo::Edit::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_edit(&polygons_edit);

        let rtree_edit = undoredo::Edit::with_removed_inserted(removed.rtree, inserted.rtree);
        self.rtree.apply_edit(&rtree_edit);

        let unionfind_edit =
            undoredo::Edit::with_removed_inserted(removed.unionfind, inserted.unionfind);
        self.unionfind.apply_edit(&unionfind_edit);
    }
}

impl<
    K: RTreeNum,
    PCE: Clone + KeyedCollection,
    PC: KeyedCollection + FlushEdit<PCE>,
    PRE: Clone + KeyedCollection,
    PR: KeyedCollection + FlushEdit<PRE>,
    UFPCE: Clone + KeyedCollection,
    UFPC: FlushEdit<UFPCE>,
    UFRCE: Clone + KeyedCollection,
    UFRC: FlushEdit<UFRCE>,
> FlushEdit<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn flush_edit(&mut self) -> Edit<PolygonUnionFind<K, PCE, PRE, UFPCE, UFRCE>> {
        let (removed_polygons, inserted_polygons) = self.polygons.flush_edit().dissolve();
        let (removed_rtree, inserted_rtree) = self.rtree.flush_edit().dissolve();
        let (removed_unionfind, inserted_unionfind) = self.unionfind.flush_edit().dissolve();

        undoredo::Edit::with_removed_inserted(
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
    PC: Clone + ApplyEdit<PCE>,
    RCE: Clone + KeyedCollection,
    RC: Clone + ApplyEdit<RCE>,
> ApplyEdit<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn apply_edit(&mut self, edit: &Edit<UnionFind<PCE, RCE>>) {
        let (removed, inserted) = edit.clone().dissolve();

        let parents_edit = undoredo::Edit::with_removed_inserted(removed.parents, inserted.parents);
        self.parents.apply_edit(&parents_edit);

        let ranks_edit = undoredo::Edit::with_removed_inserted(removed.ranks, inserted.ranks);
        self.ranks.apply_edit(&ranks_edit);
    }
}

impl<PCE: KeyedCollection, PC: FlushEdit<PCE>, RCE: KeyedCollection, RC: FlushEdit<RCE>>
    FlushEdit<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn flush_edit(&mut self) -> Edit<UnionFind<PCE, RCE>> {
        let (removed_parents, inserted_parents) = self.parents.flush_edit().dissolve();
        let (removed_ranks, inserted_ranks) = self.ranks.flush_edit().dissolve();

        undoredo::Edit::with_removed_inserted(
            UnionFind::from_parents_ranks(removed_parents, removed_ranks),
            UnionFind::from_parents_ranks(inserted_parents, inserted_ranks),
        )
    }
}
