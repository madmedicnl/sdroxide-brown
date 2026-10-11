# The antenna utility — ideas

> **Status: idea, not a plan.** Written down so it is not lost and not
> re-derived. Operator's ask, 2026-10-11.
>
> **A prototype exists, off `main`, on branch `fork/antenna-tool`:**
> `tools/antenna/` — a standalone cargo workspace (like `tools/rsp1-capture`).
> **Milestone A (the tool layer) is built:** the closed-form calculator (dipole,
> inverted-V, verticals, ground plane, delta/quad loop, Yagi start) with SWR; the
> data model (wires/source/loads/ground/segmentation); the `.maa` reader and
> writer; `antennas.json`; the `R`/`I`/`F` equation box and length units; and the
> CB/SWL presets. 27 tests, `cargo run -- 27.265`. Not a workspace member, not
> wired into the program, not pushed. The one A item deferred is the egui tool
> window (it belongs to integration, E). **Milestone B's core is also in and
> validated for junction-free wires** — a triangular/rooftop MoM solver whose
> dipole impedance and resonant length match the textbook values; junctions,
> ground, loads and the frequency sweep remain (see B below). Decisions:
> **attenuation-only obstacles** and the rest recorded below.

## The wish, in his words

> "I really would love to have a **simple utility but with advanced options**.
> It should do: I want freq X — how long should the wires of my dipole be, or how
> would that delta loop be like. A pattern preview where you can **place
> trees/obstacles** in, so you can see what happens to the TX pattern."

So it is two things wearing one name: a **calculator** a beginner can use, and a
**field-strength preview** an operator will poke at.

## The reference: MMANA-GAL

`http://gal-ana.de/basicmm/en/` — a free-for-personal **antenna analyzer** built
on the **MININEC-3** method-of-moments engine. It does: a wire-table editor, a
graphical/GUI wire and element editor, calculation of feed impedance, SWR, gain
and front-to-back, 2D and 3D far-field radiation patterns, Z/SWR/gain-vs-freq
plots, lumped loads (L/C/traps/R+jX), real-ground models, stacking, and an
optimizer. Its own limits (basic version): 10 000 segments, 600 wires.

It is the shape to aim at, not the thing to copy — its source is **closed**
(personal-use only), so the engine is reimplemented, not lifted.

## Where it would live

- A **tool window**, not a mode — `self.tool_window` already gives every menu
  tool a ⇱ chip and a remembered geometry (the SDRuno window set). Native-only;
  keep it out of any wasm-reachable crate.
- **Fork-only**, no upstream PR (standing direction).
- If it persists geometry/settings, that is a `UiSettings` field — no
  `PROTO_VERSION` bump; if it stores nothing, no config change at all.

## What is clean to build on

- **NEC-2 is a US-government public-domain work**, and the page itself says the
  MININEC BASIC engine was published public-domain. A thin-wire solver can be
  written from those references (and from the open literature) with a clean
  licence.
- We do **not** copy MMANA-GAL or any GPL NEC port's source.

## The engine, honestly

A thin-wire method-of-moments core is the hard part, not the UI:

- geometry (wire segments, junctions, taper segmentation),
- an EFIE solve (pulse basis, delta-gap source) → a dense complex linear system,
- a **ground model** (perfect / real via reflection coefficients — where most
  accuracy is won or lost),
- far-field integration for the patterns,
- then Z, SWR, gain, F/B and the plots on top.

None of it is novel; all of it is fiddly, and its accuracy is only as good as
the segmentation and the ground model — the traps MMANA's own help documents
(segment < 0.001 λ, radius/segment ratio, tapering near bends).

## The obstacle idea is the interesting half, and the non-standard one

"Place trees/obstacles and see what happens to the TX pattern" is **not** plain
MoM. In the physics, trees and terrain are:

- **vegetation/attenuation loss** along the path (a dB-per-metre-ish model), and
- **diffraction** off edges (knife-edge / Deygout-style), not a conductor you
  solve currents on.

So it is not "add a wire and solve" — it is a **separate propagation-overlay**
layer: you place an obstacle with a size/height/loss, and the far-field pattern
is multiplied by an attenuation computed along each azimuth/elevation ray. That
is a different algorithm from the MoM core, and it is where the honest design
question lives. It can be a coarse, clearly-labelled approximation (good enough
to answer "does that big tree to the north-east kill my low-angle takeoff?") or
a project of its own. Do not let it silently become either.

## What we need, in MMANA's terms

Every MMANA feature maps to one component. The table is the build list, in the
order the solver forces them.

| MMANA feature | what it is | our piece |
|---|---|---|
| Geometry table / Wire editor | wires as (x1,y1,z1)–(x2,y2,z2), radius, SEG | `Wire` model + table UI |
| Element editor | line / quad / triangle / loop forms | shape presets → wires |
| Sources | delta-gap feed, phase, voltage | `Source` on a segment |
| Loads | R+jX, L, C, parallel-LC trap | `Load` on a segment |
| Segmentation | DM1/DM2/SC/EC tapering | segmentation pass |
| Calculate | the MoM solve → Zin, SWR, gain, F/B | `sdroxide-ant` solver |
| Ground | perfect, or εr + σ (real) | ground model |
| Plots | Z / SWR / gain / F-B vs frequency | frequency sweep + plots |
| Far field | 2D azimuth+elevation, 3D | far-field sum + plot |
| Optimizer | tune chosen vars to targets | last |
| Wire scale / Make stack | rescale, arrays | presets / copy-multiply |
| File (`.maa`) | the text model format | our own JSON (+ optional `.maa` import) |

The minimum to be useful is: **Wire + Source + segmentation + ground + solve +
SWR/gain + one polar plot**. Loads, 3D, the optimizer and stacking are the
"advanced options".

## The formulas

### A. Closed-form lengths — the calculator, no solver

This answers "freq X → how long", and needs no matrix at all.

- λ = c/f; practical λ(m) ≈ 300/f(MHz), λ(ft) ≈ 984/f(MHz).
- **Half-wave dipole:** L = k·λ/2, k ≈ 0.95 (thin wire, well above ground) →
  ≈ 142.6/f(MHz) m, **468/f(MHz) ft**. Feed Z ≈ 73 + j42 Ω (≈ 73 Ω trimmed to
  resonance).
- **Quarter-wave vertical/monopole:** L = k·λ/4 ≈ 71.3/f(MHz) m (234/f ft) over a
  ground plane. Z ≈ 36 Ω.
- **5/8-λ vertical:** L = 0.625 λ.
- **Folded dipole:** same length as the half-wave; Z ≈ 4× (≈ 290 Ω).
- **Inverted-V:** two legs of k·λ/4, apex 90–120°; Z ≈ 50 Ω near 90°.
- **Full-wave delta loop:** perimeter ≈ 1.02 λ, equilateral side = P/3 ≈ 0.34 λ;
  Z ≈ 100–120 Ω. Fed at the bottom centre (horizontal-ish) or a corner (vertical).
- **Full-wave quad loop:** perimeter ≈ 1.03 λ, side ≈ λ/4.
- **Yagi starting values** (the solver refines them): reflector 0.50 λ, driven
  0.47–0.48 λ, directors 0.40–0.45 λ, spacing 0.15–0.25 λ.

SWR from the feed impedance: Γ = (Z − Z0)/(Z + Z0), SWR = (1+|Γ|)/(1−|Γ|).

The shortening factor k is not a constant — it depends on wire diameter and
height. That is exactly why phase 2 exists: the calculator gives a first length,
the solver gives the true one.

### B. The MoM core — the solver

Thin-wire **electric-field integral equation** on a wire curve C, current I(s′):

- Green's function of free space: G(R) = e^(−jkR)/R, R = |r − r′|.
  **Reduced kernel:** the source lies on the wire axis and the field point on the
  wire surface, so for a straight segment R = √((s − s′)² + a²) and G stays
  finite as s → s′.
- Enforce zero tangential E on the wire surface (all segments but the feed):
  ŝ·(E^inc + E^scat) = 0, with E^scat = −jωA − ∇φ,
  A(r) = (μ0/4π)∫ I(s′) ŝ′ G ds′,
  φ(r) = (1/4πε0) ∫ (1/jω)(dI/ds′) G ds′  (from continuity).
- Discretise: pulse basis (I constant per segment), **point matching** at segment
  centres → the dense complex system [Z][I] = [V].
- Matrix element, reduced-kernel Pocklington form:
  Z_mn = (jη0/4πk) ∫_{Δn} [ k² ŝ_m·ŝ_n − ∂²/(∂s_m ∂s_n) ] G(s_m,s_n) ds_n.
  The **self term** (m = n) is finite because of the reduced kernel and is
  dominated by ln(Δ_n/a); every other term is an ordinary quadrature. Overall
  sign conventions differ between texts — **NEC-2's documentation (public domain)
  is the reference to implement against**, not a half-remembered paper.
- **Source:** delta-gap. E^inc = V/Δ on the feed segment only, so V_m = V on that
  row and 0 elsewhere; I_feed = I at the feed segment.
- **Load** on a segment: add its impedance to the diagonal, Z_nn += Z_L
  (R+jX; L → jωL; C → 1/jωC; a trap is the parallel-LC impedance).

Segmentation (MMANA's DM1/DM2/SC/EC): taper so a segment runs between λ/(SC·DM1)
and λ/DM2, finest near bends and junctions. The accuracy traps MMANA itself
documents: segment < 0.001 λ is inaccurate, wire length/segment ratio < 4 for the
radius, and tapering beats uniform segmentation for a bent wire.

### C. Outputs

- Input power and impedance: P_in = ½·Re(V·I_f*), Z_in = V/I_f.
- Far field, summing segments as short dipoles (valid once Δ ≪ λ):
  **E(θ,φ) = (jη0 k e^(−jkr)/4πr) · Σ_n I_n Δ_n [ ŝ_n − (ŝ_n·r̂)r̂ ] e^(j k r̂·r_n)**.
  U = r²|E|²/(2η0), gain **G(θ,φ) = 4πU/P_in**; dBd = dBi − 2.15.
- **F/B:** gain at the forward maximum minus the maximum over the rear sector
  (MMANA's default rear sector is 120°).

### D. Ground

- **Perfect ground:** image theory. Mirror the antenna; horizontal current → the
  image current is reversed, vertical → it is the same, so a horizontal antenna
  has a null straight down.
- **Real ground:** multiply each image contribution by the **Fresnel reflection
  coefficient** at the grazing angle ψ, with the complex permittivity
  **εc = εr − j·60σλ** (σ in S/m; MMANA's table is in mS/m):
  R_v = (εc sinψ − √(εc − cos²ψ)) / (εc sinψ + √(εc − cos²ψ))   (vertical pol)
  R_h = (sinψ − √(εc − cos²ψ)) / (sinψ + √(εc − cos²ψ))          (horizontal pol)
  Presets from MMANA's own table: sea water (81, 4000), wet ground (5–15, 1–10),
  dry field/forest (13, 5), suburb/industrial (5, 1).
- **The known weak spot, and worth saying in the tool:** MININEC's reflection-
  coefficient ground is accurate for a plane wave over a homogeneous half-space,
  and worst for an antenna **< 0.2 λ above ground** — which is why MMANA tells
  users to reach for NEC2 there. We should say the same rather than pretend.

### E. Obstacle overlay — the tree/terrain layer

Not MoM: an attenuation multiplied into the far field per azimuth/elevation ray,
**|E′| = |E|·10^(−A/20)**.

- **Vegetation** (ITU-R P.833 family):
  Weissberger modified — A_dB = 1.33·f_GHz^0.284·d^0.588 (14 ≤ d ≤ 400 m);
  COST-235, d < 14 m — A_dB = 15.6·f_MHz^(−0.009)·d^0.26 (in-leaf) or
  26.6·f_MHz^(−0.2)·d^0.5 (out-of-leaf).
- **Diffraction** off an edge — knife-edge Fresnel:
  v = h·√((2/λ)(1/d1 + 1/d2)); J(v) = 6.9 + 20·log10(√((v − 0.1)² + 1) + v − 0.1)
  (≈ 0 for v ≤ −0.7). Several edges: Deygout.

This is a coarse, **labelled** model — it is not a terrain solver and cannot be,
without a DEM.

## What it could answer, phased

1. **Calculator (small, high value).** Type a frequency and get the wire lengths
   for the common shapes — half-wave dipole, inverted-V, vertical, ground plane,
   delta loop (full-wave, the equilateral and the 1:1:1 variants), and the
   feedpoint of each. No solver required: closed-form lengths + the shortening
   factor. Answers "20 m dipole → ~5.06 m a side" in one keystroke.
2. **Solver.** A wire-table editor + the MoM core → feed Z, SWR across a band,
   and a 2D azimuth/elevation polar plot. Presets for the shapes above.
3. **Far-field 3D** and an **optimizer** last — MMANA's bulk, and where a
   half-done version misleads.
4. **Obstacle overlay** — the tree/terrain attenuation layer, clearly labelled
   as an approximation, on top of the pattern.

The realistic caveat to repeat to anyone who asks: a from-scratch MoM engine is
a project, and phase 1 alone covers most real CB/SWL questions (is my dipole the
right length, and why is my SWR high here).

## Decisions (2026-10-11)

1. **Obstacle overlay — attenuation-only.** Vegetation + knife-edge, as in §E,
   labelled an approximation. No terrain/DEM model.
2. **TX first, RX nice-to-have.** The tool is a transmit-pattern designer; feeding
   the predicted takeoff into the receive/propagation side is a later extra, not
   a v1 requirement.
3. **Shapes: wire, delta loop, stick (vertical), … expandable.** Start with the
   wire family and grow the list; magnetic loop/quad can come later.
4. **Calculator first.** Phase 1 (the closed-form calculator) is the agreed start;
   the solver follows.
5. **Optimizer: later.** Wanted, but not at the beginning.
6. **Import `.maa`: definitely.** Reading MMANA's own plain-text format lets users
   bring existing models in.
7. **Persistence: own JSON** (`antennas.json`, like `memories.json` — easy to
   share, no `UiSettings`/wire change).
8. **Presets: yes.** Ship the common CB/SWL shapes.
9. **Units: metres *and* MMANA's feet/inches/λ, plus the `R`/`I`/`F` equation
   box** (`R/4` = λ/4). So the expression parser is in scope, not a nicety.
10. **CB/SWL-first.** The audience and the worked examples are the 11 m / SWL
    operator.

Still open:

- **Far-field fidelity.** A 2D azimuth + elevation pair is the useful minimum; a
  3D hemisphere can reuse the solar scene's renderer. Recommendation: **start
  with the 2D polar pair** (exact angles and nulls are readable on it) and add a
  **3D toggle** once the solver works — the 3D looks better but reads worse for
  engineering, so it should complement the 2D, not replace it.

## Consequences for the plan

- The **calculator's presets** (decision 8) and the **`.maa` reader** (6) mean
  `tools/antenna` grows a small file-format module before the solver does.
- The **equation box** (9) is its own little parser over `R`/`I`/`F` and the
  four operators — cheap, but it is work, not a checkbox.
- The **RF engine** stays as in §B–§D; decisions 2, 3 and 5 only move *when*
  things land, not *what* they are.

## Todo (from the feasibility assessment, 2026-10-11)

Ordered to ship the low-risk half first, then prove the risky half before more
UI is built around it. **Milestone gate: if this starts to look like
reimplementing MMANA, stop — a wrong solver is worse than none.** Keep it on
`fork/antenna-tool`, unpushed, until it is worth merging.

**A — Tool layer (low risk, shippable on its own)**
- [ ] Collect golden reference values for the solver tests — a dipole's Zin, a
      known delta loop, a known Yagi's gain and F/B — from NEC/MMANA/ARRL
      examples. Prerequisite for B's tests, do it first. *(Deferred to B — it is
      solver work, not tool work.)*
- [x] `.maa` reader and writer (MMANA's plain-text model format) — both variants
      read, variant B written; MMANA's own example is the fixture.
- [x] `antennas.json` persistence (own file, like `memories.json`) — serde, with
      the `float_roundtrip` feature so a coordinate reloads bit-identical.
- [x] The `R`/`I`/`F` equation parser and unit conversion (m / ft / in / λ).
- [x] CB/SWL shape presets, and the shape list made **expandable** — dipole,
      inverted-V, vertical (stick), ground plane, delta loop, Yagi.
- [ ] The calculator tool-window UI — **moved to E (integration)**: it cannot
      exist in the standalone prototype, only once the code is in the program.

The data model (`model.rs`) is also in place — the wire/source/load/ground /
segmentation types the `.maa` file and `antennas.json` both hold. The
*segmentation algorithm* (turning DM1/DM2/SC/EC into concrete counts) is still a
solver task (B); the model only stores the parameters.

**B — Solver (the risky half; validated before C is built on it)**

*The core is built and validated — see `tools/antenna/src/solver.rs`. A dipole
of the preset length solves to ~70 + j9 Ω at 27.265 MHz (SWR 1.46); a 3-element
Yagi comes out at 7.5 dBi with 15 dB front-to-back. The near-singular self term
was the hard part and is handled.*

- [x] Golden reference values — a **short** dipole's input resistance against
      `20π²(l/λ)²` (7.7 Ω vs 7.9 Ω at 0.2 λ); a thin dipole **resonates** at
      ≈0.47–0.48 λ with a **≈73 Ω** resistance; a λ/2 dipole has **~2.15 dBi**
      and no front-to-back. All live tests.
- [x] **Triangular (rooftop) current basis, Galerkin, reduced kernel.**
- [x] Delta-gap source at a node; complex solve; `Zin = V/I_feed`.
- [x] Uniform segmentation (fixed per-wire or ~80/λ auto).
- [x] **Junction handling** — wires that meet share one node/current (Kirchhoff
      comes free). The delta loop solves.
- [x] **Far-field gain/F-B and a frequency sweep** (`max_gain`,
      `front_to_back_db`, `sweep`).
- [ ] **Bend-feed model** — a feed on a bend or multi-way junction is currently
      **refused** (the delta gap has no single direction there); this blocks the
      inverted-V apex and the ground-plane star. The next solver item.
- [ ] Lumped loads on a segment (R+jX, L, C, parallel-LC trap) — the model holds
      them; the solver does not yet apply them.
- [x] **Ground — free space and perfect (image method).** Validated: a
      quarter-wave monopole over a perfect ground comes out at half a dipole,
      ≈35 Ω and ~5.1 dBi (3 dB over the dipole). The presets now carry a ground
      (horizontals in free space, the vertical/GP over perfect).
- [ ] **Real (Fresnel) ground** — refused for now rather than treated as
      something else; the reflection-coefficient model with the εr/σ presets and
      the <0.2 λ caveat is the next ground item.
- [ ] MMANA's DM1/DM2/SC/EC tapering.

**C — Patterns**
- [ ] 2D azimuth + elevation polar plots (the readable view — do this first).
- [ ] 3D hemisphere toggle, reusing the solar-scene renderer.

**D — Advanced (only once A–C are honest)**
- [ ] Optimizer over chosen variables (jX / SWR / gain / F-B / elevation).
- [ ] Obstacle attenuation overlay — vegetation + knife-edge, **labelled an
      approximation** (decision 1).
- [ ] Stacking and wire scaling (MMANA's Make stack / Wire scale).

**E — Integration**
- [ ] Promote `tools/antenna` → `crates/sdroxide-ant`, wire the tool window
      (`self.tool_window`), then merge the branch to `main`.
- [ ] Manual section, README entry and a quick-start.
- [ ] RX nice-to-have (decision 2): feed the predicted takeoff into the
      receive/propagation side.
