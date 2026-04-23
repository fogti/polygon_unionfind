// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;

use maplike::{Container, Get};

use crate::{Add, Clip, Inflate, PolygonId, Sub};

#[derive(Clone, Debug)]
pub struct Inflated<I, K> {
    inflatee: I,
    offset: K,
}

impl<I, K> Inflated<I, K> {
    #[inline]
    pub fn inflatee(&self) -> &I {
        &self.inflatee
    }

    #[inline]
    pub fn offset(&self) -> &K {
        &self.offset
    }
}

impl<I: Default, K> Inflated<I, K> {
    #[inline]
    pub fn new(offset: K) -> Self {
        Self {
            inflatee: I::default(),
            offset,
        }
    }
}

impl<K: Clone, P: Clone + Inflate<K>, S: Add<P, Output = PolygonId>> Add<P>
    for Inflated<S, K>
{
    type Output = PolygonId;

    fn add(&mut self, polygon: P) -> PolygonId {
        self.inflatee.add(polygon.inflate(self.offset.clone()))
    }
}

impl<K: Clone, P: Clone + Inflate<K>, S: Sub<P>> Sub<P>
    for Inflated<S, K>
{
    type Output = S::Output;

    fn sub(&mut self, polygon: P) -> S::Output {
        self.inflatee.sub(polygon.inflate(self.offset.clone()))
    }
}

impl<I: Container, K> Container for Inflated<I, K> {
    type Key = I::Key;
    type Value = I::Value;
}

impl<I: Get<K2>, K, K2> Get<K2> for Inflated<I, K> {
    #[inline]
    fn get(&self, key: &K2) -> Option<&Self::Value> {
        self.inflatee.get(key)
    }
}

#[derive(Clone, Debug)]
pub struct Negated<M, S> {
    minuend: M,
    subtrahend: S,
}

impl<M, S> Negated<M, S> {
    #[inline]
    pub fn minuend(&self) -> &M {
        &self.minuend
    }

    #[inline]
    pub fn subtrahend(&self) -> &S {
        &self.subtrahend
    }

    #[inline]
    pub fn new(minuend: M, subtrahend: S) -> Self {
        Self {
            minuend,
            subtrahend,
        }
    }
}

impl<M: Default, S: Default> Default for Negated<M, S> {
    #[inline]
    fn default() -> Self {
        Self {
            minuend: M::default(),
            subtrahend: S::default(),
        }
    }
}

impl<
    P: Clone,
    M: Sub<P, Output = (Vec<PolygonId>, Vec<P>)>,
    S: Add<P, Output = PolygonId> + Get<PolygonId, Value = P>,
> Add<P> for Negated<M, S>
{
    type Output = (Vec<PolygonId>, Vec<P>);

    fn add(&mut self, polygon: P) -> (Vec<PolygonId>, Vec<P>) {
        let id = self.subtrahend.add(polygon.clone());

        self.minuend
            .sub(self.subtrahend.get(&id).unwrap().clone())
    }
}

#[derive(Clone, Debug)]
pub struct Paralleled<S> {
    primary: S,
    parallels: Vec<S>,
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn primary(&self) -> &S {
        &self.primary
    }

    #[inline]
    pub fn parallels(&self) -> &Vec<S> {
        &self.parallels
    }
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn new(primary: S, parallels: Vec<S>) -> Self {
        Self { primary, parallels }
    }
}

impl<P: Clone, S: Add<P>> Add<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn add(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let mut parallel_outputs = Vec::with_capacity(self.parallels.len());

        for parallel in &mut self.parallels {
            parallel_outputs.push(parallel.add(polygon.clone()));
        }

        let primary_output = self.primary.add(polygon);
        (primary_output, parallel_outputs)
    }
}

impl<P: Clone, S: Sub<P>> Sub<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn sub(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let mut parallel_outputs = Vec::with_capacity(self.parallels.len());

        for parallel in &mut self.parallels {
            parallel_outputs.push(parallel.sub(polygon.clone()));
        }

        let primary_output = self.primary.sub(polygon);
        (primary_output, parallel_outputs)
    }
}

impl<P: Clone, S: Clip<P>> Clip<P> for Paralleled<S> {
    type Output = S::Output;

    fn clip(&mut self, polygon: P) -> S::Output {
        for parallel in &mut self.parallels {
            parallel.clip(polygon.clone());
        }

        self.primary.clip(polygon)
    }
}
