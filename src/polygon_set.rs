// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::marker::PhantomData;

use maplike::{Get, Insert, Push, Remove, Set};
use rstar::{
    AABB, Envelope, RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;
use stable_vec::StableVec;

use crate::{
    Polygon, PolygonId, Rings,
    bool_ops::{Difference, Union},
};

#[derive(Clone, Debug)]
pub struct PolygonSet<
    K: RTreeNum,
    P = Polygon<K>,
    PC = StableVec<P>,
    PR = AsRefRTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>,
> {
    polygons: PC,
    rtree: PR,
    scalar_marker: PhantomData<K>,
    polygon_marker: PhantomData<P>,
}

impl<K: RTreeNum, P, PC: Default, PR: Default> Default for PolygonSet<K, P, PC, PR> {
    fn default() -> Self {
        Self {
            polygons: PC::default(),
            rtree: PR::default(),
            scalar_marker: PhantomData,
            polygon_marker: PhantomData,
        }
    }
}

impl<K: RTreeNum, P, PC: Default, PR: Default> PolygonSet<K, P, PC, PR> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, P, PC, PR> PolygonSet<K, P, PC, PR>
where
    K: RTreeNum,
    P: Clone + Rings<K> + Union<P> + Difference<P>,
    PC: Get<usize, Value = P> + Push<usize> + Remove<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, PolygonId>, Value = ()>
        + Remove<GeomWithData<Rectangle<[K; 2]>, PolygonId>>,
{
    #[inline]
    pub fn polygons(&self) -> &PC {
        &self.polygons
    }

    #[inline]
    pub fn rtree(&self) -> &PR {
        &self.rtree
    }

    pub fn add(&mut self, polygon: P) -> PolygonId {
        let rectangle = Self::rectangle_from_polygon(&polygon);
        let neighbor_ids: Vec<PolygonId> = self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rectangle.envelope())
            .map(|neighbor| neighbor.data)
            .collect();

        let mut maybe_absorber_id: Option<PolygonId> = None;

        for neighbor_id in neighbor_ids {
            let neighbor = self.polygons.get(&neighbor_id.index()).unwrap().clone();

            if let Some(absorber_id) = maybe_absorber_id {
                let absorber = self.polygons.get(&absorber_id.index()).unwrap().clone();

                if let Some(merged) = P::union(absorber, neighbor) {
                    self.polygons.set(absorber_id.index(), merged.clone());
                    self.reinsert_polygon_in_rtree(absorber_id, &merged);

                    self.remove_polygon_from_rtree(neighbor_id);
                    self.polygons.remove(&neighbor_id.index());
                }
            } else {
                if let Some(merged) = P::union(polygon.clone(), neighbor.clone()) {
                    self.polygons.set(neighbor_id.index(), merged.clone());
                    self.reinsert_polygon_in_rtree(neighbor_id, &merged);

                    maybe_absorber_id = Some(neighbor_id);
                }
            };
        }

        maybe_absorber_id.unwrap_or_else(|| {
            let new_id = PolygonId::new(self.polygons.push(polygon));
            let new_polygon = self.polygons.get(&new_id.index()).unwrap().clone();
            let new_bbox = Self::rectangle_from_polygon(&new_polygon);

            self.rtree
                .insert(GeomWithData::new(new_bbox.clone(), new_id), ());

            new_id
        })
    }

    pub fn subtract(&mut self, polygon: P) -> Vec<PolygonId> {
        let rectangle = Self::rectangle_from_polygon(&polygon);
        let neighbor_ids: Vec<PolygonId> = self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rectangle.envelope())
            .map(|neighbor| neighbor.data)
            .collect();

        let mut piece_ids = Vec::new();

        for neighbor_id in neighbor_ids {
            let neighbor = self.polygons.get(&neighbor_id.index()).unwrap();
            let difference = P::difference(neighbor.clone(), polygon.clone());

            if difference.is_empty() {
                self.remove_polygon_from_rtree(neighbor_id);
                self.polygons.remove(&neighbor_id.index());
            } else {
                // Difference can result in multiple disjoint polygons. We reuse
                // the id of the neighbor for the first piece. The remaining
                // parts are pushed.

                self.polygons
                    .set(neighbor_id.index(), difference[0].clone());
                self.reinsert_polygon_in_rtree(neighbor_id, &difference[0]);
                piece_ids.push(neighbor_id);

                for piece in difference.into_iter().skip(1) {
                    let piece_bbox = Self::rectangle_from_polygon(&piece);
                    let piece_id = PolygonId::new(self.polygons.push(piece));

                    self.rtree
                        .insert(GeomWithData::new(piece_bbox, piece_id), ());
                    piece_ids.push(piece_id);
                }
            }
        }

        piece_ids
    }

    fn remove_polygon_from_rtree(&mut self, polygon_id: PolygonId) {
        let geom_with_data = self
            .rtree
            .as_ref()
            .iter()
            .find(|g| g.data == polygon_id)
            .cloned();
        if let Some(geom_with_data) = geom_with_data {
            self.rtree.remove(&geom_with_data);
        }
    }

    fn reinsert_polygon_in_rtree(&mut self, polygon_id: PolygonId, polygon: &P) {
        self.remove_polygon_from_rtree(polygon_id);
        let rect = Self::rectangle_from_polygon(polygon);
        self.rtree.insert(GeomWithData::new(rect, polygon_id), ());
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
