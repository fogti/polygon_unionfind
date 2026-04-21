// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;

use maplike::{Container, Get};

use crate::{Clip, Exclude, Include, Inflate, PolygonId};

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

impl<K: Clone, P: Clone + Inflate<K>, S: Include<P, Output = PolygonId>> Include<P>
    for Inflated<S, K>
{
    type Output = PolygonId;

    fn include(&mut self, polygon: P) -> PolygonId {
        self.inflatee.include(polygon.inflate(self.offset.clone()))
    }
}

impl<K: Clone, P: Clone + Inflate<K>, S: Exclude<P, Output = Vec<PolygonId>>> Exclude<P>
    for Inflated<S, K>
{
    type Output = Vec<PolygonId>;

    fn exclude(&mut self, polygon: P) -> Vec<PolygonId> {
        self.inflatee.exclude(polygon.inflate(self.offset.clone()))
    }
}

impl<I: Container, K> Container for Inflated<I, K> {
    type Key = I::Key;
    type Value = I::Value;
}

impl<Key, I: Get<Key>, K> Get<Key> for Inflated<I, K> {
    fn get(&self, key: &Key) -> Option<&Self::Value> {
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
    M: Exclude<P, Output = Vec<PolygonId>>,
    S: Include<P, Output = PolygonId> + Get<PolygonId, Value = P>,
> Include<P> for Negated<M, S>
{
    type Output = Vec<PolygonId>;

    fn include(&mut self, polygon: P) -> Vec<PolygonId> {
        let id = self.subtrahend.include(polygon.clone());

        self.minuend
            .exclude(self.subtrahend.get(&id).unwrap().clone())
    }
}

impl<
    P: Clone,
    M: Exclude<P, Output = Vec<PolygonId>>,
    S: Include<P, Output = PolygonId> + Get<PolygonId, Value = P>,
> Exclude<P> for Negated<M, S>
{
    type Output = Vec<PolygonId>;

    fn exclude(&mut self, polygon: P) -> Vec<PolygonId> {
        let id = self.subtrahend.include(polygon.clone());

        self.minuend
            .exclude(self.subtrahend.get(&id).unwrap().clone())
    }
}

impl<M: Container, S> Container for Negated<M, S> {
    type Key = M::Key;
    type Value = M::Value;
}

impl<Key, M: Get<Key>, S> Get<Key> for Negated<M, S> {
    fn get(&self, key: &Key) -> Option<&Self::Value> {
        self.minuend.get(key)
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

impl<P: Clone, S: Include<P>> Include<P> for Paralleled<S> {
    type Output = S::Output;

    fn include(&mut self, polygon: P) -> S::Output {
        for parallel in &mut self.parallels {
            parallel.include(polygon.clone());
        }

        self.primary.include(polygon)
    }
}

impl<P: Clone, S: Exclude<P>> Exclude<P> for Paralleled<S> {
    type Output = S::Output;

    fn exclude(&mut self, polygon: P) -> S::Output {
        for parallel in &mut self.parallels {
            parallel.exclude(polygon.clone());
        }

        self.primary.exclude(polygon)
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
