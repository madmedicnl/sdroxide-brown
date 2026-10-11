//! The antenna model — the geometry a `.maa` file or `antennas.json` holds, and
//! the shape presets build.
//!
//! This is data only: wires, a source, loads, the ground and MMANA's
//! segmentation parameters. Nothing here solves anything (that is the solver,
//! later). Field names and the numeric codes follow MMANA-GAL so a `.maa` file
//! round-trips without translation loss.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A straight wire segment end-points in metres, its radius and how MMANA should
/// divide it into segments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wire {
    pub from: [f64; 3],
    pub to: [f64; 3],
    pub radius_m: f64,
    pub seg: Seg,
}

/// MMANA's segmentation directive for one wire (the file's SEG column).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seg {
    /// A positive integer: exactly this many segments.
    Fixed(u32),
    /// `0` — automatic regular segmentation.
    AutoUniform,
    /// `-1` — automatic tapering (the usual choice).
    AutoTaper,
    /// `-2` — taper only at the start end.
    TaperStart,
    /// `-3` — taper only at the finish end.
    TaperEnd,
}

impl Seg {
    /// The integer as it appears in the file (negative values are MMANA's).
    pub fn code(self) -> i32 {
        match self {
            Seg::Fixed(n) => n as i32,
            Seg::AutoUniform => 0,
            Seg::AutoTaper => -1,
            Seg::TaperStart => -2,
            Seg::TaperEnd => -3,
        }
    }

    pub fn from_code(code: i32) -> Seg {
        match code {
            n if n > 0 => Seg::Fixed(n as u32),
            0 => Seg::AutoUniform,
            -1 => Seg::AutoTaper,
            -2 => Seg::TaperStart,
            -3 => Seg::TaperEnd,
            _ => Seg::AutoTaper,
        }
    }
}

/// Where on a wire a source or a load sits: MMANA's `w<#>C|B|E[±offset]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WirePoint {
    /// 1-based wire number, as MMANA counts.
    pub wire: u32,
    pub at: Pos,
    /// Segments away from `at`, positive toward the wire's end.
    pub offset: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pos {
    Begin,
    Center,
    End,
}

impl fmt::Display for WirePoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = match self.at {
            Pos::Begin => 'b',
            Pos::Center => 'c',
            Pos::End => 'e',
        };
        if self.offset == 0 {
            write!(f, "w{}{}", self.wire, letter)
        } else {
            write!(f, "w{}{}{}", self.wire, letter, self.offset)
        }
    }
}

impl WirePoint {
    /// Parse `w1c`, `W3C1`, `w4c-1`, `v5b` … (case- and `v`/`w`-tolerant).
    pub fn parse(s: &str) -> Option<WirePoint> {
        let s = s.trim();
        let mut it = s.chars();
        let lead = it.next()?.to_ascii_lowercase();
        if lead != 'w' && lead != 'v' {
            return None;
        }
        let rest: String = it.collect();
        let split = rest.find(|c: char| c.is_ascii_alphabetic())?;
        let (digits, tail) = rest.split_at(split);
        let wire: u32 = digits.parse().ok()?;
        let mut tc = tail.chars();
        let at = match tc.next()?.to_ascii_lowercase() {
            'b' => Pos::Begin,
            'c' => Pos::Center,
            'e' => Pos::End,
            _ => return None,
        };
        let offset: i32 = tc.as_str().trim().parse().unwrap_or(0);
        Some(WirePoint { wire, at, offset })
    }
}

/// A feed: MMANA's `w…, phase°, volts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub at: WirePoint,
    pub phase_deg: f64,
    pub volts: f64,
}

/// A lumped load. Held as MMANA writes it — a designator and four numbers — for
/// now: the file's field meaning depends on the load type, and typing it is a
/// solver-time job, not a storage one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Load {
    pub at: WirePoint,
    pub values: [f64; 4],
}

/// The ground under the antenna (the file's `***G/H/M/R/AzEl/X***` line).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ground {
    pub kind: GroundKind,
    /// Conductivity, mS/m, as MMANA stores it (`0.0` = unspecified).
    pub sigma_ms_m: f64,
    /// Radials, informational.
    pub radials: i32,
    /// Reference impedance for the SWR/reflection readout.
    pub ref_z: f64,
    /// Pattern display angles and the height offset, kept for round-trip.
    pub az_deg: f64,
    pub el_deg: f64,
    pub height_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroundKind {
    FreeSpace,
    Perfect,
    Real,
    SommerfeldNorton,
}

impl GroundKind {
    pub fn code(self) -> i32 {
        match self {
            GroundKind::FreeSpace => 0,
            GroundKind::Perfect => 1,
            GroundKind::Real => 2,
            GroundKind::SommerfeldNorton => -1,
        }
    }

    pub fn from_code(code: i32) -> GroundKind {
        match code {
            1 => GroundKind::Perfect,
            2 => GroundKind::Real,
            -1 => GroundKind::SommerfeldNorton,
            _ => GroundKind::FreeSpace,
        }
    }
}

impl Default for Ground {
    fn default() -> Ground {
        Ground {
            kind: GroundKind::Real,
            sigma_ms_m: 5.0,
            radials: 0,
            ref_z: 50.0,
            az_deg: 0.0,
            el_deg: 0.0,
            height_m: 0.0,
        }
    }
}

/// MMANA's auto-segmentation parameters (`***Segmentation***`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segmentation {
    pub dm1: u32,
    pub dm2: u32,
    pub sc: f64,
    pub ec: u32,
}

impl Default for Segmentation {
    fn default() -> Segmentation {
        // MMANA's own worked example, and a sane starting point.
        Segmentation { dm1: 200, dm2: 20, sc: 2.0, ec: 1 }
    }
}

/// A whole model: geometry, feed, loads, ground, frequency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Antenna {
    pub name: String,
    pub comment: String,
    pub freq_hz: f64,
    pub wires: Vec<Wire>,
    pub sources: Vec<Source>,
    pub loads: Vec<Load>,
    pub ground: Ground,
    pub segmentation: Segmentation,
}

impl Default for Antenna {
    fn default() -> Antenna {
        Antenna {
            name: String::new(),
            comment: String::new(),
            freq_hz: 0.0,
            wires: Vec::new(),
            sources: Vec::new(),
            loads: Vec::new(),
            ground: Ground::default(),
            segmentation: Segmentation::default(),
        }
    }
}

impl Antenna {
    /// A one-wire model with a wire from `a` to `b` and a centre feed — the
    /// shape every preset starts from.
    pub fn wire_with_feed(
        freq_hz: f64,
        name: &str,
        a: [f64; 3],
        b: [f64; 3],
        radius_m: f64,
        seg: Seg,
    ) -> Antenna {
        Antenna {
            name: name.to_string(),
            freq_hz,
            wires: vec![Wire { from: a, to: b, radius_m, seg }],
            sources: vec![Source {
                at: WirePoint { wire: 1, at: Pos::Center, offset: 0 },
                phase_deg: 0.0,
                volts: 1.0,
            }],
            ..Antenna::default()
        }
    }
}
