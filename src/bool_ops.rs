// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::{convert::TryFrom, slice};

use i_overlay::{
    core::{fill_rule::FillRule, overlay::Overlay, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    i_float::int::point::IntPoint,
    i_shape::int::shape::IntShape,
};

use crate::{Polygon, PolygonWithData, Rings};

pub trait Union<T> {
    fn union(subj: T, clip: T) -> Option<T>;
}

pub trait Intersection<T> {
    fn intersect(subj: T, clip: T) -> Vec<T>;
}

pub trait Difference<T> {
    fn difference(subj: T, clip: T) -> Vec<T>;
}

fn rings_to_shape<K, P: Rings<K>>(polygon: &P) -> Vec<Vec<[K; 2]>>
where
    K: Clone,
{
    let mut shape = Vec::new();
    shape.push(polygon.exterior().to_vec());
    shape.extend(polygon.interiors().map(<[_]>::to_vec));
    shape
}

fn first_merged_shape<K>(union: Vec<Vec<Vec<[K; 2]>>>) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)> {
    if union.len() >= 2 {
        return None;
    }

    let mut merged_shape = union.into_iter().next()?;
    if merged_shape.is_empty() {
        return None;
    }

    let exterior = merged_shape.remove(0);
    Some((exterior, merged_shape))
}

fn all_merged_shapes<K>(shapes: Vec<Vec<Vec<[K; 2]>>>) -> Vec<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)> {
    shapes
        .into_iter()
        .filter_map(|mut shape| {
            if shape.is_empty() {
                return None;
            }
            let exterior = shape.remove(0);
            Some((exterior, shape))
        })
        .collect()
}

fn overlay_int_polygons<K, A: Rings<K>, B: Rings<K>>(
    a: &A,
    b: &B,
    overlay_rule: OverlayRule,
) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)>
where
    K: Copy + TryFrom<i32>,
    i32: TryFrom<K>,
{
    let to_int_ring = |ring: &[[K; 2]]| -> Option<Vec<IntPoint>> {
        ring.iter()
            .map(|vertex| {
                Some(IntPoint::new(
                    i32::try_from(vertex[0]).ok()?,
                    i32::try_from(vertex[1]).ok()?,
                ))
            })
            .collect()
    };

    let mut subj_shape: IntShape = Vec::new();
    subj_shape.push(to_int_ring(a.exterior())?);
    for ring in a.interiors() {
        subj_shape.push(to_int_ring(ring)?);
    }

    let mut clip_shape: IntShape = Vec::new();
    clip_shape.push(to_int_ring(b.exterior())?);
    for ring in b.interiors() {
        clip_shape.push(to_int_ring(ring)?);
    }

    let union = Overlay::with_shapes(slice::from_ref(&subj_shape), slice::from_ref(&clip_shape))
        .overlay(overlay_rule, FillRule::EvenOdd);

    if union.len() >= 2 {
        return None;
    }

    let merged_shape = union.first()?;
    if merged_shape.is_empty() {
        return None;
    }

    let from_int_ring = |ring: &[IntPoint]| -> Option<Vec<[K; 2]>> {
        ring.iter()
            .map(|vertex| Some([K::try_from(vertex.x).ok()?, K::try_from(vertex.y).ok()?]))
            .collect()
    };

    let exterior = from_int_ring(&merged_shape[0])?;
    let interiors = merged_shape[1..]
        .iter()
        .map(|ring| from_int_ring(ring))
        .collect::<Option<Vec<_>>>()?;

    Some((exterior, interiors))
}

fn overlay_int_polygons_many<K, A: Rings<K>, B: Rings<K>>(
    a: &A,
    b: &B,
    overlay_rule: OverlayRule,
) -> Option<Vec<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)>>
where
    K: Copy + TryFrom<i32>,
    i32: TryFrom<K>,
{
    let to_int_ring = |ring: &[[K; 2]]| -> Option<Vec<IntPoint>> {
        ring.iter()
            .map(|vertex| {
                Some(IntPoint::new(
                    i32::try_from(vertex[0]).ok()?,
                    i32::try_from(vertex[1]).ok()?,
                ))
            })
            .collect()
    };

    let mut subj_shape: IntShape = Vec::new();
    subj_shape.push(to_int_ring(a.exterior())?);
    for ring in a.interiors() {
        subj_shape.push(to_int_ring(ring)?);
    }

    let mut clip_shape: IntShape = Vec::new();
    clip_shape.push(to_int_ring(b.exterior())?);
    for ring in b.interiors() {
        clip_shape.push(to_int_ring(ring)?);
    }

    let shapes = Overlay::with_shapes(slice::from_ref(&subj_shape), slice::from_ref(&clip_shape))
        .overlay(overlay_rule, FillRule::EvenOdd);

    let from_int_ring = |ring: &[IntPoint]| -> Option<Vec<[K; 2]>> {
        ring.iter()
            .map(|vertex| Some([K::try_from(vertex.x).ok()?, K::try_from(vertex.y).ok()?]))
            .collect()
    };

    shapes
        .into_iter()
        .map(|shape| {
            if shape.is_empty() {
                return None;
            }
            let exterior = from_int_ring(&shape[0])?;
            let interiors = shape[1..]
                .iter()
                .map(|ring| from_int_ring(ring))
                .collect::<Option<Vec<_>>>()?;
            Some((exterior, interiors))
        })
        .collect()
}

macro_rules! impl_union_float {
    ($k:ty) => {
        impl Union<Polygon<$k>> for Polygon<$k> {
            fn union(a: Polygon<$k>, b: Polygon<$k>) -> Option<Polygon<$k>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result = subj_shape.overlay(&clip_shape, OverlayRule::Union, FillRule::EvenOdd);
                let (exterior, interiors) = first_merged_shape(result)?;
                Some(Polygon {
                    exterior,
                    interiors,
                })
            }
        }

        impl<W> Union<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn union(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Option<PolygonWithData<$k, W>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result = subj_shape.overlay(&clip_shape, OverlayRule::Union, FillRule::EvenOdd);
                let (exterior, interiors) = first_merged_shape(result)?;
                Some(PolygonWithData {
                    exterior,
                    interiors,
                    data: a.data,
                })
            }
        }
    };
}

macro_rules! impl_union_int {
    ($k:ty) => {
        impl Union<Polygon<$k>> for Polygon<$k> {
            fn union(a: Polygon<$k>, b: Polygon<$k>) -> Option<Polygon<$k>> {
                let (exterior, interiors) = overlay_int_polygons(&a, &b, OverlayRule::Union)?;
                Some(Polygon {
                    exterior,
                    interiors,
                })
            }
        }

        impl<W> Union<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn union(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Option<PolygonWithData<$k, W>> {
                let (exterior, interiors) = overlay_int_polygons(&a, &b, OverlayRule::Union)?;
                Some(PolygonWithData {
                    exterior,
                    interiors,
                    data: a.data,
                })
            }
        }
    };
}

macro_rules! impl_intersect_float {
    ($k:ty) => {
        impl Intersection<Polygon<$k>> for Polygon<$k> {
            fn intersect(a: Polygon<$k>, b: Polygon<$k>) -> Vec<Polygon<$k>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result =
                    subj_shape.overlay(&clip_shape, OverlayRule::Intersect, FillRule::EvenOdd);
                all_merged_shapes(result)
                    .into_iter()
                    .map(|(exterior, interiors)| Polygon {
                        exterior,
                        interiors,
                    })
                    .collect()
            }
        }

        impl<W: Clone> Intersection<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn intersect(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Vec<PolygonWithData<$k, W>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result =
                    subj_shape.overlay(&clip_shape, OverlayRule::Intersect, FillRule::EvenOdd);
                all_merged_shapes(result)
                    .into_iter()
                    .map(|(exterior, interiors)| PolygonWithData {
                        exterior,
                        interiors,
                        data: a.data.clone(),
                    })
                    .collect()
            }
        }
    };
}

macro_rules! impl_intersect_int {
    ($k:ty) => {
        impl Intersection<Polygon<$k>> for Polygon<$k> {
            fn intersect(a: Polygon<$k>, b: Polygon<$k>) -> Vec<Polygon<$k>> {
                overlay_int_polygons_many(&a, &b, OverlayRule::Intersect)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(exterior, interiors)| Polygon {
                        exterior,
                        interiors,
                    })
                    .collect()
            }
        }

        impl<W: Clone> Intersection<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn intersect(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Vec<PolygonWithData<$k, W>> {
                overlay_int_polygons_many(&a, &b, OverlayRule::Intersect)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(exterior, interiors)| PolygonWithData {
                        exterior,
                        interiors,
                        data: a.data.clone(),
                    })
                    .collect()
            }
        }
    };
}

macro_rules! impl_difference_float {
    ($k:ty) => {
        impl Difference<Polygon<$k>> for Polygon<$k> {
            fn difference(a: Polygon<$k>, b: Polygon<$k>) -> Vec<Polygon<$k>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result =
                    subj_shape.overlay(&clip_shape, OverlayRule::Difference, FillRule::EvenOdd);
                all_merged_shapes(result)
                    .into_iter()
                    .map(|(exterior, interiors)| Polygon {
                        exterior,
                        interiors,
                    })
                    .collect()
            }
        }

        impl<W: Clone> Difference<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn difference(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Vec<PolygonWithData<$k, W>> {
                let subj_shape = rings_to_shape(&a);
                let clip_shape = rings_to_shape(&b);
                let result =
                    subj_shape.overlay(&clip_shape, OverlayRule::Difference, FillRule::EvenOdd);
                all_merged_shapes(result)
                    .into_iter()
                    .map(|(exterior, interiors)| PolygonWithData {
                        exterior,
                        interiors,
                        data: a.data.clone(),
                    })
                    .collect()
            }
        }
    };
}

macro_rules! impl_difference_int {
    ($k:ty) => {
        impl Difference<Polygon<$k>> for Polygon<$k> {
            fn difference(a: Polygon<$k>, b: Polygon<$k>) -> Vec<Polygon<$k>> {
                overlay_int_polygons_many(&a, &b, OverlayRule::Difference)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(exterior, interiors)| Polygon {
                        exterior,
                        interiors,
                    })
                    .collect()
            }
        }

        impl<W: Clone> Difference<PolygonWithData<$k, W>> for PolygonWithData<$k, W> {
            fn difference(
                a: PolygonWithData<$k, W>,
                b: PolygonWithData<$k, W>,
            ) -> Vec<PolygonWithData<$k, W>> {
                overlay_int_polygons_many(&a, &b, OverlayRule::Difference)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(exterior, interiors)| PolygonWithData {
                        exterior,
                        interiors,
                        data: a.data.clone(),
                    })
                    .collect()
            }
        }
    };
}

// We don't provide implementations for `i64` because `i_overlay` does not
// support these -- not a single value beyond `i32` range is supported. And in
// fact, `i_overlay`'s author treats `|coord| < 2^29` as a safe bound, which
// narrower than even the `i32` range (`-2^31` to `2^31-1`).

impl_union_float!(f32);
impl_union_float!(f64);
impl_union_int!(i8);
impl_union_int!(i16);
impl_union_int!(i32);
//impl_union_int!(i64);

impl_intersect_float!(f32);
impl_intersect_float!(f64);
impl_intersect_int!(i8);
impl_intersect_int!(i16);
impl_intersect_int!(i32);
//impl_intersect_int!(i64);

impl_difference_float!(f32);
impl_difference_float!(f64);
impl_difference_int!(i8);
impl_difference_int!(i16);
impl_difference_int!(i32);
//impl_difference_int!(i64);
