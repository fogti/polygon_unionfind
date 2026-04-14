# polygon_unionfind

`polygon_unionfind` is a disjoint-set data structure (union-find) where sets
are polygons.

The polygons are stored in an R-tree. 

It stores polygons, tracks connected components with union-find, and uses
`i_overlay` to merge intersecting polygons into a single representative shape.

## Basic usage

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

## Features

- `std` (default): enables `std` support.
- `undoredo` (optional): enables integration aliases intended for `undoredo`
  recorders/deltas.

## License

Licensed under either:

- MIT License
- Apache License, Version 2.0
