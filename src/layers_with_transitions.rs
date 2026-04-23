// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use rstar::RTreeNum;
use rstar::RTreeObject;

use crate::bool_ops::{Difference, Intersect, Union};
use crate::{
    Add, Inflate, Inflated, Negated, Paralleled, PolygonId, PolygonSet, Sub,
    PolygonUnionFind, PolygonWithData, Rings, polygon::rectangle_from_polygon,
};

pub type LayerWithParallel<K, P> =
    Paralleled<Paralleled<Negated<PolygonSet<K, P>, Inflated<PolygonUnionFind<K, P>, K>>>>;
pub type TransitionLayer<K, P> = Paralleled<PolygonSet<K, P>>;

#[derive(Clone)]
pub struct LayersWithTransitions<K: RTreeNum, P = PolygonWithData<K>> {
    pub layers: Vec<LayerWithParallel<K, P>>,
    pub transitions: Vec<TransitionLayer<K, P>>,
}

impl<K: RTreeNum, P> LayersWithTransitions<K, P> {
    #[inline]
    pub fn new(
        layers: Vec<LayerWithParallel<K, P>>,
        transitions: Vec<TransitionLayer<K, P>>,
    ) -> Self {
        assert!(
            transitions.len() == layers.len().saturating_sub(1),
            "transitions must have exactly layers.len() - 1 entries"
        );
        Self {
            layers,
            transitions,
        }
    }

    #[inline]
    pub fn layers(&self) -> &[LayerWithParallel<K, P>] {
        &self.layers
    }

    #[inline]
    pub fn transitions(&self) -> &[TransitionLayer<K, P>] {
        &self.transitions
    }

    #[inline]
    pub fn layer(&self, index: usize) -> Option<&LayerWithParallel<K, P>> {
        self.layers.get(index)
    }

    #[inline]
    pub fn transition(&self, index: usize) -> Option<&TransitionLayer<K, P>> {
        self.transitions.get(index)
    }
}

impl<K, P> LayersWithTransitions<K, P>
where
    K: RTreeNum + Ord,
    P: Clone + Rings<K> + Inflate<K> + Union<P> + Difference<P> + Intersect<P>,
{
    pub fn add_into_layer(&mut self, layer_index: usize, polygon: P) {
        if layer_index >= self.layers.len() {
            return;
        }

        let (clipping_polygons, removed_polygons) = {
            let layer = self.layers.get_mut(layer_index).unwrap();
            let (inner_outputs, _outer_parallel_outputs) = layer.add(polygon);
            let (inner_primary_output, inner_parallel_outputs) = inner_outputs;
            let (parallel_ids, removed_polygons) = inner_parallel_outputs
                .first()
                .cloned()
                .unwrap_or(inner_primary_output);

            // We hard-code the last parallel to be the polygon-set that clips
            // the transition layer.
            let Some(clipping_source) = layer.primary().parallels().last().map(|s| s.minuend())
            else {
                return;
            };

            (
                Self::polygons_under_ids(&parallel_ids, clipping_source),
                removed_polygons,
            )
        };

        // Transition i corresponds to the window (i, i+1). Hence, a write to
        // layer k affects transitions k-1 and k.
        if layer_index > 0 {
            self.exclude_removed(layer_index - 1, &removed_polygons);
            self.clip_transpolygon(layer_index - 1, clipping_polygons.clone());
        }
        if layer_index < self.transitions.len() {
            self.exclude_removed(layer_index, &removed_polygons);
            self.clip_transpolygon(layer_index, clipping_polygons);
        }
    }

    fn polygons_under_ids(ids: &[PolygonId], source: &PolygonSet<K, P>) -> Vec<P> {
        ids.iter()
            .filter_map(|id| source.polygons().get(id.index()).cloned())
            .collect()
    }

    fn exclude_removed(&mut self, transition_index: usize, removed_polygons: &[P]) {
        if removed_polygons.is_empty() {
            return;
        }

        let transition = &mut self.transitions[transition_index];

        for removed in removed_polygons {
            let _ = transition.sub(removed.clone());
        }
    }

    fn clip_transpolygon(&mut self, transpolygon_index: usize, clipping_polygons: Vec<P>) {
        let transition = &mut self.transitions[transpolygon_index];

        if clipping_polygons.is_empty() {
            // If there is nothing to clip, the transition is unaffected.
            return;
        }

        let mut located_transids = BTreeSet::new();

        for clipping_polygon in &clipping_polygons {
            let clip_bbox = rectangle_from_polygon(clipping_polygon);

            for hit in transition
                .primary()
                .rtree()
                .as_ref()
                .locate_in_envelope_intersecting(&clip_bbox.envelope())
            {
                located_transids.insert(hit.data);
            }
        }

        let located_transpolygons: Vec<P> = located_transids
            .into_iter()
            .filter_map(|id| transition.primary().polygons().get(id.index()).cloned())
            .collect();

        for transpolygon in located_transpolygons {
            let mut clipped_union = PolygonSet::<K, P>::new();
            let mut has_intersection = false;

            for clipping_polygon in &clipping_polygons {
                for piece in P::intersect(transpolygon.clone(), clipping_polygon.clone()) {
                    has_intersection = true;
                    clipped_union.add(piece);
                }
            }

            if !has_intersection {
                // If transpolygon is not intersected, skip it. Otherwise, it
                // would get removed, which would be incorrect.
                continue;
            }

            transition.sub(transpolygon);

            for (_idx, piece) in clipped_union.polygons().iter() {
                transition.add(piece.clone());
            }
        }
    }
}
