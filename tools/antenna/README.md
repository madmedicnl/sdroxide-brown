# Antenna utility — prototype

A **standalone** prototype for the antenna utility idea in
[`docs/antenna-tool-ideas.md`](../../docs/antenna-tool-ideas.md), on branch
`fork/antenna-tool`. Nothing here is on `main` and nothing is wired into the
program yet.

Its own cargo workspace on purpose, like `tools/rsp1-capture`: the root build
(`members = ["crates/*"]`) never sees it, so it cannot affect a release and it
builds without the vendored submodules.

## What it is now

The **calculator** half only — closed-form dimensions and SWR arithmetic, no
method-of-moments engine. Given a frequency it prints the wire lengths for a
dipole, inverted-V, quarter-wave and 5/8-wave vertical, ground plane, delta loop,
quad loop and a Yagi starting layout.

```
cargo run -- 27.265
cargo test
```

## What comes next

The solver (`Zin`, SWR across a band, patterns, ground), then an obstacle
attenuation overlay — see the ideas doc for the formulas and the phasing.

## Note on the shortening factor

`K = 0.95` is the common thin-wire end-effect value, and it is a *starting*
number, not exact — it depends on wire diameter and height. That is what the
solver exists to replace.
