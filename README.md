# polygon_unionfind

`polygon_unionfind` is a disjoint-set data structure (union-find) where sets
are polygons.

Upon insertion, each polygon is merged with all the intersecting polygons
that are already present. To reduce the cost of finding intersecting polygons,
polygons are spatially filtered by being kept in an R-tree.

## Usage

### Adding dependency

First, add `polygon_unionfind` as a dependency to your `Cargo.toml`:

```toml
[dependencies]
polygon_unionfind = "0.5"
```

### Basic usage

Following is a basic usage example of `polygon_unionfind`.

```rust
use polygon_unionfind::{Point, Polygon, PolygonUnionFind};

let mut polygon_unionfind = PolygonUnionFind::<i64, &str>::new();

let first = polygon_unionfind.insert(Polygon {
    vertices: vec![
        Point { x: 0, y: 0 },
        Point { x: 3, y: 0 },
        Point { x: 0, y: 3 },
    ],
    weight: "first",
});

let second = polygon_unionfind.insert(Polygon {
    vertices: vec![
        Point { x: 1, y: 0 },
        Point { x: 4, y: 0 },
        Point { x: 1, y: 3 },
    ],
    weight: "second",
});

// Overlapping polygons are now in the same set and thus have the same
// representative.
let repr_first = polygon_unionfind.find(first).vertices.len();
let repr_second = polygon_unionfind.find(second).vertices.len();
assert_eq!(rep_first, rep_second);
```

## Documentation

See the [documentation](https://docs.rs/dcel/latest/dcel) for more information
about `dcel`'s usage.

## Features

- `std` (default): enables `std` support.
- `undoredo` (optional): enables integration with the
  [`undoredo`](https://crates.io/crates/undoredo) crate.

## Packaging

`dcel` is published as a [crate](https://crates.io/crates/dcel) on the
[Crates.io](https://crates.io/) registry.

## Contributing

We welcome issues and pull requests from anyone to our canonical
[repository](https://codeberg.org/topola/dcel) on Codeberg.

## Licence

### Outbound licence

`dcel` is dual-licensed as under either of

- [MIT license](./LICENSES/MIT.txt),
- [Apache License, Version 2.0](./LICENSES/Apache-2.0.txt),

at your option.

### Inbound licence

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you will be dual-licensed as described above,
without any additional terms or conditions.
