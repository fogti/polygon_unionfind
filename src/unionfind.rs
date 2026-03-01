// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use maplike::{Get, Push, Set};

/// Disjoint-set union data structure, also known as Union-Find.
#[derive(Clone, Debug)]
pub struct UnionFind<PC = Vec<usize>, RC = PC> {
    /// `parents[i]` is the parent of node `i`.
    pub(crate) parents: PC,
    /// `ranks[i]` is the upper bound of tree height for node `i`.
    pub(crate) ranks: RC,
}

impl<
    PC: Get<usize, Value = usize> + FromIterator<usize> + Push<usize> + Set<usize>,
    RC: Get<usize, Value = usize> + FromIterator<usize> + Push<usize> + Set<usize>,
> UnionFind<PC, RC>
{
    /// Create a new `UnionFind` with `size` elements (0..size).
    #[inline]
    pub fn with_len(len: usize) -> Self {
        Self::from_parents_ranks(
            PC::from_iter(0..len),
            RC::from_iter(std::iter::repeat(0).take(len)),
        )
    }
}

impl<PC: Default, RC: Default> UnionFind<PC, RC> {
    #[inline]
    pub fn new() -> Self {
        Self::from_parents_ranks(Default::default(), Default::default())
    }
}

impl<PC, RC> UnionFind<PC, RC> {
    /// Create a new `UnionFind` from given parent and rank collections.
    #[inline]
    pub fn from_parents_ranks(parents: PC, ranks: RC) -> Self {
        Self { parents, ranks }
    }

    /// Dissolve the polygon-union-find, returning its parent and rank
    /// collections, ceding ownership over them.
    #[inline]
    pub fn dissolve(self) -> (PC, RC) {
        (self.parents, self.ranks)
    }
}

impl<
    PC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    RC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> UnionFind<PC, RC>
{
    pub fn new_set(&mut self) -> usize {
        let new_set_index = self.ranks.push(0);
        self.parents.push(new_set_index);

        new_set_index
    }

    /// Find the representative of `x`, performing path compression along the
    /// way.
    pub fn find_compress(&mut self, x: usize) -> usize {
        if *self.parents.get(&x).unwrap() != x {
            // Perform the path compression.
            let parent = self.find_compress(*self.parents.get(&x).unwrap());
            self.parents.set(x, parent);
        }

        *self.parents.get(&x).unwrap()
    }

    pub fn find(&mut self, x: usize) -> usize {
        if *self.parents.get(&x).unwrap() != x {
            return self.find(*self.parents.get(&x).unwrap());
        }

        *self.parents.get(&x).unwrap()
    }

    /// Unionize the sets containing `x` and `y`, minimizing the rank.
    ///
    /// Returns true if merged, false if already in the same set.
    pub fn union(&mut self, x: usize, y: usize) -> bool {
        let mut x_representative = self.find_compress(x);
        let mut y_representative = self.find_compress(y);

        if x_representative == y_representative {
            return false; // Already connected.
        }

        // Perform union by rank.

        if self.ranks.get(&x_representative).unwrap() < self.ranks.get(&y_representative).unwrap() {
            std::mem::swap(&mut x_representative, &mut y_representative);
        }

        self.parents.set(y_representative, x_representative);

        if self.ranks.get(&x_representative).unwrap() == self.ranks.get(&y_representative).unwrap()
        {
            let rank = *self.ranks.get(&x_representative).unwrap();
            self.ranks.set(x_representative, rank + 1);
        }

        true
    }

    /// Check if `x` and `y` are in the same set.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find_compress(x) == self.find_compress(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(5);
        for i in 0..5 {
            assert_eq!(unionfind.parents[i], i);
        }
    }

    #[test]
    fn test_new_set() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::new();

        // First set.
        let s0 = unionfind.new_set();
        assert_eq!(s0, 0);
        assert_eq!(*unionfind.parents.get(&s0).unwrap(), s0);
        assert_eq!(*unionfind.ranks.get(&s0).unwrap(), 0);

        // Second set.
        let s1 = unionfind.new_set();
        assert_eq!(s1, 1);
        assert_eq!(*unionfind.parents.get(&s1).unwrap(), s1);
        assert_eq!(*unionfind.ranks.get(&s1).unwrap(), 0);

        // Make sure the two sets are not connected.
        assert!(!unionfind.connected(s0, s1));
    }

    #[test]
    fn test_union_idempotence() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(3);

        unionfind.union(0, 1);
        let representative_before = unionfind.find_compress(0);

        unionfind.union(0, 1); // Perform union on the same pair again.
        let representative_after = unionfind.find_compress(1);

        assert_eq!(representative_before, representative_after);
    }

    #[test]
    fn test_union_and_find() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(5);
        unionfind.union(0, 1);
        unionfind.union(1, 2);

        let representative0 = unionfind.find_compress(0);
        let representative1 = unionfind.find_compress(1);
        let representative2 = unionfind.find_compress(2);

        // The first three elements should have the same representative.
        assert_eq!(representative0, representative1);
        assert_eq!(representative1, representative2);

        // 3 and 4 are however still separate.
        assert_ne!(unionfind.find_compress(3), representative0);
        assert_ne!(unionfind.find_compress(4), representative0);
    }

    #[test]
    fn test_connected() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(4);
        unionfind.union(0, 1);
        unionfind.union(2, 3);

        assert!(unionfind.connected(0, 1));
        assert!(unionfind.connected(2, 3));
        assert!(!unionfind.connected(0, 2));

        unionfind.union(1, 2);
        assert!(unionfind.connected(0, 3));
    }
}
