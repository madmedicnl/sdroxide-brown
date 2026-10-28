//! MMANA-GAL `.maa` files — read and write.
//!
//! The format is line-oriented and has two variants in the wild (OpenNEC's
//! survey: ~78% A, ~22% B). MMANA-GAL itself writes **B**, so that is what this
//! writes; the reader accepts both.
//!
//! ```text
//! [title]
//! *
//! <freq MHz>
//! ***Wires***
//! <nw>
//! x1, y1, z1, x2, y2, z2, radius_m, seg      (x nw)
//! ***Source***
//! <ns>, 0
//! w1c, <phase_deg>, <volts>                  (x ns)
//! ***Load***
//! <nl>, 0
//! w1c, <v0>, <v1>, <v2>, <v3>                (x nl)
//! ***Segmentation***
//! <dm1>, <dm2>, <sc>, <ec>
//! ***G/H/M/R/AzEl/X***
//! <gtype>, <sigma mS/m>, <radials>, <refz>, <az>, <el>, <height m>
//! ###Comment###
//! <text>
//! ```
//!
//! Variant A instead puts a combined `nw, nl, ns` counts line where B has
//! `***Wires***`; the reader detects that by a three-number line where a wire
//! line always has eight.

use crate::model::*;

/// Parse a `.maa` document.
pub fn read(text: &str) -> Result<Antenna, String> {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let mut a = Antenna::default();
    let mut i = 0;

    // Header: an optional title, an optional `*`, then the frequency in MHz.
    loop {
        let l = next_nonblank(&lines, &mut i).ok_or("no frequency line")?.trim();
        if l == "*" {
            continue;
        }
        if l.starts_with("***") || l.starts_with("###") {
            return Err("no frequency line".into());
        }
        if let Ok(f) = l.parse::<f64>() {
            a.freq_hz = f * 1e6;
            break;
        }
        if a.name.is_empty() {
            a.name = l.to_string();
        }
    }

    // Wires: variant B has the `***Wires***` header; variant A a counts line.
    let line = next_nonblank(&lines, &mut i).ok_or("no wire block")?;
    let nw = if line.trim() == "***Wires***" {
        first_int(next_nonblank(&lines, &mut i).ok_or("no wire count")?) as usize
    } else {
        // Variant A combined counts `nw, nl, ns` — the first is the wire count.
        let nums = numbers(line);
        if nums.is_empty() {
            return Err("no wire count".into());
        }
        nums[0] as usize
    };
    for _ in 0..nw {
        let l = next_nonblank(&lines, &mut i).ok_or("wire block ends early")?;
        a.wires.push(parse_wire(l)?);
    }

    // The remaining labelled sections, in any order.
    while let Some(l) = next_nonblank(&lines, &mut i) {
        let t = l.trim();
        if t == "***Source***" {
            let ns =
                first_int(next_nonblank(&lines, &mut i).ok_or("source count")?).max(0) as usize;
            for _ in 0..ns {
                let s = next_nonblank(&lines, &mut i).ok_or("source block ends early")?;
                a.sources.push(parse_source(s)?);
            }
        } else if t == "***Load***" {
            let nl = first_int(next_nonblank(&lines, &mut i).ok_or("load count")?).max(0) as usize;
            for _ in 0..nl {
                let s = next_nonblank(&lines, &mut i).ok_or("load block ends early")?;
                a.loads.push(parse_load(s)?);
            }
        } else if t == "***Segmentation***" {
            if let Some(s) = next_nonblank(&lines, &mut i) {
                let n = numbers(s);
                if n.len() >= 4 {
                    a.segmentation = Segmentation {
                        dm1: n[0].max(1.0) as u32,
                        dm2: n[1].max(1.0) as u32,
                        sc: n[2],
                        ec: n[3].max(1.0) as u32,
                    };
                }
            }
        } else if t.starts_with("***G/H/M/R") {
            if let Some(s) = next_nonblank(&lines, &mut i) {
                let n = numbers(s);
                if n.len() >= 7 {
                    a.ground = Ground {
                        kind: GroundKind::from_code(n[0] as i32),
                        eps_r: 13.0, // not stored in the file — a UI-only setting
                        sigma_ms_m: n[1],
                        radials: n[2] as i32,
                        ref_z: n[3],
                        az_deg: n[4],
                        el_deg: n[5],
                        height_m: n[6],
                    };
                }
            }
        } else if let Some(rest) = t.strip_prefix("###Comment###") {
            let mut c = rest.trim().to_string();
            // Any following lines up to the next marker are the comment too.
            while let Some(n) = lines.get(i) {
                let nt = n.trim();
                if nt.starts_with("***") || nt.starts_with("###") {
                    break;
                }
                if !nt.is_empty() {
                    if !c.is_empty() {
                        c.push('\n');
                    }
                    c.push_str(nt);
                }
                i += 1;
            }
            a.comment = c;
        }
        // Any other line (an unknown `***Section***` or a stray) is skipped.
    }
    Ok(a)
}

/// Write a model as a `.maa` document (variant B, what MMANA-GAL writes).
pub fn write(a: &Antenna) -> String {
    let mut out = String::new();
    if !a.name.is_empty() {
        out.push_str(&a.name);
        out.push('\n');
    }
    out.push_str("*\n");
    out.push_str(&format!("{}\n", a.freq_hz / 1e6));

    out.push_str("***Wires***\n");
    out.push_str(&format!("{}\n", a.wires.len()));
    for w in &a.wires {
        out.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {}, {}\n",
            w.from[0],
            w.from[1],
            w.from[2],
            w.to[0],
            w.to[1],
            w.to[2],
            w.radius_m,
            w.seg.code()
        ));
    }

    out.push_str("***Source***\n");
    out.push_str(&format!("{}, 0\n", a.sources.len()));
    for s in &a.sources {
        out.push_str(&format!("{}, {}, {}\n", s.at, s.phase_deg, s.volts));
    }

    out.push_str("***Load***\n");
    out.push_str(&format!("{}, 0\n", a.loads.len()));
    for l in &a.loads {
        out.push_str(&format!(
            "{}, {}, {}, {}, {}\n",
            l.at, l.values[0], l.values[1], l.values[2], l.values[3]
        ));
    }

    out.push_str("***Segmentation***\n");
    out.push_str(&format!(
        "{}, {}, {}, {}\n",
        a.segmentation.dm1, a.segmentation.dm2, a.segmentation.sc, a.segmentation.ec
    ));

    out.push_str("***G/H/M/R/AzEl/X***\n");
    out.push_str(&format!(
        "{}, {}, {}, {}, {}, {}, {}\n",
        a.ground.kind.code(),
        a.ground.sigma_ms_m,
        a.ground.radials,
        a.ground.ref_z,
        a.ground.az_deg,
        a.ground.el_deg,
        a.ground.height_m
    ));

    out.push_str("###Comment###\n");
    out.push_str(&a.comment);
    out.push('\n');
    out
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn next_nonblank<'a>(lines: &[&'a str], i: &mut usize) -> Option<&'a str> {
    while *i < lines.len() {
        let l = lines[*i];
        *i += 1;
        if !l.trim().is_empty() {
            return Some(l);
        }
    }
    None
}

/// Comma-separated tokens, trimmed, empties dropped.
fn parts(line: &str) -> Vec<String> {
    line.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// Every number on the line, whether comma- or space-separated.
fn numbers(line: &str) -> Vec<f64> {
    line.split([',', ' ', '\t']).filter_map(|p| p.trim().parse::<f64>().ok()).collect()
}

fn first_int(line: &str) -> i64 {
    numbers(line).first().map(|f| *f as i64).unwrap_or(0)
}

fn parse_wire(line: &str) -> Result<Wire, String> {
    let n = numbers(line);
    if n.len() < 8 {
        return Err(format!("wire needs 8 fields: {line}"));
    }
    Ok(Wire {
        from: [n[0], n[1], n[2]],
        to: [n[3], n[4], n[5]],
        radius_m: n[6],
        seg: Seg::from_code(n[7] as i32),
    })
}

fn parse_source(line: &str) -> Result<Source, String> {
    let p = parts(line);
    let at = WirePoint::parse(&p[0]).ok_or_else(|| format!("bad source point: {line}"))?;
    let phase_deg = p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let volts = p.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    Ok(Source { at, phase_deg, volts })
}

fn parse_load(line: &str) -> Result<Load, String> {
    let p = parts(line);
    let at = WirePoint::parse(&p[0]).ok_or_else(|| format!("bad load point: {line}"))?;
    let mut values = [0.0; 4];
    for (k, v) in values.iter_mut().enumerate() {
        *v = p.get(k + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    }
    Ok(Load { at, values })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MMANA-GAL's own worked example, from its help — a 435 MHz mobile ground
    /// plane, with a source, a load, segmentation and a comment.
    const MMANA_EXAMPLE: &str = "\
*
435.0
***Wires***
1
0.0, 0.0, 0.0, 0.0, 0.0, 0.492, 0.0015, -1
***Source***
1, 1
w1b, 0.0, 1.0
***Load***
1, 1
w1c, 0, 0.03, 0.0, 0.0
***Segmentation***
200, 20, 2.0, 1
***G/H/M/R/AzEl/X***
2, 0.0, 4, 50.0, 120, 60, 0.0
###Comment###
Mod by Mario Chomicz, G8ODE 16/11/2009 14:17:50
";

    #[test]
    fn reads_the_mmana_example() {
        let a = read(MMANA_EXAMPLE).unwrap();
        assert!((a.freq_hz - 435_000_000.0).abs() < 1.0);
        assert_eq!(a.wires.len(), 1);
        assert_eq!(a.wires[0].to, [0.0, 0.0, 0.492]);
        assert!((a.wires[0].radius_m - 0.0015).abs() < 1e-12);
        assert_eq!(a.wires[0].seg, Seg::AutoTaper);
        assert_eq!(a.sources.len(), 1);
        assert_eq!(a.sources[0].at, WirePoint { wire: 1, at: Pos::Begin, offset: 0 });
        assert!((a.sources[0].volts - 1.0).abs() < 1e-12);
        assert_eq!(a.loads.len(), 1);
        assert!((a.loads[0].values[1] - 0.03).abs() < 1e-12);
        assert_eq!(a.segmentation, Segmentation { dm1: 200, dm2: 20, sc: 2.0, ec: 1 });
        assert_eq!(a.ground.kind, GroundKind::Real);
        assert_eq!(a.ground.radials, 4);
        assert!(a.comment.contains("G8ODE"));
    }

    /// Write, read it back, and the model is unchanged.
    #[test]
    fn round_trips_through_the_file() {
        let a = read(MMANA_EXAMPLE).unwrap();
        let again = read(&write(&a)).unwrap();
        assert_eq!(a, again);
    }

    /// Variant A: a combined counts line where B has the `***Wires***` header.
    #[test]
    fn reads_variant_a() {
        let text = "\
20m dipole
7.1
1, 1, 1
0.0, 0.0, 5.0, 0.0, 0.0, -5.0, 0.001, -1
***Source***
1
w1c, 0.0, 1.0
";
        let a = read(text).unwrap();
        assert_eq!(a.name, "20m dipole");
        assert!((a.freq_hz - 7_100_000.0).abs() < 1.0);
        assert_eq!(a.wires.len(), 1);
        assert_eq!(a.wires[0].from, [0.0, 0.0, 5.0]);
        assert_eq!(a.sources.len(), 1);
        assert_eq!(a.sources[0].at.at, Pos::Center);
    }

    /// The designator forms MMANA uses, including offsets.
    #[test]
    fn wire_point_designators() {
        assert_eq!(
            WirePoint::parse("w1c").unwrap(),
            WirePoint { wire: 1, at: Pos::Center, offset: 0 }
        );
        assert_eq!(
            WirePoint::parse("W3C1").unwrap(),
            WirePoint { wire: 3, at: Pos::Center, offset: 1 }
        );
        assert_eq!(
            WirePoint::parse("w4c-1").unwrap(),
            WirePoint { wire: 4, at: Pos::Center, offset: -1 }
        );
        assert_eq!(
            WirePoint::parse("v5b").unwrap(),
            WirePoint { wire: 5, at: Pos::Begin, offset: 0 }
        );
        assert_eq!(
            WirePoint::parse("w6e3").unwrap(),
            WirePoint { wire: 6, at: Pos::End, offset: 3 }
        );
        assert!(WirePoint::parse("1c").is_none());
        assert!(WirePoint::parse("garbage").is_none());
        // Display is the inverse for the canonical lowercase form.
        assert_eq!(WirePoint::parse("w2c").unwrap().to_string(), "w2c");
        assert_eq!(WirePoint::parse("w7c-2").unwrap().to_string(), "w7c-2");
    }

    /// A broken document is an error, not a half-built antenna.
    #[test]
    fn malformed_input_is_refused() {
        assert!(read("not a frequency\n").is_err());
        assert!(read("7.1\n***Wires***\n3\n0,0,0,0,0,1,0.001,-1\n").is_err());
    }
}
