//! Drawing the picture composer's document, and the hand-off of that document
//! to the engine.
//!
//! The **document itself** is a wire type ([`sdroxide_types::Composer`]): each
//! transmit preset carries its own, so the console, a browser tab attached to
//! the same radio and the next session all compose the same picture. This module
//! is the half that only a client can do — turning layers into pixels — and the
//! little state machine that keeps one operator's edits from fighting the
//! engine's echo of them.
//!
//! Rendering is pure Rust (`ab_glyph` + `image`) so the native app and the wasm
//! browser client compose identically.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use sdroxide_types::{Command, Composer, Face, ImagePresets, TextLayer};

use crate::sstv::{Ink, TextSink, crop_scale, draw_text_into, text_width};

/// A box in picture pixels: `(x0, y0, x1, y1)`, top-left first.
pub(crate) type Box2 = (i32, i32, i32, i32);

/// The bundled font a face is printed with.
///
/// Bundled rather than taken from the system on purpose: the browser client has
/// no access to the operator's fonts, so a system face would make the same
/// document compose into two different pictures depending on which screen
/// pressed SEND. All three ship in `assets/fonts` under the OFL.
pub(crate) fn font(face: Face) -> Option<FontRef<'static>> {
    const BOLD: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf");
    const PLAIN: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-Regular.ttf");
    const MONO: &[u8] = include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf");
    let raw = match face {
        Face::ChakraBold => BOLD,
        Face::Chakra => PLAIN,
        Face::Mono => MONO,
    };
    debug_assert_eq!(face.file(), match face {
        Face::ChakraBold => "ChakraPetch-SemiBold.ttf",
        Face::Chakra => "ChakraPetch-Regular.ttf",
        Face::Mono => "ShareTechMono-Regular.ttf",
    });
    FontRef::try_from_slice(raw).ok()
}

/// Compose the frame: `src` is the base picture in interleaved RGB, or `None`
/// for a flat dark frame so a layout can be tried before a picture is chosen.
pub(crate) fn render(
    doc: &Composer,
    w: u16,
    h: u16,
    src: Option<(&[u8], u16, u16)>,
) -> (Vec<u8>, u16, u16) {
    let mut img = match src {
        Some((rgb, sw, sh)) => crop_scale(rgb, sw, sh, w, h),
        None => vec![24u8; usize::from(w) * usize::from(h) * 3],
    };
    draw_onto(doc, &mut img, w, h);
    (img, w, h)
}

/// Draw every visible layer over an already sized frame.
pub(crate) fn draw_onto(doc: &Composer, img: &mut [u8], w: u16, h: u16) {
    let (uw, uh) = (usize::from(w), usize::from(h));
    for l in &doc.layers {
        if l.hidden || l.text.trim().is_empty() {
            continue;
        }
        if let Some(band) = &l.band {
            draw_band(img, uw, uh, l, band);
        }
        draw_layer(img, uw, uh, l);
    }
}

/// The box `doc.layers[i]` covers at frame size `(w, h)`, for hit-testing a drag
/// and for drawing the selection outline.
pub(crate) fn layer_box(doc: &Composer, i: usize, w: u16, h: u16) -> Option<Box2> {
    let l = doc.layers.get(i)?;
    let font = font(l.face)?;
    let (w, h) = (f32::from(w), f32::from(h));
    let scale = PxScale::from(f32::from(l.px.max(4)));
    let half = (text_width(&l.text, &font, scale) / 2.0).max(6.0);
    let half_h = (f32::from(l.px.max(4)) * 0.75).max(6.0);
    let (ax, ay) = (l.x * w, l.y * h);
    // A turned line's box is the same width and height, swapped.
    let (hx, hy) = if l.vertical { (half_h, half) } else { (half, half_h) };
    Some((
        (ax - hx).round() as i32,
        (ay - hy).round() as i32,
        (ax + hx).round() as i32,
        (ay + hy).round() as i32,
    ))
}

/// The layer under `(px, py)` — topmost first, so the line the operator sees on
/// top is the one a click picks up.
pub(crate) fn layer_at(doc: &Composer, px: f32, py: f32, w: u16, h: u16) -> Option<usize> {
    for i in (0..doc.layers.len()).rev() {
        if doc.layers[i].hidden {
            continue;
        }
        if let Some((x0, y0, x1, y1)) = layer_box(doc, i, w, h)
            && px >= x0 as f32
            && px <= x1 as f32
            && py >= y0 as f32
            && py <= y1 as f32
        {
            return Some(i);
        }
    }
    None
}

/// Fill the band behind a layer: rows across the frame for a horizontal line,
/// columns down it for a turned one. It is centred on the layer, so dragging the
/// text drags its strip with it.
fn draw_band(img: &mut [u8], w: usize, h: usize, layer: &TextLayer, band: &sdroxide_types::Strip) {
    let t = usize::from(band.thickness.max(1));
    let [fr, fg, fb] = band.fill;
    if layer.vertical {
        let cx = (layer.x * w as f32).round() as i32;
        let x0 = cx - t as i32 / 2;
        for x in x0..x0 + t as i32 {
            if x < 0 || x >= w as i32 {
                continue;
            }
            let u = (x - x0) as f32 / t as f32;
            let (r, g, b) = band_colour(fr, fg, fb, band.fill2, u);
            for y in 0..h {
                let i = (y * w + x as usize) * 3;
                img[i] = r;
                img[i + 1] = g;
                img[i + 2] = b;
            }
        }
    } else {
        let cy = (layer.y * h as f32).round() as i32;
        let y0 = cy - t as i32 / 2;
        for y in y0..y0 + t as i32 {
            if y < 0 || y >= h as i32 {
                continue;
            }
            let u = (y - y0) as f32 / t as f32;
            let (r, g, b) = band_colour(fr, fg, fb, band.fill2, u);
            let row = y as usize * w;
            for x in 0..w {
                let i = (row + x) * 3;
                img[i] = r;
                img[i + 1] = g;
                img[i + 2] = b;
            }
        }
    }
}

/// The band's colour at `u` (0 at the first edge, 1 at the last): a straight
/// two-stop gradient, or the original fade to black.
fn band_colour(fr: u8, fg: u8, fb: u8, fill2: Option<[u8; 3]>, u: f32) -> (u8, u8, u8) {
    match fill2 {
        Some([r2, g2, b2]) => {
            let lerp = |a: u8, c: u8| (f32::from(a) + (f32::from(c) - f32::from(a)) * u) as u8;
            (lerp(fr, r2), lerp(fg, g2), lerp(fb, b2))
        }
        None => {
            let shade = 1.0 - u;
            (
                (f32::from(fr) * shade) as u8,
                (f32::from(fg) * shade) as u8,
                (f32::from(fb) * shade) as u8,
            )
        }
    }
}

/// Draw one line, centred on its anchor — across the frame, or turned a quarter
/// turn clockwise about that same anchor.
fn draw_layer(img: &mut [u8], w: usize, h: usize, layer: &TextLayer) {
    let Some(font) = font(layer.face) else { return };
    let px = f32::from(layer.px.max(4));
    let scale = PxScale::from(px);
    let scaled = font.as_scaled(scale);
    let tw = text_width(&layer.text, &font, scale);
    let ax = layer.x * w as f32;
    let ay = layer.y * h as f32;
    // Baseline from the font's own ascent and descent, so a face with a deep
    // descender still sits in the middle of its band.
    let baseline = ay + (scaled.ascent() - scaled.descent()) / 2.0 - scaled.descent() / 2.0;
    let left = ax - tw / 2.0;
    let ink = Ink::Solid(layer.ink);
    let span = w as f32;

    let mut plain = crate::sstv::RgbSink { buf: img, w, h };
    if layer.vertical {
        // Painted flat and then turned about the anchor: one lattice rotation,
        // which is lossless, unlike scaling a rendered line.
        let mut turned = Turned { inner: &mut plain, cx: ax.round(), cy: ay.round() };
        paint_line(&mut turned, left, baseline, layer, &font, scale, ink, span);
    } else {
        paint_line(&mut plain, left, baseline, layer, &font, scale, ink, span);
    }
}

/// Print the line, with its outline first when it has one.
fn paint_line<S: TextSink>(
    sink: &mut S,
    left: f32,
    baseline: f32,
    layer: &TextLayer,
    font: &FontRef<'static>,
    scale: PxScale,
    ink: Ink,
    span: f32,
) {
    if let Some(oc) = layer.outline {
        // Eight directions, scaled to the text: a 12-pixel line needs one pixel
        // of edge, a 60-pixel headline needs more.
        let off = (f32::from(layer.px) / 14.0).max(1.0);
        for (ox, oy) in [
            (-off, 0.0),
            (off, 0.0),
            (0.0, -off),
            (0.0, off),
            (-off, -off),
            (off, -off),
            (-off, off),
            (off, off),
        ] {
            draw_text_into(
                sink,
                left + ox,
                baseline + oy,
                &layer.text,
                font,
                scale,
                Ink::Solid(oc),
                span,
                1.0,
            );
        }
    }
    draw_text_into(sink, left, baseline, &layer.text, font, scale, ink, span, 1.0);
}

/// A sink that turns everything a quarter turn clockwise about a point.
struct Turned<'a, S: TextSink> {
    inner: &'a mut S,
    cx: f32,
    cy: f32,
}

impl<S: TextSink> TextSink for Turned<'_, S> {
    fn paint(&mut self, x: i32, y: i32, rgb: (u8, u8, u8), alpha: f32) {
        let dx = x as f32 - self.cx;
        let dy = y as f32 - self.cy;
        // (dx, dy) → (-dy, dx): a point to the right of the anchor moves below
        // it, which is what makes the line read downwards.
        self.inner.paint((self.cx - dy).round() as i32, (self.cy + dx).round() as i32, rgb, alpha);
    }
}

// ── Whose copy of the document is the truth ──
//
// The document lives in the engine's preset store, and comes back on every
// presets announcement. One slot at a time is client-owned — whichever the
// operator is editing — so an echo of our own write, or an edit made on another
// screen, can update the other four without fighting the drag in hand. The same
// arrangement the preset messages use, for the same reason.

/// The slot claimed for editing, with the document being edited.
pub(crate) type Edit = Option<(usize, Composer)>;

/// The document to draw for `slot`: what is being edited if it is this slot,
/// otherwise what the engine says.
pub(crate) fn shown<'a>(
    edit: &'a Edit,
    presets: &'a ImagePresets,
    slot: usize,
) -> &'a Composer {
    match edit {
        Some((i, doc)) if *i == slot => doc,
        _ => presets.slots.get(slot).map(|s| &s.composer).unwrap_or(&EMPTY),
    }
}

static EMPTY: Composer = Composer { layers: Vec::new() };

/// Claim `slot` for editing, taking the engine's document as the starting
/// point. Returns whether the claim changed, so an unchanged claim does not
/// recompose the preview every frame.
pub(crate) fn claim(edit: &mut Edit, presets: &ImagePresets, slot: usize) -> bool {
    if matches!(edit, Some((i, _)) if *i == slot) {
        return false;
    }
    let doc = presets.slots.get(slot).map(|s| s.composer.clone()).unwrap_or_default();
    *edit = Some((slot, doc));
    true
}

/// Give the slot back to the engine.
pub(crate) fn release(edit: &mut Edit, presets: &ImagePresets, sent: &Edit) -> Option<Command> {
    let cmd = pending(edit, presets, sent);
    *edit = None;
    cmd
}

/// The command that would bring the engine up to date, if any.
///
/// `None` when nothing is claimed, when the engine already has this document, or
/// when this very document has already been sent and is only waiting for the
/// echo — which is what stops an un-echoed write being repeated on every frame.
pub(crate) fn pending(edit: &Edit, presets: &ImagePresets, sent: &Edit) -> Option<Command> {
    let (slot, doc) = edit.as_ref()?;
    let engine = presets.slots.get(*slot).map(|s| &s.composer);
    if engine == Some(doc) {
        return None;
    }
    if matches!(sent, Some((i, d)) if i == slot && d == doc) {
        return None;
    }
    Some(Command::ImageSetComposer { slot: *slot as u8, composer: doc.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{ImageSlotInfo, Strip};

    fn presets(docs: &[Composer]) -> ImagePresets {
        ImagePresets {
            slots: docs
                .iter()
                .map(|c| ImageSlotInfo { composer: c.clone(), ..ImageSlotInfo::default() })
                .collect(),
        }
    }

    fn doc(text: &str) -> Composer {
        let mut c = Composer::default();
        c.layers.push(TextLayer { text: text.into(), ..TextLayer::default() });
        c
    }

    fn frame(w: u16, h: u16) -> Vec<u8> {
        // Mid grey, so ink and band are both findable.
        vec![128u8; usize::from(w) * usize::from(h) * 3]
    }

    fn ink_rows(img: &[u8], w: u16, h: u16, want: [u8; 3]) -> Vec<usize> {
        let w = usize::from(w);
        let mut rows = Vec::new();
        for y in 0..usize::from(h) {
            let hit = (0..w).any(|x| {
                let i = (y * w + x) * 3;
                img[i] == want[0] && img[i + 1] == want[1] && img[i + 2] == want[2]
            });
            if hit {
                rows.push(y);
            }
        }
        rows
    }

    /// **A layer goes where its fractions say.** The document is written once
    /// and rendered at the mode's size, so this is the property everything else
    /// rests on — and it is why the document can ride the wire at all.
    #[test]
    fn a_layer_lands_at_its_fraction_of_the_frame() {
        let mut c = Composer::default();
        c.layers.push(TextLayer {
            text: "TEST".into(),
            x: 0.5,
            y: 0.25,
            px: 24,
            ink: [255, 0, 0],
            outline: None,
            ..Default::default()
        });
        let (img, w, h) = render(&c, 320, 256, None);
        let rows = ink_rows(&img, w, h, [255, 0, 0]);
        assert!(!rows.is_empty(), "nothing was printed");
        let mid = rows[rows.len() / 2];
        assert!(
            (mid as i32 - 64).abs() < 10,
            "a line at 0.25 of a 256-pixel frame printed at row {mid}"
        );
        // And the same document at another frame size lands at the same place.
        let (img2, w2, h2) = render(&c, 640, 512, None);
        let rows2 = ink_rows(&img2, w2, h2, [255, 0, 0]);
        let mid2 = rows2[rows2.len() / 2];
        assert!((mid2 as i32 - 128).abs() < 14, "row {mid2} at double size");
    }

    /// **A vertical layer reads downwards and its band runs the other way.**
    /// This is the "banner down the side" the operator asked for, and the one
    /// case that has to turn glyphs rather than lay out a line.
    #[test]
    fn a_vertical_layer_turns_the_text_and_its_band_with_it() {
        let mut c = Composer::default();
        c.layers.push(TextLayer {
            text: "F4NRM".into(),
            x: 0.15,
            y: 0.5,
            px: 22,
            ink: [255, 255, 0],
            outline: None,
            vertical: true,
            band: Some(Strip { thickness: 30, fill: [0, 0, 200], fill2: Some([0, 0, 200]) }),
            ..Default::default()
        });
        let (img, w, h) = render(&c, 320, 256, None);
        let rows = ink_rows(&img, w, h, [255, 255, 0]);
        // A turned line is taller than it is wide: it must cover far more rows
        // than a horizontal one of the same text would.
        assert!(rows.len() > 30, "a turned line covered only {} rows", rows.len());
        assert!(rows.len() < usize::from(h), "a turned line covered the whole frame");
        // The band is a column, not a row: at the layer's x, every row is blue
        // except where the letters are.
        let x = (0.15 * f32::from(w)) as usize;
        for y in 0..usize::from(h) {
            let i = (y * usize::from(w) + x) * 3;
            if img[i] == 255 && img[i + 1] == 255 {
                continue;
            }
            assert_eq!((img[i], img[i + 1], img[i + 2]), (0, 0, 200), "row {y} misses the band");
        }
    }

    /// The topmost layer is the one a click picks up, and a hidden one is not
    /// there at all.
    #[test]
    fn a_click_picks_the_top_layer_and_never_a_hidden_one() {
        let mut c = Composer::default();
        c.layers.push(TextLayer { text: "UNDER".into(), y: 0.5, px: 40, ..Default::default() });
        c.layers.push(TextLayer { text: "OVER".into(), y: 0.5, px: 40, ..Default::default() });
        assert_eq!(layer_at(&c, 160.0, 128.0, 320, 256), Some(1));
        c.layers[1].hidden = true;
        assert_eq!(layer_at(&c, 160.0, 128.0, 320, 256), Some(0));
        c.layers[0].hidden = true;
        assert_eq!(layer_at(&c, 160.0, 128.0, 320, 256), None);
    }

    /// Moving a layer's fractions moves its box, which is what a drag does.
    #[test]
    fn dragging_moves_the_box() {
        let mut c = Composer::default();
        c.layers.push(TextLayer { text: "CQ".into(), x: 0.2, y: 0.2, ..Default::default() });
        let before = layer_box(&c, 0, 320, 256).unwrap();
        c.layers[0].x = 0.8;
        let after = layer_box(&c, 0, 320, 256).unwrap();
        assert!(after.0 > before.0, "the box did not move right");
    }

    // ── the hand-off ──

    /// **The engine's document is the truth for every slot that is not being
    /// edited**, and the edit is the truth for the one that is — which is what
    /// lets an echo arrive without yanking the line out of the operator's hand.
    #[test]
    fn the_slot_being_edited_follows_the_hand_and_the_others_follow_the_engine() {
        let p = presets(&[doc("a"), doc("b"), doc("c")]);
        let mut edit: Edit = None;
        assert!(claim(&mut edit, &p, 1));
        assert_eq!(shown(&edit, &p, 1).layers[0].text, "b");
        // The operator types: the edit is what the preview draws...
        if let Some((_, d)) = edit.as_mut() {
            d.layers[0].text = "typed".into();
        }
        assert_eq!(shown(&edit, &p, 1).layers[0].text, "typed");
        // ...and an edit made elsewhere still reaches the other slots.
        assert_eq!(shown(&edit, &p, 0).layers[0].text, "a");
        assert_eq!(shown(&edit, &p, 2).layers[0].text, "c");
        // A slot the engine has never mentioned reads empty rather than panicking.
        assert!(!shown(&edit, &p, 99).is_active());
    }

    /// A write goes out once, and is not repeated while its echo is in flight.
    #[test]
    fn a_document_is_sent_once_and_not_again_until_it_changes() {
        let p = presets(&[doc("old")]);
        let mut edit: Edit = None;
        let mut sent: Edit = None;
        claim(&mut edit, &p, 0);
        assert_eq!(pending(&edit, &p, &sent), None, "an unchanged document is not written");

        if let Some((_, d)) = edit.as_mut() {
            d.layers[0].text = "new".into();
        }
        let cmd = pending(&edit, &p, &sent).expect("the change is written");
        assert!(matches!(&cmd, Command::ImageSetComposer { slot: 0, composer } if composer.layers[0].text == "new"));
        sent = edit.clone();
        assert_eq!(pending(&edit, &p, &sent), None, "not repeated before the echo");

        // The echo lands: the engine now has it, so nothing is pending.
        let echoed = presets(&[doc("new")]);
        assert_eq!(pending(&edit, &echoed, &sent), None);

        // Releasing an edited slot hands it back and writes the difference.
        if let Some((_, d)) = edit.as_mut() {
            d.layers[0].text = "final".into();
        }
        let cmd = release(&mut edit, &echoed, &sent).expect("the last edit is written");
        assert!(matches!(&cmd, Command::ImageSetComposer { slot: 0, composer } if composer.layers[0].text == "final"));
        assert!(edit.is_none(), "the claim is released");
    }

    /// An empty document is not active, so a preset nobody has composed still
    /// sends the picture it always did, banner and caption included.
    #[test]
    fn an_empty_document_falls_back_to_the_banner() {
        assert!(!Composer::default().is_active());
        let mut c = Composer::default();
        c.layers.push(TextLayer::default());
        assert!(!c.is_active(), "a layer with no text draws nothing");
        c.layers[0].text = "HI".into();
        assert!(c.is_active());
        c.layers[0].hidden = true;
        assert!(!c.is_active());
    }

    /// A grey frame of the right size is what a document with no picture under
    /// it composes onto.
    #[test]
    fn an_empty_slot_composes_onto_a_flat_frame() {
        let c = doc("HI");
        let (img, w, h) = render(&c, 64, 48, None);
        assert_eq!(img.len(), 64 * 48 * 3);
        assert_eq!((w, h), (64, 48));
    }

    /// For the eye, and only when asked: a sample composition to look at.
    /// A layout is judged by looking, not by counting pixels.
    #[test]
    fn a_sample_layout_can_be_dumped_to_look_at() {
        if std::env::var("SDROXIDE_COMPOSER_DUMP").is_err() {
            return;
        }
        let mut c = Composer::default();
        c.layers.push(TextLayer {
            text: "19DCG373".into(),
            face: Face::Mono,
            x: 0.5,
            y: 0.08,
            px: 30,
            band: Some(Strip { thickness: 44, fill: [150, 20, 20], fill2: Some([20, 0, 0]) }),
            ..Default::default()
        });
        c.layers.push(TextLayer {
            text: "CQ CQ DE 19DCG373".into(),
            face: Face::Chakra,
            x: 0.5,
            y: 0.55,
            px: 26,
            ink: [255, 230, 60],
            ..Default::default()
        });
        c.layers.push(TextLayer {
            text: "PSE K".into(),
            face: Face::ChakraBold,
            x: 0.5,
            y: 0.75,
            px: 22,
            ..Default::default()
        });
        c.layers.push(TextLayer {
            text: "JO22".into(),
            face: Face::Mono,
            x: 0.07,
            y: 0.5,
            px: 20,
            vertical: true,
            band: Some(Strip { thickness: 26, fill: [0, 40, 120], fill2: Some([0, 0, 30]) }),
            ..Default::default()
        });
        let (img, w, h) = render(&c, 320, 256, None);
        let png = crate::sstv::encode_png(&img, w, h).unwrap();
        std::fs::write("/tmp/opencode/composer-sample.png", png).unwrap();
        eprintln!("wrote /tmp/opencode/composer-sample.png");
        let _ = frame(w, h);
    }
}
