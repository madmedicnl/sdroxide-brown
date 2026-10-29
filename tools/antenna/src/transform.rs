//! Scaling a design and stacking copies into an array (MMANA's "Wire scale" /
//! "Make stack"). Pure geometry: coordinates move, feeds and loads keep their
//! wires. A stacked array carries one source per copy, fed in phase.

use crate::model::Antenna;

/// Scale a design's geometry by `factor` — a frequency change (`f_old / f_new`)
/// or a size change. Wire coordinates scale; the radius and feed stay.
pub fn scale(a: &Antenna, factor: f64) -> Antenna {
    let mut out = a.clone();
    for w in &mut out.wires {
        w.from = [w.from[0] * factor, w.from[1] * factor, w.from[2] * factor];
        w.to = [w.to[0] * factor, w.to[1] * factor, w.to[2] * factor];
    }
    out
}

/// Stack `n` copies of a design into an array, `spacing_m` apart, along the
/// vertical (z) axis if `vertical`, else the y axis. Each copy keeps its own
/// feed, so the result is a multi-source array (solved in phase).
pub fn stack(a: &Antenna, n: usize, spacing_m: f64, vertical: bool) -> Antenna {
    let nw = a.wires.len();
    let mut out = a.clone();
    out.name = format!("{} ×{n}", a.name);
    out.wires.clear();
    out.sources.clear();
    out.loads.clear();
    for i in 0..n {
        let off = i as f64 * spacing_m;
        for w in &a.wires {
            let mut w2 = w.clone();
            if vertical {
                w2.from[2] += off;
                w2.to[2] += off;
            } else {
                w2.from[1] += off;
                w2.to[1] += off;
            }
            out.wires.push(w2);
        }
        for s in &a.sources {
            let mut s2 = s.clone();
            s2.at.wire += (i * nw) as u32;
            out.sources.push(s2);
        }
        for l in &a.loads {
            let mut l2 = l.clone();
            l2.at.wire += (i * nw) as u32;
            out.loads.push(l2);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets;

    /// Scaling a dipole by 2 doubles its length; scaling by the frequency ratio
    /// re-tunes it.
    #[test]
    fn scaling_moves_the_wires() {
        let a = presets::dipole(27e6, 10.0);
        let big = scale(&a, 2.0);
        let half_span = (big.wires[0].to[0] - big.wires[0].from[0]).abs();
        let orig_span = (a.wires[0].to[0] - a.wires[0].from[0]).abs();
        assert!((half_span - 2.0 * orig_span).abs() < 1e-9, "span {half_span} vs {orig_span}");
    }

    /// A 2-stack doubles the wires and the sources, offset along the chosen axis.
    #[test]
    fn stacking_duplicates_wires_and_sources() {
        let a = presets::dipole(27e6, 10.0);
        let nw = a.wires.len();
        let ns = a.sources.len();
        let st = stack(&a, 2, 1.5, true);
        assert_eq!(st.wires.len(), 2 * nw);
        assert_eq!(st.sources.len(), 2 * ns);
        // The second copy is one spacing higher, its source on the shifted wire.
        let z0 = a.wires[0].from[2];
        assert!((st.wires[nw].from[2] - (z0 + 1.5)).abs() < 1e-9);
        assert_eq!(st.sources[1].at.wire, a.sources[0].at.wire + nw as u32);
    }

    /// A 2-stack of Yagis solves (multi-source) and gains over a single Yagi.
    #[test]
    fn a_stacked_yagi_solves_and_gains() {
        use crate::solver::{SolverConfig, analyze, max_gain};
        let f = 28.5e6;
        let cfg = SolverConfig { segs_per_wavelength: 20.0 };
        let single = presets::yagi(f, 3, presets::DEFAULT_HEIGHT_M);
        let g1 = max_gain(&analyze(&single, &cfg).unwrap()).0;
        let stacked = stack(&single, 2, 0.5 * crate::wavelength_m(f), true);
        let g2 = max_gain(&analyze(&stacked, &cfg).unwrap()).0;
        assert!(g2 > g1 + 1.0, "stacked {g2:.1} should beat single {g1:.1} by ~3 dB");
    }
}
