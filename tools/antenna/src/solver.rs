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
//! the dense complex system `[Z][I] = [V]`. The feed is a delta gap at a node:
//! `V` on that basis function alone, so `Zin = V / I_feed`.
//!
//! **Junctions are handled by sharing the node.** Wires whose ends meet within a
//! tolerance collapse to one graph node with one current unknown, whose basis is
//! the tent over *every* segment incident on it — so a bend, a T or a five-way
//! ground-plane junction all carry Kirchhoff's current law at the node for free.
//! A free end (one incident segment) carries no current and is not an unknown
//! unless it is fed.
//!
//! **A feed must sit on a straight-through node** (or a free end). The delta gap
//! is a gap in a straight wire; feeding a bend or a multi-way junction has no
//! single gap direction and this solver gets it wrong, so such a design is
//! **refused, not solved wrong** — the inverted-V apex and the ground plane's
//! radial star are the cases that wait on a proper bend-feed model.
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

use num_complex::Complex64;
use std::f64::consts::PI;

pub use crate::C;
use crate::model::{Antenna, Pos, Seg};

const MU0: f64 = 1.256_637_062_12e-6;
const EPS0: f64 = 8.854_187_812_8e-12;

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
    /// The graph nodes of each wire, in order (for locating the feed).
    wire_nodes: Vec<Vec<usize>>,
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
    let mut wire_nodes: Vec<Vec<usize>> = Vec::new();
    for w in &a.wires {
        let len = dist(w.from, w.to);
        if len <= 0.0 {
            wire_nodes.push(Vec::new());
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
        for i in 0..n {
            let from = node_pts[ids[i]];
            let to = node_pts[ids[i + 1]];
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
        wire_nodes.push(ids);
    }
    let mut incident = vec![Vec::new(); node_pts.len()];
    for (si, s) in segs.iter().enumerate() {
        incident[s.a].push(si);
        incident[s.b].push(si);
    }
    Mesh { node_pts, segs, incident, wire_nodes }
}

// ── the solve ────────────────────────────────────────────────────────────────

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
}

/// Solve one model at its own frequency.
pub fn analyze(a: &Antenna, cfg: &SolverConfig) -> Result<Solution, String> {
    if a.freq_hz <= 0.0 {
        return Err("the design has no frequency".into());
    }
    if a.sources.is_empty() {
        return Err("the design has no source".into());
    }
    let m = build_mesh(a, cfg);
    if m.segs.is_empty() {
        return Err("the design has no wires".into());
    }

    // The feed node.
    let src = &a.sources[0];
    let fw = (src.at.wire as usize).saturating_sub(1);
    if fw >= m.wire_nodes.len() || m.wire_nodes[fw].len() < 2 {
        return Err(format!("the source names wire {}, which is not there", fw + 1));
    }
    let n = m.wire_nodes[fw].len() - 1;
    let base = match src.at.at {
        Pos::Begin => 0isize,
        Pos::Center => (n as isize) / 2,
        Pos::End => n as isize,
    };
    let li = (base + src.at.offset as isize).clamp(0, n as isize) as usize;
    let feed_node = m.wire_nodes[fw][li];

    // A delta-gap feed is a gap in a *straight* wire. Feeding a bend or a
    // multi-way junction is a different model (the gap has no single direction)
    // and this solver gets it wrong — refuse rather than answer wrongly.
    let incidents = &m.incident[feed_node];
    let feed_ok = match incidents.len() {
        1 => true, // a free end: a series source, fine
        2 => {
            let (s0, s1) = (incidents[0], incidents[1]);
            dot(m.segs[s0].dir, m.segs[s1].dir).abs() > 0.9999
        }
        _ => false,
    };
    if !feed_ok {
        return Err(
            "the feed sits where wires meet at an angle — feeding a bend or junction is not \
             modelled yet, so this shape cannot be solved"
                .into(),
        );
    }

    // Unknowns: every node with two or more segments (a bend, junction or a
    // through-connection), plus the feed node wherever it lands.
    let mut unknowns: Vec<usize> = Vec::new();
    let mut index = vec![None; m.node_pts.len()];
    for (node, slot) in index.iter_mut().enumerate() {
        if m.incident[node].len() >= 2 || node == feed_node {
            *slot = Some(unknowns.len());
            unknowns.push(node);
        }
    }
    if unknowns.is_empty() {
        return Err("no node to solve for".into());
    }
    let feed_unknown = index[feed_node].expect("the feed node was added");

    let k = 2.0 * PI * a.freq_hz / C;
    let omega = 2.0 * PI * a.freq_hz;
    let nu = unknowns.len();

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
                }
            }
            z[mi][ni] = acc;
        }
    }

    let mut rhs = vec![Complex64::new(0.0, 0.0); nu];
    rhs[feed_unknown] = Complex64::new(src.volts, 0.0);

    let currents = solve_complex(&mut z, &rhs).ok_or("the interaction matrix is singular")?;
    let i_feed = currents[feed_unknown];
    if i_feed.norm() == 0.0 {
        return Err("the feed current came out zero".into());
    }
    let zin = Complex64::new(src.volts, 0.0) / i_feed;
    Ok(Solution { freq_hz: a.freq_hz, currents, unknowns, feed_unknown, zin })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Source, WirePoint};

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

    /// Wires that meet share a node, so a junction solves. The delta loop — fed
    /// on a straight-through node (the middle of the bottom side) — gives a
    /// finite, physical impedance. A feed where wires meet at an *angle* (the
    /// inverted-V apex, the ground plane base) is a different model and is
    /// refused, not solved wrong.
    #[test]
    fn junctions_solve_and_a_bend_feed_is_refused() {
        let loopa = crate::presets::delta_loop(28_500_000.0, 10.0);
        let sol = analyze(&loopa, &SolverConfig::default()).expect("delta loop should solve");
        assert!(sol.zin.re.is_finite() && sol.zin.re > 0.0, "R = {:.1}", sol.zin.re);

        for a in [
            crate::presets::inverted_v(28_500_000.0, 10.0),
            crate::presets::ground_plane(28_500_000.0),
        ] {
            let err = analyze(&a, &SolverConfig::default()).unwrap_err();
            assert!(err.contains("bend"), "{}: {err}", a.name);
        }
    }

    #[test]
    fn swr_reads_the_match() {
        assert!((swr(Complex64::new(50.0, 0.0), 50.0) - 1.0).abs() < 1e-9);
        assert!((swr(Complex64::new(100.0, 0.0), 50.0) - 2.0).abs() < 1e-9);
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
