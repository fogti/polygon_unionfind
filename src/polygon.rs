// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::{iter::Map, slice::Iter};

pub trait Rings<K> {
    type RingIter<'a>: Iterator<Item = &'a [[K; 2]]>
    where
        Self: 'a,
        K: 'a;

    fn exterior(&self) -> &[[K; 2]];
    fn interiors<'a>(&'a self) -> Self::RingIter<'a>;
}

#[derive(Clone, Debug)]
pub struct Polygon<K> {
    /// Polygon boundary.
    pub exterior: Vec<[K; 2]>,
    /// Polygon interior rings.
    pub interiors: Vec<Vec<[K; 2]>>,
}

#[derive(Clone, Debug)]
pub struct PolygonWithWeight<K, W = ()> {
    /// Polygon boundary.
    pub exterior: Vec<[K; 2]>,
    /// Polygon interior rings.
    pub interiors: Vec<Vec<[K; 2]>>,
    /// User-defined payload for this polygon.
    pub weight: W,
}

impl<K> Rings<K> for Polygon<K> {
    type RingIter<'a>
        = Map<Iter<'a, Vec<[K; 2]>>, fn(&'a Vec<[K; 2]>) -> &'a [[K; 2]]>
    where
        K: 'a;

    fn exterior(&self) -> &[[K; 2]] {
        self.exterior.as_slice()
    }

    fn interiors<'a>(&'a self) -> Self::RingIter<'a> {
        self.interiors.iter().map(Vec::as_slice)
    }
}

impl<K, W> Rings<K> for PolygonWithWeight<K, W> {
    type RingIter<'a>
        = Map<Iter<'a, Vec<[K; 2]>>, fn(&'a Vec<[K; 2]>) -> &'a [[K; 2]]>
    where
        K: 'a,
        W: 'a;

    fn exterior(&self) -> &[[K; 2]] {
        self.exterior.as_slice()
    }

    fn interiors<'a>(&'a self) -> Self::RingIter<'a> {
        self.interiors.iter().map(Vec::as_slice)
    }
}
