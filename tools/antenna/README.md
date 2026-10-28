# Antenna utility — prototype

A **standalone** prototype for the antenna utility idea in
[`docs/antenna-tool-ideas.md`](../../docs/antenna-tool-ideas.md), on branch
`fork/antenna-tool`. Nothing here is on `main` and nothing is wired into the
program yet.

Its own cargo workspace on purpose, like `tools/rsp1-capture`: the root build
(`members = ["crates/*"]`) never sees it, so it cannot affect a release and it
builds without the vendored submodules.

## What it is now

The **tool layer** (milestone A) — no method-of-moments engine yet:

- `lib.rs` — the closed-form calculator (dimensions + SWR) and a Yagi start.
- `model.rs` — the data model: wires, source, loads, ground, segmentation.
- `maa.rs` — MMANA-GAL `.maa` read/write (both variants read, B written).
- `store.rs` — `antennas.json` save/load.
- `expr.rs` — the `R`/`I`/`F` equation box and the length units.
- `presets.rs` — the CB/SWL shape presets (dipole, inverted-V, stick, ground
  plane, delta loop, Yagi), built from the same lengths as the calculator.
- `solver.rs` — the method-of-moments solver (triangular/rooftop basis, Galerkin,
  reduced kernel), validated against the textbook dipole values. Junctions
  solve: an unfed junction shares a node, and a feed at a non-collinear bend
  (the inverted-V apex) splits into the two wires' terminals.

```
cargo run -- 27.265                              # dimensions table
cargo run -- --presets 27.265                    # presets, as .maa
cargo run -- --solve 27.265                      # Zin and SWR of the presets
cargo run -- --pattern FILE 27.265               # dipole azimuth pattern as SVG
cargo run -- --pattern3d DIR 27.265              # each preset's 3D pattern (PLY + SVG)
cargo run -- --expr "5*R/8" 27.265               # the equation box
cargo run -- --convert 17ft --to m               # unit conversion
cargo test
```

## What comes next

The multi-way-junction feed (the ground plane), and the 2D polar plots. The
frequency sweep, far-field gain/F-B, the bend feed, lumped loads and the
Fresnel (real) ground are done. See the ideas doc for the formulas and the
phasing.

## Note on the shortening factor

`K = 0.95` is the common thin-wire end-effect value, and it is a *starting*
number, not exact — it depends on wire diameter and height. That is what the
solver exists to replace.
