//! Write a sample QSL card to /tmp so it can be looked at.
//! Run: cargo run -p sdroxide-ui --example qsl_card_probe
fn main() {
    let (w, h) = (640u16, 480u16);
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (x, y) = (u32::from(x), u32::from(y));
            px.extend_from_slice(&[
                (x * 255 / u32::from(w)) as u8,
                (y * 255 / u32::from(h)) as u8,
                ((x + y) % 255) as u8,
            ]);
        }
    }
    let mut spec = sdroxide_ui::qsl_card::CardSpec::new(px, w, h)
        .with("From", "19DCG044")
        .with("To", "19DC373")
        .with("Date", "28-Sep-2026")
        .with("Freq", "27.267 MHz")
        .with("Mode", "SSTV")
        .with("RST sent", "57")
        .with("RST rcvd", "59")
        .with("Grid", "IO93WQ");
    spec.callsign = "19DCG044".into();
    spec.comment = "Thanks for the SSTV picture.\n\n73 from Roeg aan de Zaan, Holland".into();
    let png = spec.compose_png(1000, 620).expect("a font is bundled");
    std::fs::write("/tmp/opencode/qsl_card_sample.png", &png).unwrap();
    println!("wrote /tmp/opencode/qsl_card_sample.png ({} bytes)", png.len());
}
