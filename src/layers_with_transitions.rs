// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
#[cfg(feature = "undoredo")]
use core::marker::PhantomData;

use rstar::RTreeNum;
use rstar::RTreeObject;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::bool_ops::{Difference, Intersect, Union};
use crate::{
    Add, Inflate, Inflated, Negated, Paralleled, PolygonId, PolygonSet, PolygonUnionFind,
    PolygonWithData, Rings, Sub, polygon::rectangle_from_polygon,
};
#[cfg(feature = "undoredo")]
use crate::{RecordingNegated, RecordingPolygonSet};

pub type LayerWithParallel<K, P> =
    Paralleled<Paralleled<Negated<PolygonSet<K, P>, Inflated<PolygonUnionFind<K, P>, K>>>>;
pub type TransitionLayer<K, P> = Paralleled<PolygonSet<K, P>>;

#[cfg(feature = "undoredo")]
/// Layer-with-parallel based on recording containers for delta-based Undo/Redo.
pub type RecordingLayerWithParallel<K, P = PolygonWithData<K>> =
    Paralleled<Paralleled<RecordingNegated<K, P>>>;

#[cfg(feature = "undoredo")]
/// Transition layer based on recording containers for delta-based Undo/Redo.
pub type RecordingTransitionLayer<K, P = PolygonWithData<K>> = Paralleled<RecordingPolygonSet<K, P>>;

#[derive(Clone)]
pub struct LayersWithTransitions<
    K: RTreeNum,
    P = PolygonWithData<K>,
    L = LayerWithParallel<K, P>,
    T = TransitionLayer<K, P>,
> {
    pub layers: Vec<L>,
    pub transitions: Vec<T>,
    scalar_and_polygon_marker: PhantomData<(K, P)>,
}

impl<K: RTreeNum, P, L, T> LayersWithTransitions<K, P, L, T> {
    #[inline]
    pub fn new(layers: Vec<L>, transitions: Vec<T>) -> Self {
        assert!(
            transitions.len() == layers.len().saturating_sub(1),
            "transitions must have exactly layers.len() - 1 entries"
        );
        Self {
            layers,
            transitions,
            scalar_and_polygon_marker: PhantomData,
        }
    }

    #[inline]
    pub fn layers(&self) -> &[L] {
        &self.layers
    }

    #[inline]
    pub fn transitions(&self) -> &[T] {
        &self.transitions
    }

    #[inline]
    pub fn layer(&self, index: usize) -> Option<&L> {
        self.layers.get(index)
    }

    #[inline]
    pub fn transition(&self, index: usize) -> Option<&T> {
        self.transitions.get(index)
    }
}

impl<K, P> LayersWithTransitions<K, P, LayerWithParallel<K, P>, TransitionLayer<K, P>>
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

#[cfg(feature = "undoredo")]
/// `LayersWithTransitions`-equivalent using recording container combinators.
pub type RecordingLayersWithTransitions<K, P = PolygonWithData<K>> = LayersWithTransitions<
    K,
    P,
    RecordingLayerWithParallel<K, P>,
    RecordingTransitionLayer<K, P>,
>;

#[cfg(feature = "undoredo")]
/// Half-delta of `LayersWithTransitions`.
pub type LayersWithTransitionsHalfDelta<K, P, LE, TE> = LayersWithTransitions<K, P, LE, TE>;

#[cfg(feature = "undoredo")]
/// Delta of `LayersWithTransitions` for delta-based Undo/Redo.
pub type LayersWithTransitionsDelta<K, P, LE, TE> =
    Delta<LayersWithTransitionsHalfDelta<K, P, LE, TE>>;

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P,
    LE: Clone,
    L: Clone + ApplyDelta<LE>,
    TE: Clone,
    T: Clone + ApplyDelta<TE>,
> ApplyDelta<LayersWithTransitionsHalfDelta<K, P, LE, TE>> for LayersWithTransitions<K, P, L, T>
{
    fn apply_delta(&mut self, delta: LayersWithTransitionsDelta<K, P, LE, TE>) {
        let (removed, inserted) = delta.dissolve();

        for (layer, (removed_layer, inserted_layer)) in self.layers.iter_mut().zip(
            removed
                .layers
                .into_iter()
                .zip(inserted.layers.into_iter()),
        ) {
            layer.apply_delta(Delta::with_removed_inserted(removed_layer, inserted_layer));
        }

        for (transition, (removed_transition, inserted_transition)) in self
            .transitions
            .iter_mut()
            .zip(removed.transitions.into_iter().zip(inserted.transitions.into_iter()))
        {
            transition.apply_delta(Delta::with_removed_inserted(
                removed_transition,
                inserted_transition,
            ));
        }
    }
}

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P,
    LE: Clone,
    L: FlushDelta<LE>,
    TE: Clone,
    T: FlushDelta<TE>,
> FlushDelta<LayersWithTransitionsHalfDelta<K, P, LE, TE>> for LayersWithTransitions<K, P, L, T>
{
    fn flush_delta(&mut self) -> LayersWithTransitionsDelta<K, P, LE, TE> {
        let mut removed_layers = Vec::with_capacity(self.layers.len());
        let mut inserted_layers = Vec::with_capacity(self.layers.len());
        let mut removed_transitions = Vec::with_capacity(self.transitions.len());
        let mut inserted_transitions = Vec::with_capacity(self.transitions.len());

        for layer in &mut self.layers {
            let (removed_layer, inserted_layer) = layer.flush_delta().dissolve();
            removed_layers.push(removed_layer);
            inserted_layers.push(inserted_layer);
        }

        for transition in &mut self.transitions {
            let (removed_transition, inserted_transition) = transition.flush_delta().dissolve();
            removed_transitions.push(removed_transition);
            inserted_transitions.push(inserted_transition);
        }

        Delta::with_removed_inserted(
            LayersWithTransitions {
                layers: removed_layers,
                transitions: removed_transitions,
                scalar_and_polygon_marker: PhantomData,
            },
            LayersWithTransitions {
                layers: inserted_layers,
                transitions: inserted_transitions,
                scalar_and_polygon_marker: PhantomData,
            },
        )
    }
}
