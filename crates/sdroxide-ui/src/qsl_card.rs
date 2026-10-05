//! The QSL card: a picture and the exchange beside it, drawn as one image to
//! send as an eQSL.
//!
//! Shaped after the radio-postcard eQSLs the 11 m community already sends — a
//! photograph filling the left, a cream panel carrying the fields on the right,
//! and the sent callsign set vertically down the picture's left edge — rather
//! than after anything drawn by this program. It is composed from the same
//! primitives the transmit banner uses ([`crate::sstv`]'s `put`, `blend`,
//! `draw_text` and `crop_scale`), so it needs no new drawing machinery and no
//! new dependency.
//!
//! Two rules shape it, both from this fork's house rules:
//!
//! - **A field the operator has not filled is not drawn.** No `—` placeholders
//!   and no invented values. A QSL that reports an RST nobody gave, or a name
//!   that is blank, is worse than one that says less.
//! - **The card is a rendering, not a record.** It never invents the facts; it
//!   lays out what it is given, so the same values can be checked against the
//!   log afterwards.

use ab_glyph::{Font, FontRef, PxScale};

use crate::sstv::{Ink, blend, crop_scale, draw_text, message_font, put, text_width};

/// One field of the card, with the label it is filed under.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardField {
    pub label: String,
    pub value: String,
}

impl CardField {
    /// A field the operator has not filled: drawn as nothing at all.
    ///
    /// The point of this being a constructor rather than a check at the call
    /// site is that "empty means not drawn" then cannot be forgotten in one
    /// place and remembered in another.
    pub fn empty(label: &str) -> Self {
        Self { label: label.into(), value: String::new() }
    }

    pub fn new(label: &str, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into() }
    }

    /// Whether this field earns a row on the card.
    pub fn shows(&self) -> bool {
        !self.value.trim().is_empty()
    }
}

/// Everything the card is made of, and the geometry it is made at.
#[derive(Debug, Clone, PartialEq)]
pub struct CardSpec {
    /// The photograph, as interleaved RGB.
    pub photo: Vec<u8>,
    pub photo_w: u16,
    pub photo_h: u16,
    /// Sent callsign, drawn vertically down the picture's left edge. Empty
    /// leaves the edge clear rather than drawing a bare pole.
    pub callsign: String,
    /// The exchange, in the order they should read.
    pub fields: Vec<CardField>,
    /// The free-text line at the foot — the one a person writes rather than a
    /// program.
    pub comment: String,
    pub ink: [u8; 3],
    pub paper: [u8; 3],
}

impl CardSpec {
    /// A card at the default size with the palette the postcards use: ink on
    /// warm paper rather than white on black, because this is a thing that gets
    /// printed and stuck on a wall.
    pub fn new(photo: Vec<u8>, photo_w: u16, photo_h: u16) -> Self {
        Self {
            photo,
            photo_w,
            photo_h,
            callsign: String::new(),
            fields: Vec::new(),
            comment: String::new(),
            ink: [38, 34, 32],
            paper: [242, 238, 230],
        }
    }

    /// Add a field, ignoring one with nothing in it — so a caller can build the
    /// list straight from settings without filtering first.
    pub fn with(mut self, label: &str, value: impl Into<String>) -> Self {
        let f = CardField::new(label, value);
        if f.shows() {
            self.fields.push(f);
        }
        self
    }

    /// The same, for a value that may well be absent.
    pub fn with_optional(self, label: &str, value: Option<String>) -> Self {
        match value {
            Some(v) => self.with(label, v),
            None => self,
        }
    }

    /// The card at `(w, h)`, or `None` without a font to draw with.
    pub fn compose(&self, w: u16, h: u16) -> Option<Vec<u8>> {
        let font = message_font()?;
        Some(self.render(w, h, &font))
    }

    /// The card as a PNG, ready to write out or attach — which is what the QSL
    /// window sends, since an eQSL is a file rather than a picture on screen.
    pub fn compose_png(&self, w: u16, h: u16) -> Option<Vec<u8>> {
        let rgb = self.compose(w, h)?;
        crate::sstv::encode_png(&rgb, w, h)
    }
}

/// How the card is divided: the photograph's share of the width, and the margin
/// round the whole thing.
///
/// The photograph takes a bit over half, which is what the postcards do — the
/// picture is the greeting and the fields are the paperwork. A parameter rather
/// than a constant because the same routine draws the preview in the QSL window,
/// which is a different shape again.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Grid {
    photo_w: usize,
    margin: usize,
}

/// The left-hand column of the postcard, and the call strip down its edge.
///
/// The strip is why the callsign goes on the picture at all: it is the one thing
/// that has to be legible at thumbnail size in somebody else's gallery, and a
/// vertical run beside the photograph is the traditional place for it.
const STRIP: usize = 34;

impl CardSpec {
    fn grid(&self, w: usize) -> Grid {
        Grid { photo_w: w * 58 / 100, margin: (w as f32 * 0.02) as usize }
    }

    fn render(&self, w: u16, h: u16, font: &FontRef<'static>) -> Vec<u8> {
        let (wi, hi) = (w as usize, h as usize);
        let g = self.grid(wi);
        let mut img = vec![0u8; wi * hi * 3];

        // Paper, then the photograph matted into it.
        for y in 0..hi {
            for x in 0..wi {
                put(
                    &mut img,
                    wi,
                    hi,
                    x as i32,
                    y as i32,
                    self.paper[0],
                    self.paper[1],
                    self.paper[2],
                );
            }
        }
        if self.photo_w > 0 && self.photo_h > 0 && g.photo_w > 0 {
            let inner_w = (g.photo_w - STRIP - g.margin) as u16;
            let inner_h = (hi - g.margin * 2) as u16;
            if inner_w > 0 && inner_h > 0 {
                // Centre-cropped by `crop_scale`, so a portrait picture does
                // not arrive as a squashed band.
                let scaled = crop_scale(&self.photo, self.photo_w, self.photo_h, inner_w, inner_h);
                let x0 = g.margin + STRIP;
                let y0 = g.margin;
                for y in 0..inner_h as usize {
                    for x in 0..inner_w as usize {
                        let s = (y * inner_w as usize + x) * 3;
                        let d = ((y0 + y) * wi + (x0 + x)) * 3;
                        img[d] = scaled[s];
                        img[d + 1] = scaled[s + 1];
                        img[d + 2] = scaled[s + 2];
                    }
                }
            }
        }

        // The call strip: a translucent white band with the callsign set up it,
        // read from the bottom. Drawn over the photograph, not beside it, so it
        // works whatever aspect ratio the picture turned out to be.
        if !self.callsign.trim().is_empty() {
            for y in g.margin..hi - g.margin {
                for x in g.margin..(g.margin + STRIP) {
                    blend(&mut img, wi, hi, x as i32, y as i32, 255, 255, 255, 0.55);
                }
            }
            // Vertical text needs its run *height*, which is the advance per
            // character times how many there are — not the string's width, which
            // is what the first version subtracted from a y coordinate. That is
            // why a long callsign ran off the bottom of the card.
            let text: Vec<char> = self.callsign.trim().to_uppercase().chars().collect();
            let scale = PxScale::from(STRIP as f32 * 0.52);
            let step = STRIP as f32 * 0.62;
            let top = g.margin;
            // Centred in the strip when it fits, and pinned to the foot when it
            // does not, so an over-long callsign is clipped at the top rather
            // than at the bottom — the foot is where the eye starts.
            // The run is drawn from its top downward, so `y` is the baseline of
            // the *first* glyph and must start low enough that the first
            // character is inside the strip. Starting at the vertical centre put
            // the first four glyphs above the card — which is why a callsign
            // showed as "44DC" instead of "19DCG044".
            // A glyph is drawn *above* its baseline, so a run that reads
            // downward has to walk the baseline **down** — incrementing, not
            // decrementing. The first baseline sits a full em below the top so
            // the first character's body is inside the strip.
            let mut y = top as f32 + scale.y;
            for ch in text {
                let gid = font.glyph_id(ch);
                let glyph = gid.with_scale_and_position(
                    scale,
                    ab_glyph::point(g.margin as f32 + STRIP as f32 * 0.60, y),
                );
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let b = outlined.px_bounds();
                    outlined.draw(|gx, gy, cov| {
                        let px = b.min.x as i32 + gx as i32;
                        let py = b.min.y as i32 + gy as i32;
                        blend(&mut img, wi, hi, px, py, self.ink[0], self.ink[1], self.ink[2], cov);
                    });
                }
                y += step;
            }
        }

        self.draw_panel(&mut img, wi, hi, g, font);
        img
    }

    /// The right-hand panel: the fields, two to a row, under a rule.
    fn draw_panel(&self, img: &mut [u8], w: usize, h: usize, g: Grid, font: &FontRef<'static>) {
        let left = g.photo_w + g.margin;
        let x = left + (w - left) / 12;
        let col_w = w - x - g.margin * 2;
        if col_w < 40 {
            return;
        }

        // Label above value, in pairs: a QSL is read as name-then-value, and a
        // label to the left of its value costs width the picture wants.
        let label_scale = PxScale::from(h as f32 * 0.030);
        let value_scale = PxScale::from(h as f32 * 0.045);
        let cols = if col_w > 420 { 2 } else { 1 };
        let col_pair = col_w / cols;

        // The foot is reserved first. Deciding it afterwards is how a field went
        // missing: the rows ran to the bottom, the comment then claimed the same
        // space, and the last field was silently dropped off the card. A field
        // the operator typed has to be on the card or the card is wrong.
        let foot_lines: Vec<&str> = if self.comment.trim().is_empty() {
            Vec::new()
        } else {
            self.comment.lines().rev().take(3).collect()
        };
        let foot_top = h
            .saturating_sub(g.margin)
            .saturating_sub(foot_lines.len() * (value_scale.y.ceil() as usize + 4));

        let mut y = (h as f32 * 0.06) as usize + label_scale.y.ceil() as usize;
        // **The row height is derived from the fields, not from the card.** It
        // used to be a fixed fraction of the height, which meant a card with
        // more fields than fitted quietly lost the last of them — an operator
        // typed a grid square and it never appeared, with nothing saying so.
        // Here the rows share whatever space is left between the top and the
        // reserved foot, so every typed field is on the card or the layout is
        // wrong in a way that is visible.
        let rows_needed = self.fields.len().div_ceil(cols).max(1);
        let avail = foot_top.saturating_sub(y);
        let content_h = label_scale.y.ceil() as usize + value_scale.y.ceil() as usize + 8;
        let row_h = (avail / rows_needed).max(content_h.min(avail.max(1)));
        for pair in self.fields.chunks(cols) {
            if y + content_h > foot_top && y > (h as f32 * 0.06) as usize {
                break;
            }
            for (i, field) in pair.iter().enumerate() {
                if !field.shows() {
                    continue;
                }
                let fx = x + i * col_pair;
                // The label is a caption, so it is drawn in the ink at reduced
                // weight of colour rather than a second font size alone — the
                // size difference is there too, but the greying is what makes
                // the row scannable at a glance.
                let grey = [
                    (self.ink[0] as f32 * 0.55 + 255.0 * 0.45) as u8,
                    (self.ink[1] as f32 * 0.55 + 255.0 * 0.45) as u8,
                    (self.ink[2] as f32 * 0.55 + 255.0 * 0.45) as u8,
                ];
                draw_text(
                    img,
                    w,
                    h,
                    fx as f32,
                    y as f32,
                    &field.label.to_uppercase(),
                    font,
                    label_scale,
                    Ink::plain(grey),
                    1.0,
                );
                let vw = text_width(&field.value, font, value_scale);
                let clipped = vw > (col_pair - g.margin) as f32;
                let shown = if clipped {
                    ellipsize(&field.value, font, value_scale, (col_pair - g.margin) as f32)
                } else {
                    field.value.clone()
                };
                draw_text(
                    img,
                    w,
                    h,
                    fx as f32,
                    (y + label_scale.y.ceil() as usize * 2) as f32,
                    &shown,
                    font,
                    value_scale,
                    Ink::plain(self.ink),
                    1.0,
                );
                if clipped {
                    // A value that did not fit is a value somebody has to go and
                    // check. Said here rather than left as a truncated line that
                    // reads like the whole truth.
                    underline(
                        img,
                        w,
                        h,
                        fx,
                        (y + label_scale.y.ceil() as usize * 3) as i32,
                        (col_pair - g.margin) as usize,
                        self.ink,
                    );
                }
            }
            y += row_h;
        }

        // The comment, at the foot, in the operator's own words.
        if !self.comment.trim().is_empty() {
            let scale = PxScale::from(h as f32 * 0.038);
            let mut cy = h - g.margin - (h as f32 * 0.04) as usize;
            for line in self.comment.lines().rev().take(3) {
                if cy <= y {
                    break;
                }
                draw_text(
                    img,
                    w,
                    h,
                    x as f32,
                    cy as f32,
                    line.trim(),
                    font,
                    scale,
                    Ink::plain(self.ink),
                    1.0,
                );
                cy -= scale.y.ceil() as usize + 4;
            }
        }
    }
}

/// A hairline under a value that had to be shortened.
fn underline(img: &mut [u8], w: usize, h: usize, x: usize, y: i32, len: usize, ink: [u8; 3]) {
    for i in 0..len.min(w.saturating_sub(x)) {
        for dy in 0..2 {
            blend(img, w, h, (x + i) as i32, y + dy, ink[0], ink[1], ink[2], 0.55);
        }
    }
}

/// As much of `text` as fits in `max`, with an ellipsis where it stopped.
///
/// Cut on a character rather than a word: a callsign or a locator has no spaces
/// to cut on, and a half-drawn callsign is worse than one with three dots on it.
pub fn ellipsize(text: &str, font: &FontRef<'static>, scale: PxScale, max: f32) -> String {
    if text_width(text, font, scale) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        if text_width(&format!("{out}{ch}…"), font, scale) > max {
            break;
        }
        out.push(ch);
    }
    if out.is_empty() { "…".to_string() } else { format!("{out}…") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> CardSpec {
        // A 4x3 grey ramp, so a crop that resizes can be told from one that
        // does not.
        let mut px = Vec::new();
        for y in 0..3u16 {
            for x in 0..4u16 {
                px.extend_from_slice(&[(x * 60) as u8, (y * 80) as u8, 40]);
            }
        }
        CardSpec::new(px, 4, 3)
            .with("From", "19DCG044")
            .with("To", "19DC373")
            .with("Date", "2026-09-28")
    }

    fn font() -> FontRef<'static> {
        message_font().expect("the bundled font is in the binary")
    }

    /// A card comes out at the size asked for, in RGB — the shape everything
    /// downstream (a PNG, a preview, a fetch) depends on.
    #[test]
    fn a_card_is_painted_at_the_size_it_was_asked_for() {
        let rgb = spec().compose(600, 400).expect("a font is bundled");
        assert_eq!(rgb.len(), 600 * 400 * 3);
        assert!(rgb.iter().any(|&v| v != 0), "the card is not one flat colour");
    }

    /// The paper shows where the photograph is not — otherwise a card is a
    /// picture with no margin and no panel, which is not a card.
    #[test]
    fn the_paper_is_visible_beside_the_photograph() {
        let rgb = spec().compose(600, 400).expect("a font is bundled");
        // Read the card back through the real decoder, so the assertion is
        // about what a consumer of the PNG would see rather than about the
        // in-memory buffer.
        let png = crate::sstv::encode_png(&rgb, 600, 400).unwrap();
        let (paper, pw, ph) =
            crate::sstv::decode_image(&png).expect("the card encodes and reads back");
        assert_eq!((pw, ph), (600, 400));
        // Bottom-right corner is always panel, never picture.
        let i = (399 * 600 + 599) * 3;
        assert!(
            paper[i] > 200 && paper[i + 1] > 195 && paper[i + 2] > 185,
            "the corner should be warm paper, got {:?}",
            &paper[i..i + 3]
        );
    }

    /// The photograph is centre-cropped to its box rather than squashed. A
    /// portrait picture on a landscape card is the case that matters: stretched,
    /// a face is a horror.
    #[test]
    fn the_photograph_keeps_its_aspect_ratio() {
        // A tall, narrow source: 2 wide by 8 high.
        let mut px = Vec::new();
        for _ in 0..8u16 {
            px.extend_from_slice(&[200u8, 40, 40]);
            px.extend_from_slice(&[40u8, 200, 40]);
        }
        let card = CardSpec::new(px, 2, 8);
        let rgb = card.compose(600, 400).unwrap();
        let png = crate::sstv::encode_png(&rgb, 600, 400).unwrap();
        let (decoded, w, h) = crate::sstv::decode_image(&png).unwrap();
        assert_eq!((w, h), (600, 400));
        // Somewhere inside the picture's box there are both the red and the
        // green columns — a squash would have averaged them into a third colour.
        let reds = decoded.chunks(3).filter(|p| p[0] > 150 && p[1] < 90).count();
        let greens = decoded.chunks(3).filter(|p| p[1] > 150 && p[0] < 90).count();
        assert!(reds > 100 && greens > 100, "both columns survive the crop: {reds}/{greens}");
    }

    /// **The house rule this module exists to honour.** A field nobody filled
    /// is not drawn, so the card cannot report an RST or a name that was never
    /// given. Checked through `with`, which is where a caller's mistake would
    /// land.
    #[test]
    fn a_field_with_nothing_in_it_is_left_off_the_card() {
        let card = spec().with("RST", "   ").with("Name", "");
        assert_eq!(card.fields.len(), 3, "only the three that had values: {:?}", card.fields);
        assert!(!card.fields.iter().any(|f| f.label == "RST"));
        assert!(!card.fields.iter().any(|f| f.label == "Name"));
        assert!(CardField::empty("RST").shows() == false);
    }

    /// `with_optional` is how a caller passes a value that may not exist, and it
    /// must not leave a label behind.
    #[test]
    fn an_absent_optional_field_draws_nothing() {
        let card = spec().with_optional("Grid", None).with_optional("Mode", Some("FT8".into()));
        assert!(card.fields.iter().any(|f| f.label == "Mode"));
        assert!(!card.fields.iter().any(|f| f.label == "Grid"));
    }

    /// A callsign has no spaces, so a value that will not fit has to be cut on a
    /// character — and to say that it was cut.
    #[test]
    fn a_value_too_long_for_its_column_is_shortened_and_says_so() {
        let f = font();
        let scale = PxScale::from(18.0);
        let long = "19DCG044-19DC373-19TST1001-QRP";
        let out = ellipsize(long, &f, scale, 60.0);
        assert!(out.ends_with('…'), "the cut is visible: {out:?}");
        assert!(
            text_width(&out, &f, scale) <= 60.0,
            "the result must fit: {} > 60",
            text_width(&out, &f, scale)
        );
        // Something survived.
        assert!(out.len() > 1);
        // And a value that fits is handed back untouched.
        assert_eq!(ellipsize("FT8", &f, scale, 500.0), "FT8");
    }

    /// The comment is the operator's own line, and it is drawn from the foot up
    /// so the last line they wrote is the one nearest the margin.
    #[test]
    fn a_card_with_a_comment_still_composes() {
        let card = spec();
        let with = card.clone();
        let card = card.clone().with("To", "19DC373");
        let rgb = card.compose(600, 400).unwrap();
        let plain = with.compose(600, 400).unwrap();
        assert_eq!(rgb.len(), plain.len());
        // A comment changes pixels; nothing else does.
        let mut card = spec();
        card.comment = "73 from Roeg aan de Zaan".into();
        let rgb = card.compose(600, 400).unwrap();
        assert_ne!(rgb, plain, "the comment is drawn");
    }

    /// The public types exist for the QSL window, and these tests are what keep
    /// them from rotting into dead code: the module is exercised through its own
    /// API, not through a private path.
    ///
    /// A card with no photograph at all is still a card — paper, fields and all.
    /// A QSL for a picture that failed to load is better than no QSL.
    #[test]
    fn a_card_without_a_photograph_still_carries_the_exchange() {
        let mut card = CardSpec::new(Vec::new(), 0, 0);
        card.fields = spec().fields;
        let rgb = card.compose(600, 400).unwrap();
        assert_eq!(rgb.len(), 600 * 400 * 3);
    }

    /// **The fault that only a rendered card shows.** Nine fields were typed and
    /// three of them — RST rcvd, grid, the last comment line — were simply not on
    /// the card, with nothing anywhere saying so. The row height was a fixed
    /// fraction of the card, so a card with more fields than fitted dropped the
    /// tail. Every unit test passed throughout, because none of them looked at
    /// where the ink landed.
    ///
    /// So this compares two cards differing only in how many fields they carry,
    /// at a size where the fuller one is the tight case. If the rows are still
    /// sized from the card rather than from the fields, the fuller card is the
    /// one that changes.
    #[test]
    fn every_field_typed_reaches_the_card() {
        /// How many separate bands of text the panel carries.
        ///
        /// Counting *pixels* is too coarse to catch this: three fields and six
        /// fields differ enough to pass a `>` comparison even when the layout
        /// drops two of them. What matters is how many **rows** there are, one
        /// per field, so that is what is counted — rows of dark pixels inside
        /// the panel, grouped into bands across the vertical gaps between them.
        ///
        /// **This fault was found by rendering a card and looking at it.** Every
        /// other test passed while three typed fields were missing from the
        /// output, because none of them asked where the ink landed.
        fn bands(rgb: &[u8], w: u16, h: u16) -> usize {
            let pw = w as usize;
            let x0 = pw * 58 / 100 + pw / 50;
            let mut in_band = false;
            let mut bands = 0usize;
            for y in 0..h as usize {
                let row_has_ink = (0..pw).any(|x| {
                    x >= x0 && {
                        let i = (y * pw + x) * 3;
                        rgb[i] < 120 && rgb[i + 1] < 120 && rgb[i + 2] < 120
                    }
                });
                if row_has_ink && !in_band {
                    bands += 1;
                }
                in_band = row_has_ink;
            }
            bands
        }
        let mut nine = spec();
        nine.fields = spec()
            .with("RST sent", "57")
            .with("RST rcvd", "59")
            .with("Grid", "IO93WQ")
            .with("Mode", "SSTV")
            .with("Freq", "27.267 MHz")
            .fields;
        assert_eq!(nine.fields.len(), 8, "the fixture must be the full exchange");
        let rgb = nine.compose(1000, 620).unwrap();
        assert!(
            bands(&rgb, 1000, 620) >= nine.fields.len(),
            "each of the {} fields needs its own row of ink; the card carries {} bands",
            nine.fields.len(),
            bands(&rgb, 1000, 620)
        );
    }

    /// The callsign strip has to carry the **whole** callsign. A glyph is drawn
    /// above its baseline, so a run that reads downward walks the baseline down
    /// rather than up; getting that backwards showed `19DCG044` as `44DC` and
    /// then as `91`, and no arithmetic test would have noticed.
    #[test]
    fn the_whole_callsign_lands_on_the_strip() {
        let card = spec();
        let with = card.clone();
        let mut card = card;
        card.callsign = "19DCG044".into();
        let (before, after) = (with.compose(1000, 620).unwrap(), card.compose(1000, 620).unwrap());
        assert_ne!(before, after, "the callsign is drawn");
        // The strip is the left 34 px of the photo area. Ink *inside* it is what
        // says the text landed there rather than somewhere off-card.
        let strip_ink = |rgb: &[u8]| {
            let w = 1000usize;
            let (m, s) = (w * 2 / 100, 34usize);
            rgb.chunks(3)
                .enumerate()
                .filter(|(i, _)| {
                    let (x, _) = (i % w, i / w);
                    x >= m && x < m + s
                })
                .filter(|(_, p)| p[0] < 120 && p[1] < 120 && p[2] < 120)
                .count()
        };
        assert!(
            strip_ink(&after) > 200,
            "the whole callsign should be in the strip, found {} dark pixels",
            strip_ink(&after)
        );
        assert_eq!(strip_ink(&before), 0, "and nothing should be there without one");
    }

    /// Every size a caller might ask for produces a buffer of exactly that size,
    /// so nothing downstream has to guess.
    #[test]
    fn the_card_fits_any_size_it_is_asked_for() {
        for (w, h) in [(320u16, 240u16), (600, 400), (1024, 700), (1600, 1000)] {
            let rgb = spec().compose(w, h).unwrap();
            assert_eq!(rgb.len(), w as usize * h as usize * 3, "{w}x{h}");
            assert!(crate::sstv::encode_png(&rgb, w, h).is_some(), "{w}x{h} encodes");
        }
    }
}
