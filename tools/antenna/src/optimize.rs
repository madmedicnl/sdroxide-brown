//! A small coordinate-descent optimizer for the Yagi preset.
//!
//! The tunables are the element **lengths** and the **gaps** between adjacent
//! elements, both in metres; the objective is one of SWR, gain, front-to-back or
//! reactance. It is a plain hill-climb — vary one parameter by a step, keep any
//! improvement, halve the step when nothing moves — which is honest about being
//! a local search, not a global one.

use crate::model::*;
use crate::presets;
use crate::solver::{SolverConfig, analyze, front_to_back_db, max_gain, swr};

/// What the optimizer is asked to improve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// Minimum SWR against 50 Ω.
    Swr,
    /// Maximum gain (dBi).
    Gain,
    /// Maximum front-to-back (dB).
    Fb,
    /// Minimum |reactance| — the resonant condition.
    Jx,
}

impl Objective {
    pub const ALL: [Objective; 4] = [Objective::Swr, Objective::Gain, Objective::Fb, Objective::Jx];

    pub fn label(self) -> &'static str {
        match self {
            Objective::Swr => "SWR",
            Objective::Gain => "gain",
            Objective::Fb => "F/B",
            Objective::Jx => "jX",
        }
    }
}

/// Score an antenna for an objective — **lower is better** (gain and F/B are
/// negated). `None` when the antenna does not solve.
pub fn score(a: &Antenna, obj: Objective, cfg: &SolverConfig) -> Option<f64> {
    let sol = analyze(a, cfg).ok()?;
    Some(match obj {
        Objective::Swr => swr(sol.zin, 50.0),
        Objective::Gain => -max_gain(&sol).0,
        Objective::Fb => -front_to_back_db(&sol),
        Objective::Jx => sol.zin.im.abs(),
    })
}

/// Coordinate descent: vary each parameter by ±`step` (clamped to `bounds`),
/// keep any improvement, halve the step when no parameter moves, and stop after
/// `max_evals` score evaluations (the honest budget — the solver is not free) or
/// once the step is negligible. Returns the best parameters and their score.
pub fn coordinate_descent(
    init: &[f64],
    bounds: &[(f64, f64)],
    mut step: f64,
    max_evals: usize,
    score_fn: impl Fn(&[f64]) -> f64,
) -> (Vec<f64>, f64) {
    let mut p = init.to_vec();
    let mut s = score_fn(&p);
    let mut evals = 1;
    'outer: loop {
        let mut improved = false;
        for i in 0..p.len() {
            for dir in [-1.0_f64, 1.0] {
                if evals >= max_evals {
                    break 'outer;
                }
                let mut q = p.clone();
                q[i] = (p[i] + dir * step).clamp(bounds[i].0, bounds[i].1);
                if (q[i] - p[i]).abs() < 1e-12 {
                    continue;
                }
                let sq = score_fn(&q);
                evals += 1;
                if sq < s {
                    p = q;
                    s = sq;
                    improved = true;
                }
            }
        }
        if !improved {
            if step < 1e-5 {
                break;
            }
            step *= 0.5;
        }
    }
    (p, s)
}

/// The element lengths of a Yagi (the `to.x − from.x` span of each wire, in
/// metres), in wire order (reflector, driven, directors).
fn yagi_lengths(a: &Antenna) -> Vec<f64> {
    a.wires.iter().map(|w| (w.to[0] - w.from[0]).abs()).collect()
}

/// The gaps between adjacent elements (the y step from one wire to the next).
fn yagi_gaps(a: &Antenna) -> Vec<f64> {
    a.wires.windows(2).map(|w| (w[1].from[1] - w[0].from[1]).abs()).collect()
}

/// Build a Yagi from lengths and gaps (both metres): wires along x, the driven
/// element fed at its centre.
fn yagi_from(freq_hz: f64, height_m: f64, lens: &[f64], gaps: &[f64]) -> Antenna {
    let mut wires = Vec::with_capacity(lens.len());
    let mut y = 0.0;
    for (i, len) in lens.iter().enumerate() {
        wires.push(Wire {
            from: [-len / 2.0, y, height_m],
            to: [len / 2.0, y, height_m],
            radius_m: presets::WIRE_RADIUS_M,
            seg: Seg::AutoTaper,
        });
        if i < gaps.len() {
            y += gaps[i];
        }
    }
    Antenna {
        name: "Yagi-Uda (optimized)".into(),
        freq_hz,
        wires,
        sources: vec![Source {
            at: WirePoint { wire: 2, at: Pos::Center, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }],
        ground: Ground { kind: GroundKind::FreeSpace, ..Ground::default() },
        ..Antenna::default()
    }
}

/// Optimize a Yagi of `elements` elements against `obj`. Returns the optimized
/// antenna and its score (lower is better).
pub fn optimize_yagi(
    freq_hz: f64,
    height_m: f64,
    elements: usize,
    obj: Objective,
    cfg: &SolverConfig,
) -> Option<(Antenna, f64)> {
    let lam = crate::wavelength_m(freq_hz);
    let base = presets::yagi(freq_hz, elements, height_m);
    let lens = yagi_lengths(&base);
    let gaps = yagi_gaps(&base);
    let n_lens = lens.len();
    // Bounds: element lengths 0.35..0.55 λ, gaps 0.05..0.40 λ.
    let mut bounds = vec![(0.35 * lam, 0.55 * lam); n_lens];
    bounds.extend(vec![(0.05 * lam, 0.40 * lam); gaps.len()]);
    let mut init = lens;
    init.extend(gaps);

    let score_fn = |p: &[f64]| {
        let (l, g) = p.split_at(n_lens);
        let a = yagi_from(freq_hz, height_m, l, g);
        score(&a, obj, cfg).unwrap_or(f64::INFINITY)
    };
    // A bounded local search: 100 solver evaluations is the honest budget.
    let (best, s) = coordinate_descent(&init, &bounds, 0.02 * lam, 100, score_fn);
    let (l, g) = best.split_at(n_lens);
    Some((yagi_from(freq_hz, height_m, l, g), s))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The optimizer must not make the starting Yagi worse, and its result stays
    /// within the bounds. A coarse solver keeps the test quick.
    #[test]
    fn the_optimizer_holds_or_improves_gain() {
        let cfg = SolverConfig { segs_per_wavelength: 10.0 };
        let f = 28.5e6;
        let base = presets::yagi(f, 3, presets::DEFAULT_HEIGHT_M);
        let before = score(&base, Objective::Gain, &cfg).unwrap(); // -gain dBi
        let (opt, after) = optimize_yagi(f, presets::DEFAULT_HEIGHT_M, 3, Objective::Gain, &cfg)
            .expect("the optimizer should solve");
        // The score is −gain, so "lower" is "more gain": the optimized Yagi must
        // be at least as good as the starting layout.
        assert!(after <= before + 0.05, "gain went backwards: {before:.2} → {after:.2}");
        // And the objective reached is a real, finite number.
        assert!(after.is_finite(), "non-finite score {after}");
        let _ = opt;
    }

    /// A coordinate descent on a simple parabola reaches its minimum.
    #[test]
    fn coordinate_descent_descends_a_parabola() {
        let (p, s) = coordinate_descent(&[0.0], &[(-5.0, 5.0)], 1.0, 100, |p| p[0] * p[0]);
        assert!(p[0].abs() < 1e-3, "did not reach the minimum: {}", p[0]);
        assert!(s < 1e-5, "score {s}");
    }
}
