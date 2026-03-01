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

use crate::{Point2, PolygonUnionFind};

impl<
    K: RTreeNum,
    PC: KeyedCollection + Clone + ApplyEdit<PC>,
    PR: KeyedCollection + Clone + ApplyEdit<PR>,
    UFPC: Clone + ApplyEdit<UFPC>,
    UFRC: Clone + ApplyEdit<UFRC>,
> ApplyEdit<PolygonUnionFind<K, PC, PR, UFPC, UFRC>> for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn apply_edit(&mut self, edit: &Edit<PolygonUnionFind<K, PC, PR, UFPC, UFRC>>) {
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
    PC: KeyedCollection + FlushEdit<PC>,
    PR: KeyedCollection + FlushEdit<PR>,
    UFPC: FlushEdit<UFPC>,
    UFRC: FlushEdit<UFRC>,
> FlushEdit<PolygonUnionFind<K, PC, PR, UFPC, UFRC>> for PolygonUnionFind<K, PC, PR, UFPC, UFRC>
{
    fn flush_edit(&mut self) -> Edit<PolygonUnionFind<K, PC, PR, UFPC, UFRC>> {
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

pub type RecordingPolygonUnionFind<K> = PolygonUnionFind<
    K,
    Recorder<Vec<Vec<Point2<K>>>, BTreeMap<usize, Vec<Point2<K>>>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, usize>>>,
    Recorder<StableVec<usize>>,
    Recorder<StableVec<usize>>,
>;
