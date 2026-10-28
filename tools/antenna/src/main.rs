//! `antenna` — the prototype's front door. The real home is a tool window in
//! sdroxide; this is how the pieces are exercised while they are settled.
//!
//! ```text
//! antenna [MHz]                     closed-form dimensions (default 27.265)
//! antenna --presets [MHz]           the CB/SWL shape presets, as .maa
//! antenna --expr "R/4" [MHz]        evaluate an MMANA dimension expression
//! antenna --convert 17ft --to m [MHz]   convert a length between units
//! ```

use ant::expr::{Consts, LengthUnit, eval, parse_length};
use antenna_model as ant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = Mode::Calc;
    let mut freq_mhz = 27.265_f64;
    let mut expr: Option<String> = None;
    let mut convert: Option<String> = None;
    let mut to: Option<String> = None;
    let mut pattern_out: Option<String> = None;
    let mut pattern3d_out: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--presets" => mode = Mode::Presets,
            "--solve" => mode = Mode::Solve,
            "--pattern" => {
                i += 1;
                pattern_out = args.get(i).cloned();
                mode = Mode::Pattern;
            }
            "--pattern3d" => {
                i += 1;
                pattern3d_out = args.get(i).cloned();
                mode = Mode::Pattern3d;
            }
            "--expr" => {
                i += 1;
                expr = args.get(i).cloned();
                mode = Mode::Expr;
            }
            "--convert" => {
                i += 1;
                convert = args.get(i).cloned();
                mode = Mode::Convert;
            }
            "--to" => {
                i += 1;
                to = args.get(i).cloned();
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => match other.parse::<f64>() {
                Ok(v) => {
                    freq_mhz = v;
                }
                Err(_) => {
                    eprintln!("unrecognised argument: {other}");
                    print_help();
                    std::process::exit(2);
                }
            },
        }
        i += 1;
    }
    if freq_mhz <= 0.0 {
        eprintln!("frequency must be positive");
        std::process::exit(2);
    }
    let freq_hz = freq_mhz * 1e6;

    match mode {
        Mode::Calc => calc(freq_mhz, freq_hz),
        Mode::Presets => presets(freq_mhz, freq_hz),
        Mode::Solve => solve(freq_mhz, freq_hz),
        Mode::Pattern => pattern(pattern_out.as_deref(), freq_mhz, freq_hz),
        Mode::Pattern3d => pattern3d(pattern3d_out.as_deref(), freq_mhz, freq_hz),
        Mode::Expr => do_expr(expr.as_deref().unwrap_or(""), freq_hz),
        Mode::Convert => do_convert(convert.as_deref().unwrap_or(""), to.as_deref(), freq_hz),
    }
}

enum Mode {
    Calc,
    Presets,
    Solve,
    Pattern,
    Pattern3d,
    Expr,
    Convert,
}

fn calc(mhz: f64, freq_hz: f64) {
    println!(
        "f = {mhz} MHz   lambda = {:.3} m ({:.2} ft)\n",
        ant::wavelength_m(freq_hz),
        ant::wavelength_m(freq_hz) / 0.3048
    );
    for d in ant::all(freq_hz) {
        println!("{:<22} {}", d.name, d.notes);
        for dim in &d.dims {
            println!(
                "    {:<22} {:>8.3} m  ({:>7.2} ft)",
                dim.what,
                dim.metres,
                dim.metres / 0.3048
            );
        }
        if let Some((r, x)) = d.feed_z {
            let swr = ant::swr_50(r, x);
            println!(
                "    {:<22} {r:.0}{}{:.0}j ohm  ->  SWR {swr:.2}:1 at 50 ohm",
                "nominal feed",
                if x < 0.0 { "-" } else { "+" },
                x
            );
        }
        println!();
    }
}

fn presets(mhz: f64, freq_hz: f64) {
    println!("# CB/SWL presets at {mhz} MHz\n");
    for a in ant::presets::catalog(freq_hz) {
        println!("# {} — {} wire(s)", a.name, a.wires.len());
        print!("{}", ant::maa::write(&a));
        println!();
    }
}

fn solve(mhz: f64, freq_hz: f64) {
    use ant::solver::{SolverConfig, analyze, front_to_back_db, max_gain, swr};
    println!("# feed impedance, SWR (50 ohm), gain and front-to-back at {mhz} MHz");
    println!("# a bend feed (the inverted-V apex) splits into two terminals and solves;\n");
    println!("# a feed at a three-or-more-way junction (the ground plane) is still refused\n");
    for a in ant::presets::catalog(freq_hz) {
        match analyze(&a, &SolverConfig::default()) {
            Ok(sol) => {
                let (g, th, ph) = max_gain(&sol);
                println!(
                    "{:<31} Zin {:>6.1} {:+.1}j  SWR {:.2}:1   gain {:.1} dBi @ {th:.0}/{ph:.0}   F/B {:.1} dB",
                    a.name,
                    sol.zin.re,
                    sol.zin.im,
                    swr(sol.zin, 50.0),
                    g,
                    front_to_back_db(&sol),
                );
            }
            Err(e) => println!("{:<31} — {e}", a.name),
        }
    }
}

fn pattern(path: Option<&str>, mhz: f64, freq_hz: f64) {
    use ant::solver::{SolverConfig, analyze, cut, polar_svg};
    let out = path.unwrap_or("antenna-pattern.svg");
    let a = ant::presets::dipole(freq_hz, ant::presets::DEFAULT_HEIGHT_M);
    match analyze(&a, &SolverConfig::default()) {
        Ok(sol) => {
            let az = cut(&sol, true, 90.0, 5.0);
            let svg = polar_svg(&format!("Dipole azimuth, {mhz:.3} MHz"), &az, -20.0);
            match std::fs::write(out, svg) {
                Ok(()) => println!("wrote {out}"),
                Err(e) => {
                    eprintln!("could not write {out}: {e}");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

/// Write each solvable preset's 3D pattern as a PLY mesh and an isometric SVG
/// wireframe, into the directory `dir` (or `antenna-3d/`).
fn pattern3d(dir: Option<&str>, mhz: f64, freq_hz: f64) {
    use ant::solver::{SolverConfig, analyze, iso_svg, max_gain, pattern_surface, ply_ascii};
    let dir = dir.unwrap_or("antenna-3d");
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("could not make {dir}: {e}");
        std::process::exit(1);
    }
    for a in ant::presets::catalog(freq_hz) {
        match analyze(&a, &SolverConfig::default()) {
            Ok(sol) => {
                let (verts, nt, np) = pattern_surface(&sol, 61, 72);
                let (gmax, _, _) = max_gain(&sol);
                let title = format!("{} 3D, {mhz:.3} MHz — peak {gmax:.1} dBi", a.name);
                let stem = a.name.replace(' ', "-").to_lowercase();
                let ply = format!("{dir}/{stem}.ply");
                let svg = format!("{dir}/{stem}.svg");
                if std::fs::write(&ply, ply_ascii(&verts, nt, np)).is_ok() {
                    println!("wrote {ply}");
                }
                if std::fs::write(&svg, iso_svg(&title, &verts, nt, np)).is_ok() {
                    println!("wrote {svg}");
                }
            }
            Err(e) => println!("{:<30} — {e}", a.name),
        }
    }
}

fn do_expr(text: &str, freq_hz: f64) {
    let c = Consts::at_freq(freq_hz);
    match eval(text, c) {
        Ok(m) => {
            println!("{text} = {m:.6} m  ({:.4} ft, {:.6} lambda)", m / 0.3048, m / c.lambda_m)
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn do_convert(value: &str, to: Option<&str>, freq_hz: f64) {
    let lam = ant::wavelength_m(freq_hz);
    let metres = match parse_length(value, lam) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let want = to.unwrap_or("m");
    let u = LengthUnit::ALL
        .iter()
        .copied()
        .find(|u| {
            u.suffix() == want
                || u.label() == want
                || matches!((u, want), (LengthUnit::Metres, "m"))
        })
        .unwrap_or(LengthUnit::Metres);
    println!(
        "{value} = {:.4} m = {:.4} ft = {:.4} in = {:.6} lambda",
        metres,
        LengthUnit::Feet.from_metres(metres, lam),
        LengthUnit::Inches.from_metres(metres, lam),
        LengthUnit::Wavelength.from_metres(metres, lam),
    );
    println!("  (asked for {want}: {:.4} {})", u.from_metres(metres, lam), u.suffix());
}

fn print_help() {
    println!("antenna [MHz]                     closed-form dimensions (default 27.265)");
    println!("antenna --presets [MHz]           the CB/SWL shape presets, as .maa");
    println!("antenna --solve [MHz]             solve the presets: Zin and SWR");
    println!("antenna --pattern FILE [MHz]      write the dipole azimuth pattern as SVG");
    println!("antenna --pattern3d DIR [MHz]     write each preset's 3D pattern (PLY + SVG)");
    println!("antenna --expr \"R/4\" [MHz]        evaluate an MMANA dimension expression");
    println!("antenna --convert 17ft --to m     convert a length between units");
}
