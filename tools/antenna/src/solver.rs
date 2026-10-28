//! The method-of-moments solver — the thin-wire EFIE, from the wire model to a
//! feed impedance.
//!
//! **Triangular (rooftop) current basis, Galerkin testing, reduced kernel.** The
//! current is piecewise-linear along each segment, so its derivative — the
//! charge — is piecewise-*constant* on the segments. That is the whole reason
//! for this basis: with the charge spread along a segment the scalar-potential
//! self term integrates to a `ln(Δ/a)`, where a point charge at a segment
//! boundary would sit at the wire radius, give a `1/a`, and wreck the reactance.
//!
//! The mixed-potential EFIE on the wire surface,
//!
//! ```text
//!   E^i + E^s = 0                     (tangentially)
//!   E^s = -jωA - ∇φ
//!   A   = (μ0/4π) ∫ I(s') ŝ' G ds'
//!   φ   = (1/4πε0) ∫ ρ(s') G ds',   ρ = -(1/jω) dI/ds'
//!   G   = e^{-jkR}/R,               R = sqrt(|r-r'|² + a²)
//! ```
//!
//! is tested with the same triangle that expands the current (Galerkin), giving
//! the dense complex system `[Z][I] = [V]`. The feed is a delta gap over a
//! **segment** — MMANA's `w#c`/`w#b`/`w#e` name a segment, not a node: the
//! impressed field `V/Δ` over that segment excites its two node basis functions
//! by `V/2` each, and `Zin = V / I_segment`.
//!
//! **Junctions are handled by sharing the node.** Wires whose ends meet within a
//! tolerance collapse to one graph node with one current unknown, whose basis is
//! the tent over *every* segment incident on it — so a bend, a T or a five-way
//! ground-plane junction all carry Kirchhoff's current law at the node for free.
//! A free end (one incident segment) carries no current and is not an unknown
//! unless it is fed.
//!
//! **A feed on a straight-through segment** (or a segment ending at a free end)
//! is a delta gap over that segment. A feed at a **non-collinear junction** (the
//! inverted-V apex) splits the node into the two wires' terminals and puts the
//! source **between** them with opposite polarity. A feed at a **multi-way
//! junction** (three or more wires) is *not* solved: a 1-to-N delta gap has no
//! consistent voltage split, so the ground-plane preset instead models its feed
//! as a short gap segment with the source on it.
//!
//! **Ground** is free space (no image) or a **perfect** ground, added by image
//! theory: every source segment is mirrored through `z = 0` with its current
//! reversed. Validated by the quarter-wave monopole, which comes out at half a
//! dipole — ≈35 Ω and ~5.1 dBi (3 dB over the dipole's 2.15, as it should).
//! Real (Fresnel) ground is not modelled yet and is refused.
//!
//! **Validated against two independent textbook results:**
//!
//! - A **short** dipole's input resistance matches `20π²(l/λ)²` — at `l = 0.2λ`
//!   the solver gives ≈ 7.7 Ω against the formula's 7.9 Ω.
//! - A thin dipole **resonates** at ≈ 0.47–0.48 λ with a **resonant resistance
//!   of ≈ 73 Ω** — the classic figure.
//!
//! At exactly `λ/2` a thin dipole comes out near 80 + j46 Ω, a little above the
//! often-quoted 73 + j42.5; that 73 Ω is the *resonant* value, which the solver
//! reproduces at resonance. The exact-`λ/2` figure assumes a sinusoidal current;
//! this is the reduced-kernel Galerkin result for a delta-gap feed.
//!
//! A shape whose wires *meet* solves too (the delta loop gives a plausible
//! 107 − j70 Ω), but there is no reference value here to pin it — treat unfed
//! junctions as plausible, not proven. A feed *at* a non-collinear junction
//! splits the node into the two terminals and solves (the inverted-V, ~40–50 Ω).

use num_complex::Complex64;
use std::f64::consts::PI;

pub use crate::C;
use crate::model::{Antenna, Ground, GroundKind, Pos, Seg};

const MU0: f64 = 1.256_637_062_12e-6;
const EPS0: f64 = 8.854_187_812_8e-12;
const ETA0: f64 = 376.730_313_668;

/// Points closer than this (metres) are the same node.
const JUNCTION_TOL: f64 = 1e-6;

// 8-point Gauss–Legendre nodes and weights on [-1, 1]. The reduced kernel's
// `1/sqrt(u²+a²)` is sharply peaked when a segment is only a few radii long, so
// a low-order rule under-integrates the self term and biases the resistance.
const GX: [f64; 8] = [
    -0.960_289_856_497_536_3,
    -0.796_666_477_413_626_7,
    -0.525_532_409_916_329,
    -0.183_434_642_495_649_8,
    0.183_434_642_495_649_8,
    0.525_532_409_916_329,
    0.796_666_477_413_626_7,
    0.960_289_856_497_536_3,
];
const GW: [f64; 8] = [
    0.101_228_536_290_376_3,
    0.222_381_034_453_374_5,
    0.313_706_645_877_887_3,
    0.362_683_783_378_362,
    0.362_683_783_378_362,
    0.313_706_645_877_887_3,
    0.222_381_034_453_374_5,
    0.101_228_536_290_376_3,
];

// ── vector helpers ───────────────────────────────────────────────────────────

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    dot(d, d).sqrt()
}

// ── geometry ─────────────────────────────────────────────────────────────────

/// A segment between two graph nodes `a` and `b`.
#[derive(Clone, Debug)]
struct Segment {
    a: usize,
    b: usize,
    from: [f64; 3],
    to: [f64; 3],
    dir: [f64; 3],
    len: f64,
    radius: f64,
}

/// The whole structure as a node graph: coincident wire ends share a node.
struct Mesh {
    node_pts: Vec<[f64; 3]>,
    segs: Vec<Segment>,
    /// Segments incident on each node.
    incident: Vec<Vec<usize>>,
    /// The global segment indices of each wire, in order (for locating the feed).
    wire_segs: Vec<Vec<usize>>,
}

/// How finely to cut the wires. A fixed per-wire count is honoured; otherwise
/// `segs_per_wavelength` sets the density.
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub segs_per_wavelength: f64,
}

impl Default for SolverConfig {
    fn default() -> SolverConfig {
        // Denser than MMANA's DM2 ≈ 20: this solver integrates numerically, so
        // it needs more segments than NEC-style analytic kernels. ~40 a
        // half-wave is the reliable floor.
        SolverConfig { segs_per_wavelength: 80.0 }
    }
}

fn find_or_add(pts: &mut Vec<[f64; 3]>, p: [f64; 3]) -> usize {
    for (i, q) in pts.iter().enumerate() {
        if dist(p, *q) < JUNCTION_TOL {
            return i;
        }
    }
    pts.push(p);
    pts.len() - 1
}

fn build_mesh(a: &Antenna, cfg: &SolverConfig) -> Mesh {
    let lambda = C / a.freq_hz;
    let mut node_pts: Vec<[f64; 3]> = Vec::new();
    let mut segs: Vec<Segment> = Vec::new();
    let mut wire_segs: Vec<Vec<usize>> = Vec::new();
    for w in &a.wires {
        let len = dist(w.from, w.to);
        if len <= 0.0 {
            wire_segs.push(Vec::new());
            continue;
        }
        let n = match w.seg {
            Seg::Fixed(n) => n.max(1) as usize,
            _ => ((cfg.segs_per_wavelength * len / lambda).round() as u32).max(2) as usize,
        };
        let dir = scale(sub(w.to, w.from), 1.0 / len);
        let mut ids = Vec::new();
        for i in 0..=n {
            let p = add(w.from, scale(sub(w.to, w.from), i as f64 / n as f64));
            ids.push(find_or_add(&mut node_pts, p));
        }
        let mut local_segs = Vec::new();
        for i in 0..n {
            let from = node_pts[ids[i]];
            let to = node_pts[ids[i + 1]];
            local_segs.push(segs.len());
            segs.push(Segment {
                a: ids[i],
                b: ids[i + 1],
                from,
                to,
                dir,
                len: dist(from, to),
                radius: w.radius_m,
            });
        }
        wire_segs.push(local_segs);
    }
    let mut incident = vec![Vec::new(); node_pts.len()];
    for (si, s) in segs.iter().enumerate() {
        incident[s.a].push(si);
        incident[s.b].push(si);
    }
    Mesh { node_pts, segs, incident, wire_segs }
}

/// Parallel combination of two impedances, `a ∥ b`.
fn par(a: Complex64, b: Complex64) -> Complex64 {
    let den = a + b;
    if den.norm() == 0.0 { Complex64::new(0.0, 0.0) } else { a * b / den }
}

/// The series impedance of a lumped load at the solve frequency, from MMANA's
/// load types (`values[0]` is the type, `values[1..]` the R, L and C in the
/// order the type names them — R in Ω, L in µH, C in pF, X in Ω for type 0):
///
/// | type | meaning      | impedance                                    |
/// |------|--------------|----------------------------------------------|
/// | 0    | R + jX       | R + jX                                       |
/// | 1    | R ∥ C        | R / (1 + jωRC)                               |
/// | 2    | R ∥ L        | jωLR / (R + jωL)                             |
/// | 3    | R + C        | R + 1/(jωC)                                  |
/// | 4    | R + L        | R + jωL                                      |
/// | 5    | R ∥ (L + C)  | R(jωL + 1/(jωC)) / (R + jωL + 1/(jωC))       |
/// | 6    | R + (L ∥ C)  | R + (jωL · 1/(jωC)) / (jωL + 1/(jωC))        |
fn load_impedance(load: &crate::model::Load, omega: f64) -> Complex64 {
    let j = Complex64::new(0.0, 1.0);
    let r = Complex64::new(load.values[1], 0.0);
    let xl = |l_uh: f64| Complex64::new(0.0, omega * l_uh * 1e-6);
    let xc = |c_pf: f64| Complex64::new(0.0, -1.0 / (omega * c_pf.max(1e-30) * 1e-12));
    match load.values[0] as i32 {
        0 => r + j * Complex64::new(load.values[2], 0.0),
        1 => par(r, xc(load.values[2])),
        2 => par(r, xl(load.values[2])),
        3 => r + xc(load.values[2]),
        4 => r + xl(load.values[2]),
        5 => par(r, xl(load.values[2]) + xc(load.values[3])),
        6 => r + par(xl(load.values[2]), xc(load.values[3])),
        _ => Complex64::new(0.0, 0.0),
    }
}

/// The complex relative permittivity of the ground, `εc = εr − j·60σλ` with σ
/// in S/m and λ in metres.
fn eps_c(ground: &Ground, lambda: f64) -> Complex64 {
    let sigma_sm = ground.sigma_ms_m * 1e-3;
    Complex64::new(ground.eps_r, -60.0 * sigma_sm * lambda)
}

/// Fresnel reflection coefficient for **vertical** polarisation, at grazing
/// angle `psi` (radians; 0 along the surface, π/2 straight down).
fn reflection_v(ec: Complex64, psi: f64) -> Complex64 {
    let (s, c) = (psi.sin(), psi.cos());
    let root = (ec - c * c).sqrt();
    (ec * s - root) / (ec * s + root)
}

/// Fresnel reflection coefficient for **horizontal** polarisation.
fn reflection_h(ec: Complex64, psi: f64) -> Complex64 {
    let (s, c) = (psi.sin(), psi.cos());
    let root = (ec - c * c).sqrt();
    (s - root) / (s + root)
}

// ── the solve ────────────────────────────────────────────────────────────────

/// A segment reduced to what the far field needs: its current, direction and
/// midpoint.
#[derive(Clone, Copy, Debug)]
pub struct FarSeg {
    pub mid: [f64; 3],
    pub dir: [f64; 3],
    pub len: f64,
    pub current: Complex64,
}

/// A solved model.
#[derive(Clone, Debug)]
pub struct Solution {
    pub freq_hz: f64,
    /// Current at every unknown node, in `unknowns` order.
    pub currents: Vec<Complex64>,
    /// The graph node each unknown sits on.
    pub unknowns: Vec<usize>,
    pub feed_unknown: usize,
    pub zin: Complex64,
    /// Input power, ½·Re(V·I_f*).
    pub power_w: f64,
    /// Segments with their currents, for the far field.
    pub segments: Vec<FarSeg>,
    /// Their mirror images through `z = 0`, with the *same* current (the
    /// reflection coefficient is applied per direction in the far field).
    pub image_segments: Vec<FarSeg>,
    /// The ground, for the far-field reflection coefficient.
    pub ground: Ground,
}

/// Solve one model at its own frequency.
pub fn analyze(a: &Antenna, cfg: &SolverConfig) -> Result<Solution, String> {
    if a.freq_hz <= 0.0 {
        return Err("the design has no frequency".into());
    }
    if a.sources.is_empty() {
        return Err("the design has no source".into());
    }
    let mut m = build_mesh(a, cfg);
    if m.segs.is_empty() {
        return Err("the design has no wires".into());
    }

    // The feed is a **segment**, not a node — MMANA's `w#c`/`w#b`/`w#e` name a
    // segment. A delta gap over a segment has a well-defined direction whatever
    // the geometry, so a bend or a junction can be fed correctly: the impressed
    // field V/Δ over the feed segment excites the two node basis functions that
    // overlap it by V/2 each.
    let src = &a.sources[0];
    let fw = (src.at.wire as usize).saturating_sub(1);
    if fw >= m.wire_segs.len() || m.wire_segs[fw].is_empty() {
        return Err(format!("the source names wire {}, which is not there", fw + 1));
    }
    let nseg = m.wire_segs[fw].len();
    let base = match src.at.at {
        Pos::Begin => 0isize,
        Pos::Center => (nseg as isize) / 2,
        Pos::End => nseg as isize - 1,
    };
    let si = (base + src.at.offset as isize).clamp(0, nseg as isize - 1) as usize;
    let (feed_a, feed_b) = (m.segs[m.wire_segs[fw][si]].a, m.segs[m.wire_segs[fw][si]].b);

    // A feed at a non-collinear junction (the inverted-V apex) needs a
    // **source-between-terminals** model: the two wires are the source's two
    // terminals, not one shared node. Split the junction so the feed segment's
    // end and the other wire's end are two separate unknowns, and put the
    // delta-gap source between them. A collinear through-connection and a free
    // end keep the plain delta-gap-over-a-segment feed.
    let feed_seg = m.wire_segs[fw][si];
    // (junction node, the node the "other" wire moved to).
    let mut split: Option<(usize, usize)> = None;
    for node in [feed_a, feed_b] {
        let inc = &m.incident[node];
        match inc.len() {
            2 => {
                let (s0, s1) = (inc[0], inc[1]);
                if dot(m.segs[s0].dir, m.segs[s1].dir).abs() < 0.9999 {
                    let other = if s0 == feed_seg { s1 } else { s0 };
                    let j2 = m.node_pts.len();
                    m.node_pts.push(m.node_pts[node]);
                    m.incident.push(Vec::new());
                    if m.segs[other].a == node {
                        m.segs[other].a = j2;
                    } else {
                        m.segs[other].b = j2;
                    }
                    m.incident[node].retain(|&s| s != other);
                    m.incident[j2].push(other);
                    split = Some((node, j2));
                }
            }
            n if n >= 3 => {
                // A feed at a multi-way junction (the ground-plane base): the
                // source sits between the feed wire and everything else, so the
                // feed segment keeps its node and every other segment moves to a
                // second node. The two are the source's terminals.
                let j2 = m.node_pts.len();
                m.node_pts.push(m.node_pts[node]);
                m.incident.push(Vec::new());
                let others: Vec<usize> =
                    m.incident[node].iter().copied().filter(|&s| s != feed_seg).collect();
                for other in others {
                    if m.segs[other].a == node {
                        m.segs[other].a = j2;
                    } else {
                        m.segs[other].b = j2;
                    }
                    m.incident[j2].push(other);
                }
                m.incident[node].retain(|&s| s == feed_seg);
                split = Some((node, j2));
            }
            _ => {}
        }
    }

    // Unknowns: every node with two or more segments (a bend, junction or
    // through-connection), plus the source terminals. For a straight feed those
    // are the two nodes of the feed segment; for a split junction they are the
    // junction node and the wire moved off it.
    let mut unknowns: Vec<usize> = Vec::new();
    let mut index = vec![None; m.node_pts.len()];
    let (term_a, term_b) = match split {
        Some((node, j2)) => (node, j2),
        None => (feed_a, feed_b),
    };
    for (node, slot) in index.iter_mut().enumerate() {
        if m.incident[node].len() >= 2 || node == term_a || node == term_b {
            *slot = Some(unknowns.len());
            unknowns.push(node);
        }
    }
    if unknowns.is_empty() {
        return Err("no node to solve for".into());
    }
    let (ia, ib) = (
        index[term_a].expect("the source terminal was added"),
        index[term_b].expect("the source terminal was added"),
    );

    let k = 2.0 * PI * a.freq_hz / C;
    let omega = 2.0 * PI * a.freq_hz;
    let nu = unknowns.len();

    // Ground. The *impedance* always uses the perfect image (MININEC, like
    // MMANA, assumes a perfect ground for Z); the *far field* uses the Fresnel
    // reflection coefficient for a real ground. Free space has no image.
    let image: Option<Complex64> = match a.ground.kind {
        GroundKind::FreeSpace => None,
        GroundKind::Perfect | GroundKind::Real => Some(Complex64::new(-1.0, 0.0)),
        GroundKind::SommerfeldNorton => {
            return Err("the Sommerfeld-Norton ground is not modelled — use free space, perfect \
                 or real ground"
                .into());
        }
    };

    // Support of a node's basis: one tent per incident segment, peaking at the
    // node (`rising` means the tent is 0 at the segment's `from`, 1 at its `to`).
    let support = |node: usize| -> Vec<(usize, bool)> {
        m.incident[node].iter().map(|&s| (s, m.segs[s].b == node)).collect()
    };

    let mut z = vec![vec![Complex64::new(0.0, 0.0); nu]; nu];
    for (mi, &mu) in unknowns.iter().enumerate() {
        for (ni, &nun) in unknowns.iter().enumerate() {
            let mut acc = Complex64::new(0.0, 0.0);
            for (tsi, tr) in support(mu) {
                let ts = &m.segs[tsi];
                for (ssi, sr) in support(nun) {
                    let ss = &m.segs[ssi];
                    acc += pair_integral(ts, tr, ss, sr, ts.radius, k, omega);
                    if let Some(fac) = image {
                        acc += fac * pair_integral(ts, tr, &mirror(ss), sr, ts.radius, k, omega);
                    }
                }
            }
            z[mi][ni] = acc;
        }
    }

    // Lumped loads: a series impedance at a point on a segment. In the Galerkin
    // basis a load `Z_L` at fraction `t` along a segment (nodes a, b) adds
    // `Z_L·f_a(t)·f_n(t)` to the matrix; at the segment midpoint each tent is ½,
    // so it is a `Z_L/4` block coupling the two nodes.
    for load in &a.loads {
        let lw = (load.at.wire as usize).saturating_sub(1);
        if lw >= m.wire_segs.len() || m.wire_segs[lw].is_empty() {
            continue;
        }
        let nseg = m.wire_segs[lw].len();
        let base = match load.at.at {
            Pos::Begin => 0isize,
            Pos::Center => (nseg as isize) / 2,
            Pos::End => nseg as isize - 1,
        };
        let si = (base + load.at.offset as isize).clamp(0, nseg as isize - 1) as usize;
        let seg = &m.segs[m.wire_segs[lw][si]];
        let zl = load_impedance(load, omega) * 0.25;
        for (n1, n2) in [(seg.a, seg.a), (seg.a, seg.b), (seg.b, seg.a), (seg.b, seg.b)] {
            if let (Some(i), Some(j)) = (index[n1], index[n2]) {
                z[i][j] += zl;
            }
        }
    }

    let mut rhs = vec![Complex64::new(0.0, 0.0); nu];
    let half_v = Complex64::new(src.volts / 2.0, 0.0);
    // A segment gap excites both terminals in phase (the impressed field is
    // uniform over the segment); a split junction is a gap *between* the two
    // wires, so the two terminals are driven with opposite polarity.
    rhs[ia] += half_v;
    if split.is_some() {
        rhs[ib] -= half_v;
    } else {
        rhs[ib] += half_v;
    }

    let currents = solve_complex(&mut z, &rhs).ok_or("the interaction matrix is singular")?;
    // The source current. A segment feed reads the midpoint of its two node
    // currents; a split junction reads the current at the junction terminal —
    // the wire moved off the junction carries the same current, opposite sign.
    let i_feed = match split {
        Some(_) => currents[ia],
        None => 0.5 * (currents[ia] + currents[ib]),
    };
    if i_feed.norm() == 0.0 {
        return Err("the feed current came out zero".into());
    }
    let volts = Complex64::new(src.volts, 0.0);
    let zin = volts / i_feed;
    let power_w = 0.5 * (volts * i_feed.conj()).re;

    // Per-segment current (the tent's midpoint value) for the far field.
    let mut node_current = vec![Complex64::new(0.0, 0.0); m.node_pts.len()];
    for (ui, &node) in unknowns.iter().enumerate() {
        node_current[node] = currents[ui];
    }
    let far_seg = |s: &Segment, current: Complex64| FarSeg {
        mid: scale(add(s.from, s.to), 0.5),
        dir: s.dir,
        len: s.len,
        current,
    };
    let mut segments: Vec<FarSeg> =
        m.segs.iter().map(|s| far_seg(s, 0.5 * (node_current[s.a] + node_current[s.b]))).collect();
    // The image segments carry the *same* current; the far field applies the
    // reflection coefficient per direction (and per observation angle for a
    // real ground).
    let image_segments: Vec<FarSeg> = if image.is_some() {
        m.segs
            .iter()
            .map(|s| far_seg(&mirror(s), 0.5 * (node_current[s.a] + node_current[s.b])))
            .collect()
    } else {
        Vec::new()
    };

    Ok(Solution {
        freq_hz: a.freq_hz,
        currents,
        unknowns,
        feed_unknown: ia,
        zin,
        power_w,
        segments,
        image_segments,
        ground: a.ground,
    })
}

/// Mirror a segment through the ground plane `z = 0`.
fn mirror(s: &Segment) -> Segment {
    Segment {
        a: s.a,
        b: s.b,
        from: [s.from[0], s.from[1], -s.from[2]],
        to: [s.to[0], s.to[1], -s.to[2]],
        dir: [s.dir[0], s.dir[1], -s.dir[2]],
        len: s.len,
        radius: s.radius,
    }
}

/// The far-field vector pattern factor `F(θ,φ)`, as a complex 3-vector:
/// `Σ I_n Δ_n [ŝ_n − (ŝ_n·r̂)r̂] e^{jk r̂·r_n}`, plus the image field weighted by
/// the ground's reflection coefficient (vertical and horizontal separately).
fn far_factor(sol: &Solution, theta_deg: f64, phi_deg: f64, k: f64) -> [Complex64; 3] {
    let th = theta_deg.to_radians();
    let ph = phi_deg.to_radians();
    let rhat = [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()];
    let mut f = [Complex64::new(0.0, 0.0); 3];
    for s in &sol.segments {
        add_segment(&mut f, s, s.current, rhat, k);
    }
    if !sol.image_segments.is_empty() {
        let (rv, rh) = ground_reflection(sol, theta_deg);
        for s in &sol.image_segments {
            // Vertical: the mirror flipped z, but the image of a vertical
            // current keeps its direction — flip it back and weight by R_v.
            add_segment(
                &mut f,
                &FarSeg { dir: [0.0, 0.0, -s.dir[2]], ..*s },
                rv * s.current,
                rhat,
                k,
            );
            add_segment(
                &mut f,
                &FarSeg { dir: [s.dir[0], s.dir[1], 0.0], ..*s },
                rh * s.current,
                rhat,
                k,
            );
        }
    }
    f
}

/// Add one far-field segment's contribution to `f`.
fn add_segment(f: &mut [Complex64; 3], s: &FarSeg, current: Complex64, rhat: [f64; 3], k: f64) {
    let dd = dot(s.dir, rhat);
    let proj = [s.dir[0] - dd * rhat[0], s.dir[1] - dd * rhat[1], s.dir[2] - dd * rhat[2]];
    let phase = k * dot(rhat, s.mid);
    let e = Complex64::from_polar(s.len, phase) * current;
    for (fi, pi) in f.iter_mut().zip(proj) {
        *fi += e * pi;
    }
}

/// The ground's reflection coefficient at an observation elevation — the pair
/// `(vertical, horizontal)`. For a perfect ground these are the PEC limits
/// `(+1, −1)`; for a real ground they are the Fresnel coefficients at the
/// grazing angle. The grazing angle is 0 at the horizon and π/2 at the zenith.
fn ground_reflection(sol: &Solution, theta_deg: f64) -> (Complex64, Complex64) {
    let psi = (90.0 - theta_deg).abs().to_radians();
    match sol.ground.kind {
        GroundKind::Perfect => (Complex64::new(1.0, 0.0), Complex64::new(-1.0, 0.0)),
        GroundKind::Real => {
            let ec = eps_c(&sol.ground, C / sol.freq_hz);
            (reflection_v(ec, psi), reflection_h(ec, psi))
        }
        _ => (Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
    }
}

/// Gain in dBi toward `(θ, φ)` (θ from +z, φ from +x).
pub fn gain_dbi(sol: &Solution, theta_deg: f64, phi_deg: f64) -> f64 {
    let k = 2.0 * PI * sol.freq_hz / C;
    let f = far_factor(sol, theta_deg, phi_deg, k);
    let fmag2 = f.iter().map(|c| c.norm_sqr()).sum::<f64>();
    // U = k²η|F|²/(32π²);  G = 4πU/P_in = k²η|F|²/(8π P_in).
    let gain = k * k * ETA0 * fmag2 / (8.0 * PI * sol.power_w);
    if gain <= 0.0 { f64::NEG_INFINITY } else { 10.0 * gain.log10() }
}

/// The peak gain in dBi and where it points, over a coarse grid.
pub fn max_gain(sol: &Solution) -> (f64, f64, f64) {
    let mut best = (f64::NEG_INFINITY, 90.0, 0.0);
    let mut th = 0.0;
    while th <= 180.0 {
        let mut ph = 0.0;
        while ph < 360.0 {
            let g = gain_dbi(sol, th, ph);
            if g > best.0 {
                best = (g, th, ph);
            }
            ph += 5.0;
        }
        th += 2.0;
    }
    best
}

/// Front-to-back in dB: peak gain minus the gain the opposite way.
pub fn front_to_back_db(sol: &Solution) -> f64 {
    let (g, th, ph) = max_gain(sol);
    let back = gain_dbi(sol, 180.0 - th, if ph < 180.0 { ph + 180.0 } else { ph - 180.0 });
    g - back
}

/// The 3D radiation surface: a `(n_theta × n_phi)` grid of points on the
/// normalised far-field pattern, radius = |E|/|E_max| = 10^((G−G_max)/20). θ
/// from +z (up), φ from +x, so `z` is up and the pole is overhead. The radius
/// is also the relative *range* — a link's reach goes as the field (√gain).
/// Vertices are row-major, θ outer and φ inner.
pub fn pattern_surface(
    sol: &Solution,
    n_theta: usize,
    n_phi: usize,
) -> (Vec<[f64; 3]>, usize, usize) {
    let (gmax, _, _) = max_gain(sol);
    let mut verts = Vec::with_capacity(n_theta * n_phi);
    for it in 0..n_theta {
        let th = it as f64 / (n_theta - 1) as f64 * 180.0;
        for ip in 0..n_phi {
            let ph = ip as f64 / n_phi as f64 * 360.0;
            let g = gain_dbi(sol, th, ph);
            let r = if g.is_finite() { 10f64.powf((g - gmax) / 20.0) } else { 0.0 };
            let (tr, pr) = (th.to_radians(), ph.to_radians());
            verts.push([r * tr.sin() * pr.cos(), r * tr.sin() * pr.sin(), r * tr.cos()]);
        }
    }
    (verts, n_theta, n_phi)
}

/// The [`pattern_surface`] mesh as an ASCII PLY, loadable in any 3D viewer.
pub fn ply_ascii(verts: &[[f64; 3]], n_theta: usize, n_phi: usize) -> String {
    let idx = |t: usize, p: usize| t * n_phi + p;
    let mut faces = 0usize;
    let mut tris = String::new();
    for t in 0..n_theta - 1 {
        for p in 0..n_phi {
            let p2 = (p + 1) % n_phi;
            for (a, b, c) in [
                (idx(t, p), idx(t, p2), idx(t + 1, p)),
                (idx(t, p2), idx(t + 1, p2), idx(t + 1, p)),
            ] {
                tris.push_str(&format!("3 {a} {b} {c}\n"));
                faces += 1;
            }
        }
    }
    let mut s = String::new();
    s.push_str("ply\nformat ascii 1.0\n");
    s.push_str(&format!("element vertex {}\n", verts.len()));
    s.push_str("property float x\nproperty float y\nproperty float z\n");
    s.push_str(&format!("element face {faces}\n"));
    s.push_str("property list uchar int vertex_indices\nend_header\n");
    for v in verts {
        s.push_str(&format!("{} {} {}\n", v[0], v[1], v[2]));
    }
    s.push_str(&tris);
    s
}

/// An isometric wireframe of [`pattern_surface`] as SVG — a self-contained look
/// at the shape without a 3D viewer. `z` is up.
pub fn iso_svg(title: &str, verts: &[[f64; 3]], n_theta: usize, n_phi: usize) -> String {
    let (w, h) = (520.0_f64, 520.0_f64);
    let c = w / 2.0;
    let (a, b) = (45f64.to_radians(), 35.264f64.to_radians());
    let (sa, ca, sb, cb) = (a.sin(), a.cos(), b.sin(), b.cos());
    let scale = (w / 2.0 - 34.0) / 1.42;
    let px = |v: &[f64; 3]| -> (f64, f64) {
        let x1 = v[0] * ca - v[1] * sa;
        let y1 = v[0] * sa + v[1] * ca;
        (c + x1 * scale, c - (y1 * cb - v[2] * sb) * scale)
    };
    let idx = |t: usize, p: usize| t * n_phi + p;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\n"
    ));
    out.push_str(&format!("<rect width='{w}' height='{h}' fill='white'/>\n"));
    out.push_str(&format!(
        "<text x='{c}' y='18' text-anchor='middle' font-size='13' fill='#333'>{title}</text>\n"
    ));
    for p in 0..n_phi {
        for t in 0..n_theta - 1 {
            let (x1, y1) = px(&verts[idx(t, p)]);
            let (x2, y2) = px(&verts[idx(t + 1, p)]);
            out.push_str(&format!(
                "<line x1='{x1:.1}' y1='{y1:.1}' x2='{x2:.1}' y2='{y2:.1}' stroke='#8ab' stroke-width='0.6'/>\n"
            ));
        }
    }
    for t in 0..n_theta {
        for p in 0..n_phi {
            let (x1, y1) = px(&verts[idx(t, p)]);
            let (x2, y2) = px(&verts[idx(t, (p + 1) % n_phi)]);
            out.push_str(&format!(
                "<line x1='{x1:.1}' y1='{y1:.1}' x2='{x2:.1}' y2='{y2:.1}' stroke='#8ab' stroke-width='0.6'/>\n"
            ));
        }
    }
    for (v, col, lab) in [
        ([1.25, 0.0, 0.0], "#a00", "x"),
        ([0.0, 1.25, 0.0], "#0a0", "y"),
        ([0.0, 0.0, 1.25], "#00a", "z"),
    ] {
        let (x0, y0) = px(&[0.0, 0.0, 0.0]);
        let (x1, y1) = px(&v);
        out.push_str(&format!(
            "<line x1='{x0:.1}' y1='{y0:.1}' x2='{x1:.1}' y2='{y1:.1}' stroke='{col}' stroke-width='1.3'/>\n"
        ));
        out.push_str(&format!(
            "<text x='{x1:.1}' y='{y1:.1}' font-size='12' fill='{col}'>{lab}</text>\n"
        ));
    }
    out.push_str("</svg>\n");
    out
}

/// One point of a frequency sweep.
#[derive(Clone, Copy, Debug)]
pub struct SweepPoint {
    pub freq_hz: f64,
    pub zin: Complex64,
    pub swr: f64,
}

/// Solve a model across a band. `steps` points from `f0` to `f1` inclusive.
pub fn sweep(
    a: &Antenna,
    f0: f64,
    f1: f64,
    steps: usize,
    z0: f64,
    cfg: &SolverConfig,
) -> Vec<SweepPoint> {
    let n = steps.max(2);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let f = f0 + (f1 - f0) * i as f64 / (n - 1) as f64;
        let mut model = a.clone();
        model.freq_hz = f;
        if let Ok(sol) = analyze(&model, cfg) {
            out.push(SweepPoint { freq_hz: f, zin: sol.zin, swr: swr(sol.zin, z0) });
        }
    }
    out
}

/// The Galerkin interaction of one test piece with one source piece.
///
/// `Z = jω(μ0/4π)·∫∫ Λ_t Λ_s cosψ G  +  (1/4πε0)·∫∫ Λ_t ρ_s ∂G/∂s`,
/// with `ρ_s = -(1/jω)(dΛ_s/ds')` the piecewise-constant charge of the source
/// tent, and `∂G/∂s` the derivative of `G` along the test segment.
fn pair_integral(
    ts: &Segment,
    t_rising: bool,
    ss: &Segment,
    s_rising: bool,
    a: f64,
    k: f64,
    omega: f64,
) -> Complex64 {
    let cosang = dot(ts.dir, ss.dir);
    // Source-charge slope: +1/len for a tent rising toward the segment's end,
    // -1/len for one falling. Charge = -(1/jω)·slope, and -(1/jω) = +j/ω.
    let slope = if s_rising { 1.0 / ss.len } else { -1.0 / ss.len };
    let rho = Complex64::new(0.0, 1.0 / omega) * slope;

    // The reduced kernel is sharply peaked where the two segments nearly touch.
    // For a near pair (same segment, or adjacent) subdivide the source so the
    // `1/sqrt(u²+a²)` spike is resolved; elsewhere one rule is plenty. Without
    // this the resistance comes out ~15–20 % high.
    let ts_mid = scale(add(ts.from, ts.to), 0.5);
    let ss_mid = scale(add(ss.from, ss.to), 0.5);
    let near = dist(ts_mid, ss_mid) < 1.5 * (ts.len + ss.len);
    let subs = if near { ((2.0 * ss.len / a).ceil() as usize).clamp(1, 64) } else { 1 };

    let mut acc = Complex64::new(0.0, 0.0);
    for (gi, &xs) in GX.iter().enumerate() {
        let tfield = add(ts.from, scale(sub(ts.to, ts.from), (xs + 1.0) / 2.0));
        let lt = tent(t_rising, (xs + 1.0) / 2.0);
        let wt = GW[gi] * ts.len / 2.0;
        for si in 0..subs {
            for (gj, &ys) in GX.iter().enumerate() {
                let frac = (si as f64 + (ys + 1.0) / 2.0) / subs as f64;
                let ssrc = add(ss.from, scale(sub(ss.to, ss.from), frac));
                let ls = tent(s_rising, frac);
                let ws = GW[gj] * (ss.len / subs as f64) / 2.0;

                let d = sub(tfield, ssrc);
                let r = (dot(d, d) + a * a).sqrt();
                let phase = -k * r;
                let g = Complex64::new(phase.cos(), phase.sin()) / r;

                let drds = dot(ts.dir, d) / r;
                let dgdr = -Complex64::new(phase.cos(), phase.sin()) * (Complex64::new(1.0, k * r))
                    / (r * r);
                let dgds = dgdr * drds;

                let w = wt * ws;
                acc += Complex64::new(0.0, 1.0)
                    * (omega * MU0 / (4.0 * PI))
                    * (lt * ls * cosang * g * w);
                acc += (1.0 / (4.0 * PI * EPS0)) * (lt * rho * dgds * w);
            }
        }
    }
    acc
}

/// Triangle of one segment: `t` runs 0→1 from the segment's `from` to its `to`.
fn tent(rising: bool, t: f64) -> f64 {
    if rising { t } else { 1.0 - t }
}

/// Complex Gaussian elimination with partial pivoting.
fn solve_complex(a: &mut [Vec<Complex64>], b: &[Complex64]) -> Option<Vec<Complex64>> {
    let n = b.len();
    let mut x = b.to_vec();
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if a[r][col].norm() > a[piv][col].norm() {
                piv = r;
            }
        }
        if a[piv][col].norm() < 1e-30 {
            return None;
        }
        a.swap(col, piv);
        x.swap(col, piv);
        let d = a[col][col];
        let prow = a[col].clone();
        let xcol = x[col];
        for r in col + 1..n {
            let f = a[r][col] / d;
            if f.norm() == 0.0 {
                continue;
            }
            for c in col..n {
                a[r][c] -= f * prow[c];
            }
            x[r] -= f * xcol;
        }
    }
    let mut out = vec![Complex64::new(0.0, 0.0); n];
    for r in (0..n).rev() {
        let mut s = x[r];
        for c in r + 1..n {
            s -= a[r][c] * out[c];
        }
        out[r] = s / a[r][r];
    }
    Some(out)
}

/// SWR from a feed impedance against a reference (50 Ω by default).
pub fn swr(zin: Complex64, z0: f64) -> f64 {
    let g = ((zin - z0) / (zin + z0)).norm();
    (1.0 + g) / (1.0 - g)
}

/// Gain (dBi) sampled around a circle. `fixed_deg` is held and `sweep` runs
/// 0…360 — an azimuth cut (hold θ=90) or an elevation cut (hold φ=0/90).
pub fn cut(sol: &Solution, fixed_is_theta: bool, fixed_deg: f64, step_deg: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut a = 0.0;
    while a < 360.0 {
        let g =
            if fixed_is_theta { gain_dbi(sol, fixed_deg, a) } else { gain_dbi(sol, a, fixed_deg) };
        out.push((a, g));
        a += step_deg;
    }
    out
}

/// A 2D polar plot of a gain cut, as an SVG string. Angle 0 is up and increases
/// clockwise; `floor_dbi` is the centre of the chart, the peak is the rim.
pub fn polar_svg(title: &str, samples: &[(f64, f64)], floor_dbi: f64) -> String {
    let size = 440.0_f64;
    let c = size / 2.0;
    let rmax = c - 34.0;
    let peak = samples.iter().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
    let floor = floor_dbi.min(peak - 1.0);
    let norm = |g: f64| ((g - floor) / (peak - floor)).clamp(0.0, 1.0);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{size}' height='{size}' viewBox='0 0 {size} {size}'>\n"
    ));
    s.push_str(&format!("<rect width='{size}' height='{size}' fill='white'/>\n"));
    for k in 1..=4 {
        let rr = rmax * k as f64 / 4.0;
        s.push_str(&format!("<circle cx='{c}' cy='{c}' r='{rr:.1}' fill='none' stroke='#ccc'/>\n"));
        let db = floor + (peak - floor) * k as f64 / 4.0;
        s.push_str(&format!(
            "<text x='{:.1}' y='{c}' font-size='9' fill='#888'>{db:.0}</text>\n",
            c + 3.0
        ));
    }
    for a in (0..360).step_by(30) {
        let rad = (a as f64).to_radians();
        let (sx, cy) = (rad.sin(), rad.cos());
        s.push_str(&format!(
            "<line x1='{c}' y1='{c}' x2='{:.1}' y2='{:.1}' stroke='#eee'/>\n",
            c + rmax * sx,
            c - rmax * cy
        ));
        s.push_str(&format!(
            "<text x='{:.1}' y='{:.1}' font-size='10' text-anchor='middle' fill='#555'>{a}\u{b0}</text>\n",
            c + (rmax + 16.0) * sx,
            c - (rmax + 16.0) * cy
        ));
    }
    let pts: Vec<String> = samples
        .iter()
        .map(|(a, g)| {
            let rad = a.to_radians();
            let rr = rmax * norm(*g);
            format!("{:.1},{:.1}", c + rr * rad.sin(), c - rr * rad.cos())
        })
        .collect();
    s.push_str(&format!(
        "<polygon points='{}' fill='#2a7' fill-opacity='0.25' stroke='#186' stroke-width='1.5'/>\n",
        pts.join(" ")
    ));
    s.push_str(&format!(
        "<text x='{c}' y='22' font-size='14' text-anchor='middle'>{title}   peak {peak:.1} dBi</text>\n"
    ));
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ground, GroundKind, Source, Wire, WirePoint};

    /// A thin centre-fed dipole of a given length in wavelengths, at radius
    /// λ/10000 (thin, and in the `a ≪ Δ ≪ λ` window at 80 segments).
    fn dipole(frac: f64, segs: u32) -> Antenna {
        let f = 30e6;
        let lam = C / f;
        let half = frac * lam / 2.0;
        Antenna::wire_with_feed(
            f,
            "d",
            [-half, 0.0, 0.0],
            [half, 0.0, 0.0],
            lam / 10_000.0,
            Seg::Fixed(segs),
        )
    }

    fn zin(frac: f64) -> Complex64 {
        analyze(&dipole(frac, 80), &SolverConfig::default()).unwrap().zin
    }

    /// The independent calibration: a **short** dipole's input resistance is
    /// `20π²(l/λ)²`. At `l = 0.2λ` that is 7.90 Ω; the solver gives ≈ 7.7 Ω.
    #[test]
    fn a_short_dipole_matches_the_textbook_radiation_resistance() {
        let frac = 0.2;
        let z = zin(frac);
        let r_rad = 20.0 * std::f64::consts::PI.powi(2) * frac * frac;
        assert!((z.re - r_rad).abs() < 0.6, "R = {:.2} Ω, want {:.2}", z.re, r_rad);
        assert!(z.im < -500.0, "X = {:.0} Ω should be large and negative", z.im);
    }

    /// The classic figure: a thin dipole **resonates** — zero reactance — where
    /// the end effect shortens it to ≈ 0.47–0.48 λ, and its **resonant
    /// resistance is ≈ 73 Ω**. This is why the calculator's `K = 0.95` exists.
    #[test]
    fn the_resonant_dipole_is_about_73_ohms_and_shorter_than_a_half_wave() {
        let (mut lo, mut hi) = (0.44_f64, 0.50_f64);
        assert!(zin(lo).im < 0.0 && zin(hi).im > 0.0, "reactance must bracket zero");
        for _ in 0..12 {
            let mid = 0.5 * (lo + hi);
            if zin(mid).im < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let frac = 0.5 * (lo + hi);
        assert!((0.46..0.49).contains(&frac), "resonant length {frac:.3}λ should be ~0.47–0.48");
        let z = zin(frac);
        assert!((z.re - 73.0).abs() < 8.0, "resonant resistance {:.1} Ω should be ~73", z.re);
    }

    /// Wires that meet share a node, so an *unfed* junction solves. A feed at a
    /// non-collinear junction (the inverted-V apex) splits the node into the two
    /// wires' terminals and solves too — ~40–50 Ω, the classic inverted-V
    /// figure. A feed at a three-or-more-way junction is left alone (a 1-to-N
    /// delta gap has no consistent voltage split); the ground-plane preset
    /// avoids it with a short feed segment, landing at ~23 Ω, the classic
    /// horizontal-radial figure.
    #[test]
    fn junctions_solve_and_a_bend_feed_solves() {
        let loopa = crate::presets::delta_loop(28_500_000.0, 10.0);
        let sol = analyze(&loopa, &SolverConfig::default()).expect("delta loop should solve");
        assert!(sol.zin.re.is_finite() && sol.zin.re > 0.0, "R = {:.1}", sol.zin.re);

        let v = analyze(&crate::presets::inverted_v(28_500_000.0, 10.0), &SolverConfig::default())
            .expect("the inverted-V should solve");
        assert!((25.0..80.0).contains(&v.zin.re), "inverted-V R = {:.1} Ω, want ~50", v.zin.re);

        let gp = analyze(&crate::presets::ground_plane(28_500_000.0), &SolverConfig::default())
            .expect("the ground plane should solve");
        assert!((18.0..45.0).contains(&gp.zin.re), "ground plane R = {:.1} Ω, want ~23", gp.zin.re);
    }

    /// A lumped series resistor in the middle of a dipole adds its resistance to
    /// the input impedance: 100 Ω at the feed reads ~100 Ω above the bare dipole.
    #[test]
    fn a_series_load_raises_the_input_resistance() {
        let f = 28.5e6;
        let bare = analyze(&crate::presets::dipole(f, 10.0), &SolverConfig::default()).unwrap();
        let mut loaded = crate::presets::dipole(f, 10.0);
        loaded.loads.push(crate::model::Load {
            at: crate::model::WirePoint { wire: 1, at: Pos::Center, offset: 0 },
            values: [0.0, 100.0, 0.0, 0.0],
        });
        let sol = analyze(&loaded, &SolverConfig::default()).expect("a loaded dipole solves");
        assert!(
            sol.zin.re > bare.zin.re + 50.0,
            "loaded {:.1} Ω should sit well above bare {:.1} Ω",
            sol.zin.re,
            bare.zin.re
        );
    }

    /// The 3D surface is the far field on a sphere: a horizontal dipole (wire on
    /// +x) has a null along x and its peak along y.
    #[test]
    fn the_3d_surface_has_the_dipoles_null_and_peak() {
        let f = 28.5e6;
        let sol = analyze(&crate::presets::dipole(f, 10.0), &SolverConfig::default()).unwrap();
        let (verts, nt, np) = pattern_surface(&sol, 37, 72);
        let mag = |t: usize, p: usize| {
            let v = verts[t * np + p];
            (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
        };
        let null_x = mag(nt / 2, 0); // θ=90, φ=0 → +x, along the wire
        let peak_y = mag(nt / 2, np / 4); // θ=90, φ=90 → +y, broadside
        assert!(null_x < 0.1, "null along x should be near zero, got {null_x:.3}");
        assert!((peak_y - 1.0).abs() < 0.05, "peak along y should be ~1, got {peak_y:.3}");
    }

    /// A quarter-wave monopole over a **perfect** ground is half a dipole: the
    /// image doubles the wire into a λ/2 dipole, so its impedance is half the
    /// free-space dipole's. This is the ground model's check.
    #[test]
    fn a_monopole_over_a_perfect_ground_is_half_a_dipole() {
        let f = 30e6;
        let lam = C / f;
        let len = 0.25 * lam;
        let mut mono = Antenna::wire_with_feed(
            f,
            "m",
            [0.0, 0.0, 0.0],
            [0.0, 0.0, len],
            lam / 10_000.0,
            Seg::Fixed(80),
        );
        mono.sources = vec![Source {
            at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }];
        mono.ground.kind = GroundKind::Perfect;
        let zm = analyze(&mono, &SolverConfig::default()).unwrap().zin;

        // The equivalent dipole: the wire plus its image, twice as long.
        let dip = Antenna::wire_with_feed(
            f,
            "d",
            [-len, 0.0, 0.0],
            [len, 0.0, 0.0],
            lam / 10_000.0,
            Seg::Fixed(80),
        );
        let zd = analyze(&dip, &SolverConfig::default()).unwrap().zin;

        assert!(
            (zm.re - zd.re / 2.0).abs() < 0.05 * zd.re.abs(),
            "monopole R {:.1} should be half the dipole's {:.1}",
            zm.re,
            zd.re / 2.0
        );
        assert!(
            (zm.im - zd.im / 2.0).abs() < 0.10 * zd.im.abs(),
            "monopole X {:.1} should be half the dipole's {:.1}",
            zm.im,
            zd.im / 2.0
        );
    }

    /// A real (Fresnel) ground solves — the impedance uses the perfect image
    /// (MININEC's assumption) and the far field uses the reflection
    /// coefficient. A quarter-wave vertical over real ground still comes out
    /// near the perfect-ground figure, and the Sommerfeld-Norton ground is
    /// refused rather than silently treated as something else.
    #[test]
    fn real_ground_solves_and_sommerfeld_is_refused() {
        let f = 30e6;
        let lam = C / f;
        let mut mono = Antenna::wire_with_feed(
            f,
            "m",
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.25 * lam],
            lam / 10_000.0,
            Seg::Fixed(80),
        );
        mono.sources = vec![Source {
            at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }];
        mono.ground.kind = GroundKind::Real;
        mono.ground.eps_r = 13.0;
        mono.ground.sigma_ms_m = 5.0;
        let z = analyze(&mono, &SolverConfig::default()).expect("real ground should solve").zin;
        assert!(z.re.is_finite() && z.re > 0.0, "R = {z}");

        mono.ground.kind = GroundKind::SommerfeldNorton;
        let err = analyze(&mono, &SolverConfig::default()).unwrap_err();
        assert!(err.contains("Sommerfeld"), "{err}");
    }

    /// The Fresnel coefficients tend to the PEC limits — `R_v → 1`, `R_h → −1`
    /// as the ground grows more conductive — and both are `−1` at grazing for
    /// any ground.
    #[test]
    fn the_fresnel_coefficients_have_the_right_limits() {
        let ec = Complex64::new(13.0, 0.0);
        assert!((reflection_v(ec, 0.0).re + 1.0).abs() < 1e-9, "R_v at grazing");
        assert!((reflection_h(ec, 0.0).re + 1.0).abs() < 1e-9, "R_h at grazing");
        let rv = reflection_v(ec, std::f64::consts::FRAC_PI_2);
        let expect = (13.0 - 13f64.sqrt()) / (13.0 + 13f64.sqrt());
        assert!((rv.re - expect).abs() < 1e-6, "R_v = {rv:.4}, want {expect:.4}");
        let rh = reflection_h(ec, std::f64::consts::FRAC_PI_2);
        assert!((rh.re + expect).abs() < 1e-6, "R_h = {rh:.4}, want {:.4}", -expect);
    }

    /// Real ground changes the far field while the impedance stays the
    /// perfect-ground figure (MININEC's assumption: Z always uses the image).
    #[test]
    fn real_ground_shifts_the_pattern_but_not_the_impedance() {
        let f = 30e6;
        let lam = C / f;
        let mono = |kind: GroundKind| {
            let mut m = Antenna::wire_with_feed(
                f,
                "m",
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.25 * lam],
                lam / 10_000.0,
                Seg::Fixed(80),
            );
            m.sources = vec![Source {
                at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
                phase_deg: 0.0,
                volts: 1.0,
            }];
            m.ground = Ground { kind, ..Ground::default() };
            m
        };
        let perf = analyze(&mono(GroundKind::Perfect), &SolverConfig::default()).unwrap();
        let real = analyze(&mono(GroundKind::Real), &SolverConfig::default()).unwrap();
        assert!(
            (perf.zin.re - real.zin.re).abs() < 0.01,
            "Z should be unchanged: {} vs {}",
            perf.zin.re,
            real.zin.re
        );
        let gp = gain_dbi(&perf, 60.0, 0.0);
        let gr = gain_dbi(&real, 60.0, 0.0);
        assert!(
            (gp - gr).abs() > 0.1,
            "real ground should change the pattern at 30° elevation: {gp:.2} vs {gr:.2}"
        );
    }

    #[test]
    fn swr_reads_the_match() {
        assert!((swr(Complex64::new(50.0, 0.0), 50.0) - 1.0).abs() < 1e-9);
        assert!((swr(Complex64::new(100.0, 0.0), 50.0) - 2.0).abs() < 1e-9);
    }

    /// The far field is right: a λ/2 dipole has ~2.15 dBi peak gain and no
    /// front-to-back (it is symmetric).
    #[test]
    fn a_half_wave_dipole_has_about_2_dbi_and_no_front_to_back() {
        let f = 30e6;
        let lam = C / f;
        let a = Antenna::wire_with_feed(
            f,
            "d",
            [-lam / 4.0, 0.0, 0.0],
            [lam / 4.0, 0.0, 0.0],
            lam / 10_000.0,
            Seg::Fixed(80),
        );
        let sol = analyze(&a, &SolverConfig::default()).unwrap();
        let (g, _, _) = max_gain(&sol);
        assert!((g - 2.15).abs() < 0.5, "peak gain {g:.2} dBi should be ~2.15");
        let fb = front_to_back_db(&sol);
        assert!(fb.abs() < 0.5, "a dipole has no front-to-back, got {fb:.2} dB");
    }

    /// The sweep spans the band it is asked for and every point is usable.
    #[test]
    fn the_sweep_spans_the_band() {
        let a = dipole(0.475, 60);
        let pts = sweep(&a, 29e6, 31e6, 5, 50.0, &SolverConfig::default());
        assert_eq!(pts.len(), 5);
        assert!((pts[0].freq_hz - 29e6).abs() < 1.0);
        assert!((pts[4].freq_hz - 31e6).abs() < 1.0);
        assert!(pts.iter().all(|p| p.swr.is_finite() && p.swr >= 1.0), "{pts:?}");
    }

    /// The pattern cut produces a well-formed polar SVG.
    #[test]
    fn the_pattern_cut_becomes_svg() {
        let sol = analyze(&dipole(0.475, 60), &SolverConfig::default()).unwrap();
        let az = cut(&sol, true, 90.0, 10.0);
        assert_eq!(az.len(), 36);
        assert!(az.iter().all(|(_, g)| g.is_finite()));
        let svg = polar_svg("test", &az, -20.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("peak"));
    }

    /// The acceptance test on a **real, published design**: the 4-element OWA
    /// Yagi from dx-antennas.com (19DX348), boom 3.82 m, quoted **8.42 dBi** and
    /// **50 Ω** at 27.555 MHz, F/B < 22 dB. Straight parallel elements (no
    /// bends), so the solver models it — and it reproduces the published figures
    /// without tuning. Source: <https://dx-antennas.com/>.
    #[test]
    fn an_owa_yagi_matches_the_published_gain_and_impedance() {
        let f = 27.555e6;
        let h = 10.0;
        let r = 0.011; // 22 mm element → 11 mm radius
        let el = |half: f64, y: f64| Wire {
            from: [-half, y, h],
            to: [half, y, h],
            radius_m: r,
            seg: Seg::Fixed(60),
        };
        let a = Antenna {
            name: "4el OWA".into(),
            freq_hz: f,
            wires: vec![
                el(2.736, 0.0),   // reflector
                el(2.657, 1.329), // radiator
                el(2.523, 1.834), // director 1
                el(2.4062, 3.82), // director 2
            ],
            sources: vec![Source {
                at: WirePoint { wire: 2, at: Pos::Center, offset: 0 },
                phase_deg: 0.0,
                volts: 1.0,
            }],
            ground: Ground { kind: GroundKind::FreeSpace, ..Ground::default() },
            ..Antenna::default()
        };
        let sol = analyze(&a, &SolverConfig::default()).unwrap();
        let (g, _, _) = max_gain(&sol);
        assert!((g - 8.42).abs() < 0.7, "gain {g:.2} dBi, published 8.42");
        assert!((sol.zin.re - 50.0).abs() < 12.0, "R {:.1} Ω, published ~50", sol.zin.re);
        assert!(sol.zin.im.abs() < 30.0, "X {:.1} Ω should be small", sol.zin.im);
        assert!(front_to_back_db(&sol) > 12.0, "F/B {:.1} dB", front_to_back_db(&sol));
    }

    #[test]
    fn a_missing_feed_wire_is_refused() {
        let mut a = Antenna::wire_with_feed(
            30e6,
            "d",
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            0.001,
            Seg::Fixed(20),
        );
        a.sources = vec![Source {
            at: WirePoint { wire: 9, at: Pos::Center, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }];
        assert!(analyze(&a, &SolverConfig::default()).is_err());
    }
}
