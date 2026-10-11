//! CB/SWL shape presets — the wire layouts the calculator and the editor start
//! from. Each returns a complete [`Antenna`] (geometry + a feed + ground).
//!
//! They use the same closed-form lengths as the calculator half, so a preset and
//! the "how long is my dipole" answer cannot disagree. The dimensions are
//! starting points; the solver is what makes them exact.

use crate::model::*;
use crate::{K, wavelength_m};

/// A typical thin wire radius, metres (1 mm).
pub const WIRE_RADIUS_M: f64 = 0.001;
/// A typical height above ground for the horizontal presets, metres.
pub const DEFAULT_HEIGHT_M: f64 = 10.0;

fn center_feed(wire: u32) -> Source {
    Source { at: WirePoint { wire, at: Pos::Center, offset: 0 }, phase_deg: 0.0, volts: 1.0 }
}

/// A centre-fed horizontal half-wave dipole at `height_m`.
pub fn dipole(freq_hz: f64, height_m: f64) -> Antenna {
    let total = K * wavelength_m(freq_hz) / 2.0;
    Antenna::wire_with_feed(
        freq_hz,
        "Half-wave dipole",
        [-total / 2.0, 0.0, height_m],
        [total / 2.0, 0.0, height_m],
        WIRE_RADIUS_M,
        Seg::AutoTaper,
    )
}

/// An inverted-V: two legs from one apex, at roughly 90° between them.
pub fn inverted_v(freq_hz: f64, apex_height_m: f64) -> Antenna {
    let leg = K * wavelength_m(freq_hz) / 4.0;
    let reach = leg * std::f64::consts::FRAC_1_SQRT_2; // 45° from vertical
    let drop = reach;
    let mut a = Antenna {
        name: "Inverted-V".into(),
        freq_hz,
        wires: vec![
            Wire {
                from: [0.0, 0.0, apex_height_m],
                to: [-reach, 0.0, apex_height_m - drop],
                radius_m: WIRE_RADIUS_M,
                seg: Seg::AutoTaper,
            },
            Wire {
                from: [0.0, 0.0, apex_height_m],
                to: [reach, 0.0, apex_height_m - drop],
                radius_m: WIRE_RADIUS_M,
                seg: Seg::AutoTaper,
            },
        ],
        // The feed is the apex, where the two legs meet.
        sources: vec![Source {
            at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }],
        ..Antenna::default()
    };
    a.ground = Ground { kind: GroundKind::Real, ..Ground::default() };
    a
}

/// A quarter-wave vertical ("stick") over ground.
pub fn vertical(freq_hz: f64) -> Antenna {
    let len = K * wavelength_m(freq_hz) / 4.0;
    let mut a = Antenna::wire_with_feed(
        freq_hz,
        "Quarter-wave vertical (stick)",
        [0.0, 0.0, 0.0],
        [0.0, 0.0, len],
        WIRE_RADIUS_M,
        Seg::AutoTaper,
    );
    a.sources = vec![Source {
        at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
        phase_deg: 0.0,
        volts: 1.0,
    }];
    a.ground = Ground { kind: GroundKind::Real, ..Ground::default() };
    a
}

/// A ground plane: a quarter-wave radiator with four quarter-wave radials.
pub fn ground_plane(freq_hz: f64) -> Antenna {
    let lam = wavelength_m(freq_hz);
    let rad = K * lam / 4.0;
    let radial = lam / 4.0;
    let mut wires = vec![Wire {
        from: [0.0, 0.0, 0.0],
        to: [0.0, 0.0, rad],
        radius_m: WIRE_RADIUS_M,
        seg: Seg::AutoTaper,
    }];
    let dirs = [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]];
    for d in dirs {
        wires.push(Wire {
            from: [0.0, 0.0, 0.0],
            to: [d[0] * radial, d[1] * radial, 0.0],
            radius_m: WIRE_RADIUS_M,
            seg: Seg::AutoTaper,
        });
    }
    Antenna {
        name: "Ground plane (4 radials)".into(),
        freq_hz,
        wires,
        sources: vec![Source {
            at: WirePoint { wire: 1, at: Pos::Begin, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }],
        ground: Ground { kind: GroundKind::Perfect, radials: 4, ..Ground::default() },
        ..Antenna::default()
    }
}

/// A full-wave equilateral delta loop, fed at the centre of the bottom side.
pub fn delta_loop(freq_hz: f64, height_m: f64) -> Antenna {
    let perimeter = 1.02 * wavelength_m(freq_hz);
    let s = perimeter / 3.0;
    let hh = 0.866_025_4 * s; // equilateral height
    let a = [-s / 2.0, 0.0, height_m];
    let m = [0.0, 0.0, height_m];
    let b = [s / 2.0, 0.0, height_m];
    let c = [0.0, 0.0, height_m + hh];
    let wires = vec![
        Wire { from: a, to: m, radius_m: WIRE_RADIUS_M, seg: Seg::AutoTaper },
        Wire { from: m, to: b, radius_m: WIRE_RADIUS_M, seg: Seg::AutoTaper },
        Wire { from: b, to: c, radius_m: WIRE_RADIUS_M, seg: Seg::AutoTaper },
        Wire { from: c, to: a, radius_m: WIRE_RADIUS_M, seg: Seg::AutoTaper },
    ];
    Antenna {
        name: "Full-wave delta loop".into(),
        freq_hz,
        wires,
        // The feed is the middle of the bottom side — the junction of wires 1 & 2.
        sources: vec![Source {
            at: WirePoint { wire: 2, at: Pos::Begin, offset: 0 },
            phase_deg: 0.0,
            volts: 1.0,
        }],
        ground: Ground { kind: GroundKind::Real, ..Ground::default() },
        ..Antenna::default()
    }
}

/// A Yagi-Uda starting layout with `elements` total (reflector, driven, …).
pub fn yagi(freq_hz: f64, elements: usize, height_m: f64) -> Antenna {
    let lam = wavelength_m(freq_hz);
    let n = elements.max(2);
    let spacing = 0.20 * lam;
    let lengths = {
        let mut v = vec![0.50 * lam, 0.48 * lam]; // reflector, driven
        let directors = n - 2;
        let (first, last) = (0.44 * lam, 0.40 * lam);
        for i in 0..directors {
            let t = if directors <= 1 { 0.0 } else { i as f64 / (directors - 1) as f64 };
            v.push(first + (last - first) * t);
        }
        v
    };
    let mut wires = Vec::new();
    for (i, len) in lengths.iter().enumerate() {
        let y = i as f64 * spacing;
        wires.push(Wire {
            from: [-len / 2.0, y, height_m],
            to: [len / 2.0, y, height_m],
            radius_m: WIRE_RADIUS_M,
            seg: Seg::AutoTaper,
        });
    }
    Antenna {
        name: "Yagi-Uda (starting layout)".into(),
        freq_hz,
        wires,
        // The driven element is wire 2.
        sources: vec![center_feed(2)],
        ground: Ground { kind: GroundKind::Real, ..Ground::default() },
        ..Antenna::default()
    }
}

/// Every preset for a frequency, in menu order. The list is meant to grow.
pub fn catalog(freq_hz: f64) -> Vec<Antenna> {
    vec![
        dipole(freq_hz, DEFAULT_HEIGHT_M),
        inverted_v(freq_hz, DEFAULT_HEIGHT_M),
        vertical(freq_hz),
        ground_plane(freq_hz),
        delta_loop(freq_hz, DEFAULT_HEIGHT_M),
        yagi(freq_hz, 3, DEFAULT_HEIGHT_M),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every preset is a valid model: wires, exactly one feed naming a real
    /// wire, and a frequency.
    #[test]
    fn every_preset_is_well_formed() {
        for a in catalog(27_265_000.0) {
            assert!(!a.wires.is_empty(), "{}: no wires", a.name);
            assert_eq!(a.sources.len(), 1, "{}: source count", a.name);
            let s = &a.sources[0];
            assert!(
                s.at.wire as usize <= a.wires.len(),
                "{}: feed on a wire that is not there",
                a.name
            );
            assert!(a.freq_hz > 0.0);
            assert!(a.wires.iter().all(|w| w.radius_m > 0.0), "{}: zero radius", a.name);
        }
    }

    /// The dipole preset is the calculator's length, so they cannot disagree.
    #[test]
    fn the_dipole_preset_matches_the_calculator() {
        let f = 27_265_000.0;
        let a = dipole(f, 10.0);
        let w = &a.wires[0];
        let len = (w.to[0] - w.from[0]).abs();
        let calc = crate::dipole(f).dims[0].metres;
        assert!((len - calc).abs() < 1e-9, "{len} vs {calc}");
    }

    /// The delta loop closes: each wire's end is the next wire's start.
    #[test]
    fn the_delta_loop_is_closed() {
        let a = delta_loop(28_500_000.0, 10.0);
        let n = a.wires.len();
        for i in 0..n {
            let end = a.wires[i].to;
            let start = a.wires[(i + 1) % n].from;
            assert_eq!(end, start, "wire {i} does not meet wire {}", (i + 1) % n);
        }
    }

    /// A Yagi of N elements has N wires.
    #[test]
    fn the_yagi_has_one_wire_per_element() {
        assert_eq!(yagi(50_000_000.0, 4, 10.0).wires.len(), 4);
        assert_eq!(yagi(50_000_000.0, 2, 10.0).wires.len(), 2);
    }

    /// A preset survives a .maa round trip, so it can be exported to MMANA.
    #[test]
    fn a_preset_exports_to_maa() {
        for a in catalog(27_265_000.0) {
            let text = crate::maa::write(&a);
            let back = crate::maa::read(&text).unwrap();
            assert_eq!(a, back, "{} did not round-trip", a.name);
        }
    }
}
