//! `antenna <MHz>` — print the closed-form dimensions for the common wire
//! antennas at a frequency, and the SWR they present against 50 ohms.
//!
//! This is the prototype's front door and nothing more: a way to look at the
//! numbers while the shapes and the arithmetic are settled. The real home is a
//! tool window in sdroxide.

use antenna_model as ant;

fn main() {
    let mut args = std::env::args().skip(1);
    let mhz: f64 = match args.next() {
        Some(a) => match a.parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("usage: antenna <freq_mhz>   (e.g. antenna 27.265)");
                std::process::exit(2);
            }
        },
        None => 27.265,
    };
    if mhz <= 0.0 {
        eprintln!("frequency must be positive");
        std::process::exit(2);
    }
    let freq_hz = mhz * 1e6;

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
