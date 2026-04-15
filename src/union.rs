// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::{convert::TryFrom, slice};

pub trait Union<T> {
    fn union(a: T, b: T) -> Option<T>;
}

use i_overlay::{
    core::{fill_rule::FillRule, overlay::Overlay, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    i_float::int::point::IntPoint,
    i_shape::int::shape::IntShape,
};

use crate::Polygon;

impl<W> Union<Polygon<f32, W>> for Polygon<f32, W> {
    fn union(a: Polygon<f32, W>, b: Polygon<f32, W>) -> Option<Polygon<f32, W>> {
        let mut subj_shape = Vec::with_capacity(1 + a.interiors.len());
        subj_shape.push(a.exterior.clone());
        subj_shape.extend(a.interiors.iter().cloned());

        let mut clip_shape = Vec::with_capacity(1 + b.interiors.len());
        clip_shape.push(b.exterior);
        clip_shape.extend(b.interiors);

        let union = subj_shape.overlay(&clip_shape, OverlayRule::Union, FillRule::EvenOdd);

        if union.len() >= 2 {
            return None;
        }

        let merged_shape = union.first()?;
        if merged_shape.is_empty() {
            return None;
        }

        Some(Polygon {
            exterior: merged_shape[0].clone(),
            interiors: merged_shape[1..].to_vec(),
            weight: a.weight,
        })
    }
}

impl<W> Union<Polygon<f64, W>> for Polygon<f64, W> {
    fn union(a: Polygon<f64, W>, b: Polygon<f64, W>) -> Option<Polygon<f64, W>> {
        let mut subj_shape = Vec::with_capacity(1 + a.interiors.len());
        subj_shape.push(a.exterior.clone());
        subj_shape.extend(a.interiors.iter().cloned());

        let mut clip_shape = Vec::with_capacity(1 + b.interiors.len());
        clip_shape.push(b.exterior);
        clip_shape.extend(b.interiors);

        let union = subj_shape.overlay(&clip_shape, OverlayRule::Union, FillRule::EvenOdd);

        if union.len() >= 2 {
            return None;
        }

        let merged_shape = union.first()?;
        if merged_shape.is_empty() {
            return None;
        }

        Some(Polygon {
            exterior: merged_shape[0].clone(),
            interiors: merged_shape[1..].to_vec(),
            weight: a.weight,
        })
    }
}

impl<W> Union<Polygon<i32, W>> for Polygon<i32, W> {
    fn union(a: Polygon<i32, W>, b: Polygon<i32, W>) -> Option<Polygon<i32, W>> {
        let mut subj_shape: IntShape = Vec::with_capacity(1 + a.interiors.len());
        subj_shape.push(
            a.exterior
                .iter()
                .map(|v| IntPoint::new(v[0], v[1]))
                .collect(),
        );
        subj_shape.extend(
            a.interiors
                .iter()
                .map(|ring| ring.iter().map(|v| IntPoint::new(v[0], v[1])).collect()),
        );

        let mut clip_shape: IntShape = Vec::with_capacity(1 + b.interiors.len());
        clip_shape.push(b.exterior.into_iter().map(|v| IntPoint::new(v[0], v[1])).collect());
        clip_shape.extend(
            b.interiors
                .into_iter()
                .map(|ring| ring.into_iter().map(|v| IntPoint::new(v[0], v[1])).collect()),
        );

        let union = Overlay::with_shapes(slice::from_ref(&subj_shape), slice::from_ref(&clip_shape))
            .overlay(OverlayRule::Union, FillRule::EvenOdd);

        if union.len() >= 2 {
            return None;
        }

        let merged_shape = union.first()?;
        if merged_shape.is_empty() {
            return None;
        }

        Some(Polygon {
            exterior: merged_shape[0].iter().map(|v| [v.x, v.y]).collect(),
            interiors: merged_shape[1..]
                .iter()
                .map(|ring| ring.iter().map(|v| [v.x, v.y]).collect())
                .collect(),
            weight: a.weight,
        })
    }
}

impl<W> Union<Polygon<i64, W>> for Polygon<i64, W> {
    fn union(a: Polygon<i64, W>, b: Polygon<i64, W>) -> Option<Polygon<i64, W>> {
        let to_ring = |ring: Vec<[i64; 2]>| -> Option<Vec<IntPoint>> {
            ring.into_iter()
                .map(|v| Some(IntPoint::new(i32::try_from(v[0]).ok()?, i32::try_from(v[1]).ok()?)))
                .collect()
        };

        let mut subj_shape: IntShape = Vec::with_capacity(1 + a.interiors.len());
        subj_shape.push(to_ring(a.exterior)?);
        for ring in a.interiors {
            subj_shape.push(to_ring(ring)?);
        }

        let mut clip_shape: IntShape = Vec::with_capacity(1 + b.interiors.len());
        clip_shape.push(to_ring(b.exterior)?);
        for ring in b.interiors {
            clip_shape.push(to_ring(ring)?);
        }

        let union = Overlay::with_shapes(slice::from_ref(&subj_shape), slice::from_ref(&clip_shape))
            .overlay(OverlayRule::Union, FillRule::EvenOdd);

        if union.len() >= 2 {
            return None;
        }

        let merged_shape = union.first()?;
        if merged_shape.is_empty() {
            return None;
        }

        Some(Polygon {
            exterior: merged_shape[0]
                .iter()
                .map(|v| [i64::from(v.x), i64::from(v.y)])
                .collect(),
            interiors: merged_shape[1..]
                .iter()
                .map(|ring| {
                    ring.iter()
                        .map(|v| [i64::from(v.x), i64::from(v.y)])
                        .collect()
                })
                .collect(),
            weight: a.weight,
        })
    }
}
