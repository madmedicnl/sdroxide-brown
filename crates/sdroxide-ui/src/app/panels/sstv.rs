//! The SSTV and RIFP image panel.
//!
//! [`SstvUi`] draws two things the engine owns and one it does not. The five
//! transmit presets and the gallery of received pictures live on the machine
//! the radio is plugged into — the panel holds metadata, thumbnails and
//! whatever pixels it has been handed, and asks for the rest. The picture
//! currently arriving is the exception: scanlines are painted into a texture as
//! they come, so a picture builds up on screen exactly as it does on the air.
//!
//! Compositing stays here. The crop, the header strip and the overlay text are
//! applied to the fetched source client-side, so the preview redraws on every
//! keystroke without a round trip, and only the finished picture goes back to
//! the engine as an ordinary `SstvTx` / `RifpTx`.

use crate::i18n::tr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, RichText};
use sdroxide_types::{
    Command, IMAGE_SLOTS, IMAGE_UPLOAD_MAX, ImageEntry, ImageKind, ImagePresets, Mode,
    RifpEncoding, RifpProfile, RifpSize, RifpStatus, SstvMode, SstvStatus,
};

use crate::theme::ThemedScroll;

use crate::app::panels::widgets::{pick_image, sstv_section};
use crate::app::util::shorten;
use crate::app::{SdroxideApp, rx_only_hint};

// ───────────────────────────── SSTV panel ──────────────────────────────

/// A fetched transmit source: the picture the compositor works from, and which
/// stored version it is, so a slot changed from another screen is refetched
/// rather than composed from the old photograph.
pub(in crate::app) struct SstvSlot {
    src_rgb: Vec<u8>,
    sw: u16,
    sh: u16,
    version: u32,
}

/// Pictures the gallery will hold thumbnails for.
///
/// Not a limit on the collection — the engine counts the whole store and the
/// panel says how many more there are — but on what this client keeps as
/// textures.
const GALLERY_MAX: usize = 240;

/// Points of row still needed beside the cursor before the TX-slant cluster
/// can start on the current line: the `add_space` + separator + "TX slant"
/// prefix measured ~69 and the ppm slider ~140 at a 960 pt window. Below
/// this the cluster wraps to a fresh line instead (issue #643).
const TX_TRIM_MIN_W: f32 = 230.0;

/// A received-picture gallery entry.
pub(in crate::app) struct SstvRecv {
    entry: ImageEntry,
    thumb: egui::TextureHandle,
    /// The picture at full size, once it has been fetched — or promoted
    /// straight from the one that had just been received, which the panel
    /// already held. `None` until the operator enlarges it.
    full: Option<egui::TextureHandle>,
}

/// Image-panel state, shared by SSTV and RIFP: received gallery, in-progress
/// incoming picture, transmit slots, the overlay message, the current mode, and
/// cached textures.
///
/// One workspace for both modes on purpose. The pictures an operator wants to
/// send, the captions on them, and the pictures that came back are the same
/// things whichever protocol carried them; only the control strip and the
/// transmit sizing differ.
pub(in crate::app) struct SstvUi {
    pub(in crate::app) tx_mode: SstvMode,
    /// Latest RIFP engine status (transfer progress, sessions, counters).
    pub(in crate::app) rifp: RifpStatus,
    /// Size of the picture currently arriving, so the live canvas can be built
    /// before the whole object is in. `(0, 0)` when nothing is arriving.
    pub(in crate::app) rx_dims: (u16, u16),
    /// Size the cached preview was composed at, so a change of transmit size
    /// rebuilds it.
    pub(in crate::app) preview_dims: (u16, u16),
    /// The banner drawn across the top of the transmit image, resolved from the
    /// station's digital-mode config. `None` when the operator switched it off.
    ///
    /// Resolved here rather than at each compose because it is what tells the
    /// preview it is stale: the strip changes when the callsign changes, when a
    /// template is edited, when a colour is picked, and comparing the composed
    /// banner catches all of that with one test.
    pub(in crate::app) banner: Option<crate::sstv::Banner>,
    /// How the banner and message are styled — the gradient, outlines and ink.
    /// Kept beside the banner for the same staleness test: a colour picked in
    /// the editor changes this and the preview is recomposed.
    pub(in crate::app) style: sdroxide_types::SstvStyle,
    /// Whether the banner editor window is open.
    pub(in crate::app) banner_open: bool,
    /// Auto mode: RX auto-detects the mode; TX defaults to Martin 1 until a mode
    /// is heard or the operator picks one.
    pub(in crate::app) auto: bool,
    /// The engine's transmit presets: what is in each slot and the message over
    /// it. The truth — `slots` below only caches its pixels.
    pub(in crate::app) presets: ImagePresets,
    /// Thumbnails of the presets, rebuilt when the engine announces a change.
    pub(in crate::app) slot_thumbs: Vec<Option<egui::TextureHandle>>,
    /// The message being typed and which slot it belongs to. Only that one slot
    /// is client-owned, so an echo of our own write — or an edit made on another
    /// screen — can update the other four without fighting the keyboard.
    pub(in crate::app) msg_edit: Option<(usize, String)>,
    /// Fetched source pictures, keyed by slot; `None` until one arrives.
    pub(in crate::app) slots: Vec<Option<SstvSlot>>,
    /// The version last asked for per slot.
    ///
    /// Deliberately not cleared when the answer lands: it records the *ask*, so
    /// a picture that fails to decode is asked for once rather than once a
    /// frame for ever. A slot changed on the radio arrives with a new version,
    /// which is what makes the next ask happen.
    pub(in crate::app) src_asked: HashMap<usize, u32>,
    pub(in crate::app) selected_slot: usize,
    pub(in crate::app) received: Vec<SstvRecv>,
    /// How many pictures the store holds altogether, so the gallery can say
    /// exactly how many older ones there are rather than guessing.
    pub(in crate::app) total: u32,
    /// Where the engine is saving received pictures, for the gallery to show.
    pub(in crate::app) dir: String,
    /// True while a listing page is outstanding — the reliable lane is finite
    /// and a scroll must not spray requests down it.
    pub(in crate::app) page_pending: bool,
    /// Name of the picture whose full size has been asked for. Like
    /// `src_asked`, it records the ask and is cleared when the viewer moves on,
    /// not when the answer arrives.
    pub(in crate::app) full_asked: Option<String>,
    /// The answer to that ask was empty — the store no longer has it.
    pub(in crate::app) full_gone: bool,
    /// In-progress incoming image (painted line-by-line).
    pub(in crate::app) rx_color: Option<egui::ColorImage>,
    pub(in crate::app) rx_tex: Option<egui::TextureHandle>,
    pub(in crate::app) rx_id: u32,
    /// The picture that has just finished arriving, held until the engine names
    /// the file it went into. Enlarging it then costs nothing, where fetching
    /// back what the panel was handed a moment ago would cost a megabyte.
    pub(in crate::app) fresh: Option<(egui::TextureHandle, Vec<u8>)>,
    /// The bytes of the full-size picture last fetched or received, and its
    /// name. One at a time: this is what "Save picture…" writes out, and
    /// keeping every picture's bytes for a button nobody may press would hold
    /// a gallery's worth of megabytes for nothing.
    pub(in crate::app) full_png: Option<(String, Vec<u8>)>,
    pub(in crate::app) status: SstvStatus,
    /// Received-gallery index currently shown enlarged in an overlay window.
    pub(in crate::app) enlarged: Option<usize>,
    /// Name of the picture whose DELETE has been pressed once and is waiting to
    /// be pressed again.
    ///
    /// Two presses rather than a modal, because the file goes for good and there
    /// is no undo to offer: a stray click on a chip beside SAVE would otherwise
    /// be the last anyone saw of a picture. Armed per name, so it cannot survive
    /// into the next picture and delete that one instead.
    pub(in crate::app) confirm_delete: Option<String>,
    /// The QSL mail window, when it is open — and what it is composing. Held
    /// here rather than in the call so the fields survive a frame where the
    /// window is collapsed, and so the picture's name is remembered even
    /// after it leaves `full_png`.
    ///
    /// Not on the wire and never persisted: a mail address is nobody else's to
    /// store in a settings file, and a half-written message is not worth
    /// keeping across a session.
    pub(in crate::app) qsl: Option<SstvQsl>,
    /// What the last send press did, when it could not open a mail handler.
    pub(in crate::app) qsl_note: Option<String>,
    /// Last VIS/free-run-detected mode we auto-applied to `tx_mode`, so a steady
    /// detection doesn't keep overriding the operator's manual mode choice.
    pub(in crate::app) last_detected: Option<SstvMode>,
    pub(in crate::app) preview_tex: Option<egui::TextureHandle>,
    pub(in crate::app) preview_dirty: bool,
    /// Whether the store has been listed this session.
    pub(in crate::app) listed: bool,
    /// File-picker result inbox (raw image bytes), filled by the picker task.
    pub(in crate::app) inbox: Arc<Mutex<Option<Vec<u8>>>>,
    pub(in crate::app) pick_target: Option<usize>,
    /// A local complaint about a picked file — too big to be worth sending, or
    /// one the engine refused. Cleared when the next picture is picked.
    pub(in crate::app) pick_error: Option<String>,
    /// **The document being edited**: one slot's layers, claimed from the
    /// engine's preset store. `None` when nothing is being edited, and then every
    /// slot draws the document the engine holds.
    ///
    /// One slot at a time, for the reason the preset messages are claimed the
    /// same way: the engine's echo of our own write must not yank a line out of
    /// the hand that is dragging it, and an edit made on another screen must
    /// still reach the slots we are not touching.
    pub(in crate::app) composer_edit: crate::sstv_composer::Edit,
    /// The document last sent to the engine, so an un-echoed write is not
    /// repeated on every frame.
    pub(in crate::app) composer_sent: crate::sstv_composer::Edit,
    /// When the document last changed, so a pause — the end of typing, a drag
    /// that ended without a release — settles it to the engine.
    pub(in crate::app) composer_since: Option<f64>,
    /// The station-wide overlay as the panel last saw it in the operator's
    /// config, so a change made in the editor (or on another screen) recomposes
    /// the preview.
    pub(in crate::app) overlay: sdroxide_types::Composer,
    /// True while the editor is laying out the **station overlay** rather than
    /// the selected slot's own document.
    pub(in crate::app) editing_overlay: bool,
    /// The name being typed for a new layout template.
    pub(in crate::app) template_name: String,
    /// The saved layout being renamed, if any: the name field holds the new one
    /// and saving renames rather than adding.
    pub(in crate::app) template_renaming: Option<String>,
    /// The pictures picked for a mosaic, in the order they will be laid out, and
    /// their PNGs once fetched. At most four: four is the case this exists for.
    pub(in crate::app) mosaic_picks: Vec<String>,
    pub(in crate::app) mosaic_pngs: HashMap<String, Vec<u8>>,
    /// A complaint about the mosaic — a picture that could not be fetched, most
    /// often.
    pub(in crate::app) mosaic_note: Option<String>,
    /// Whether the composer window is open.
    pub(in crate::app) composer_open: bool,
    /// The layer whose controls are shown, and which a drag moves.
    pub(in crate::app) composer_sel: Option<usize>,
    /// The preview texture is stale and has to be recomposed.
    pub(in crate::app) composer_dirty: bool,
    /// The composed preview, as a texture.
    pub(in crate::app) composer_tex: Option<egui::TextureHandle>,
    /// A drag in progress: which layer, and where in it the pointer grabbed it
    /// (as a fraction of the frame, so the layer does not jump to the cursor).
    pub(in crate::app) composer_drag: Option<(usize, f32, f32)>,
}

impl Default for SstvUi {
    fn default() -> Self {
        SstvUi {
            tx_mode: SstvMode::Martin1,
            rifp: RifpStatus::default(),
            rx_dims: (0, 0),
            preview_dims: (0, 0),
            banner: None,
            style: sdroxide_types::SstvStyle::default(),
            banner_open: false,
            auto: true,
            presets: ImagePresets::default(),
            slot_thumbs: (0..IMAGE_SLOTS).map(|_| None).collect(),
            msg_edit: None,
            slots: (0..IMAGE_SLOTS).map(|_| None).collect(),
            src_asked: HashMap::new(),
            selected_slot: 0,
            received: Vec::new(),
            total: 0,
            dir: String::new(),
            page_pending: false,
            full_asked: None,
            full_gone: false,
            rx_color: None,
            rx_tex: None,
            rx_id: 0,
            fresh: None,
            full_png: None,
            status: SstvStatus::default(),
            enlarged: None,
            confirm_delete: None,
            qsl: None,
            qsl_note: None,
            last_detected: None,
            preview_tex: None,
            preview_dirty: true,
            listed: false,
            inbox: Arc::new(Mutex::new(None)),
            pick_target: None,
            pick_error: None,
            composer_edit: None,
            composer_sent: None,
            composer_since: None,
            overlay: sdroxide_types::Composer::default(),
            editing_overlay: false,
            template_name: String::new(),
            template_renaming: None,
            mosaic_picks: Vec::new(),
            mosaic_pngs: HashMap::new(),
            mosaic_note: None,
            composer_open: false,
            composer_sel: None,
            composer_dirty: true,
            composer_tex: None,
            composer_drag: None,
        }
    }
}

impl SstvUi {
    /// A decoded scanline arrived: paint it into the in-progress image.
    pub(in crate::app) fn on_line(&mut self, id: u32, y: u16, rgb: &[u8], ctx: &egui::Context) {
        let Some(mode) = self.status.detected else { return };
        let (w, h) = mode.dimensions();
        if self.rx_id != id || self.rx_color.is_none() {
            self.rx_id = id;
            self.rx_color =
                Some(crate::sstv::color_image(&vec![0u8; w as usize * h as usize * 3], w, h));
        }
        let Some(ci) = self.rx_color.as_mut() else { return };
        let (w, h) = (w as usize, h as usize);
        if (y as usize) < h && rgb.len() >= w * 3 {
            let row = y as usize * w;
            for x in 0..w {
                ci.pixels[row + x] = Color32::from_rgb(rgb[x * 3], rgb[x * 3 + 1], rgb[x * 3 + 2]);
            }
        }
        self.rx_tex = Some(ctx.load_texture("sstv_rx", ci.clone(), egui::TextureOptions::NEAREST));
    }

    /// A completed picture arrived. The gallery entry comes from the engine a
    /// moment later, naming the file it was saved as; what is held here is the
    /// full-size texture, so enlarging it does not fetch back what the panel
    /// has just been given.
    pub(in crate::app) fn on_image(&mut self, png: &[u8], ctx: &egui::Context) {
        self.fresh = crate::sstv::decode_image(png).map(|(rgb, w, h)| {
            let ci = crate::sstv::color_image(&rgb, w, h);
            (ctx.load_texture("sstv_recv", ci, egui::TextureOptions::NEAREST), png.to_vec())
        });
        self.rx_color = None;
        self.rx_tex = None;
    }

    /// RIFP: reassembled raster rows arrived — paint them into the live
    /// picture. Only the unencoded raster gets here; everything else appears
    /// whole in [`SstvUi::on_rifp_image`].
    pub(in crate::app) fn on_rifp_rows(
        &mut self,
        id: u32,
        y: u16,
        w: u16,
        h: u16,
        gray: &[u8],
        ctx: &egui::Context,
    ) {
        if self.rx_id != id || self.rx_color.is_none() || self.rx_dims != (w, h) {
            self.rx_id = id;
            self.rx_dims = (w, h);
            self.rx_color =
                Some(crate::sstv::color_image(&vec![0u8; w as usize * h as usize * 3], w, h));
        }
        let Some(ci) = self.rx_color.as_mut() else { return };
        let (wu, hu) = (w as usize, h as usize);
        for (row, pixels) in gray.chunks_exact(wu).enumerate() {
            let y = y as usize + row;
            if y >= hu {
                break;
            }
            for (x, &g) in pixels.iter().enumerate() {
                ci.pixels[y * wu + x] = Color32::from_gray(g);
            }
        }
        self.rx_tex = Some(ctx.load_texture("rifp_rx", ci.clone(), egui::TextureOptions::NEAREST));
    }

    /// RIFP: a complete, digest-verified picture arrived.
    pub(in crate::app) fn on_rifp_image(&mut self, png: &[u8], ctx: &egui::Context) {
        self.on_image(png, ctx);
        self.rx_dims = (0, 0);
    }

    /// The engine announced the transmit presets: adopt them, rebuild the slot
    /// thumbnails, and drop any cached source whose picture has been replaced.
    pub(in crate::app) fn on_presets(&mut self, presets: ImagePresets, ctx: &egui::Context) {
        for (i, slot) in presets.slots.iter().enumerate() {
            let held = self.slots.get(i).and_then(|s| s.as_ref()).map(|s| s.version);
            if held.is_some_and(|v| v != slot.version) || (held.is_some() && !slot.has_picture()) {
                if let Some(cell) = self.slots.get_mut(i) {
                    *cell = None;
                }
                if i == self.selected_slot {
                    self.preview_dirty = true;
                }
            }
            let thumb = (!slot.thumb.is_empty())
                .then(|| crate::sstv::decode_image(&slot.thumb))
                .flatten()
                .map(|(rgb, w, h)| {
                    let ci = crate::sstv::color_image(&rgb, w, h);
                    ctx.load_texture("sstv_slot", ci, egui::TextureOptions::LINEAR)
                });
            if let Some(cell) = self.slot_thumbs.get_mut(i) {
                *cell = thumb;
            }
        }
        // The message box takes the engine's text for every slot except the one
        // under the cursor, so an edit made elsewhere shows up.
        if self.presets.slot(self.selected_slot).message != presets.slot(self.selected_slot).message
            && !matches!(&self.msg_edit, Some((i, _)) if *i == self.selected_slot)
        {
            self.preview_dirty = true;
        }
        self.presets = presets;
    }

    /// A fetched source picture for a slot.
    pub(in crate::app) fn on_slot_source(&mut self, slot: u8, version: u32, png: &[u8]) {
        let slot = slot as usize;
        let Some(cell) = self.slots.get_mut(slot) else { return };
        *cell = crate::sstv::load_source_bounded(png, 1024).map(|(rgb, sw, sh)| SstvSlot {
            src_rgb: rgb,
            sw,
            sh,
            version,
        });
        if slot == self.selected_slot {
            self.preview_dirty = true;
            self.composer_dirty = true;
        }
    }

    /// One page of the received store.
    pub(in crate::app) fn on_listing(
        &mut self,
        listing: sdroxide_types::ImageListing,
        ctx: &egui::Context,
    ) {
        self.page_pending = false;
        self.total = listing.total;
        self.dir = listing.dir;
        for entry in listing.entries {
            self.insert_entry(entry, None, ctx);
        }
    }

    /// A picture the engine has just stored. It goes to the front of the
    /// gallery, taking the full-size texture the panel already holds from the
    /// picture that has this moment finished arriving.
    pub(in crate::app) fn on_saved(&mut self, entry: ImageEntry, ctx: &egui::Context) {
        let fresh = self.fresh.take();
        if let Some((_, png)) = &fresh {
            self.full_png = Some((entry.name.clone(), png.clone()));
        }
        self.total += 1;
        self.insert_entry(entry, fresh.map(|(tex, _)| tex), ctx);
    }

    /// A picture is no longer in the store: this screen's delete, or another's.
    ///
    /// The enlarged view is an index into the list, so it is moved with the
    /// list. Deleting the picture being looked at leaves the index on the
    /// next-older one, which is what makes culling a gallery a sequence of
    /// clicks in one place rather than a reopen after every one.
    pub(in crate::app) fn on_deleted(&mut self, name: &str) {
        let Some(at) = self.received.iter().position(|r| r.entry.name == name) else { return };
        self.received.remove(at);
        // `total` counts the whole store, of which this list is only a window.
        // It comes down for a picture that was in the window, because that is
        // the case where this client knows the store really did shrink; for one
        // past the end it is left alone and the next listing settles it.
        self.total = self.total.saturating_sub(1);
        if self.full_png.as_ref().is_some_and(|(n, _)| n == name) {
            self.full_png = None;
        }
        if self.confirm_delete.as_deref() == Some(name) {
            self.confirm_delete = None;
        }
        if let Some(v) = self.enlarged {
            let next = if at < v { v - 1 } else { v };
            self.enlarged = (next < self.received.len()).then_some(next);
            // Only a delete of the picture being looked at puts a different one
            // in the window; anything above it just shifted the same picture up.
            if at == v {
                self.full_asked = None;
                self.full_gone = false;
            }
        }
    }

    /// A fetched full-size picture. An empty answer means the store no longer
    /// has it — the file moved or was deleted between listing and opening.
    pub(in crate::app) fn on_file(&mut self, name: &str, png: &[u8], ctx: &egui::Context) {
        let Some((rgb, w, h)) = crate::sstv::decode_image(png) else {
            if self.full_asked.as_deref() == Some(name) {
                self.full_gone = true;
            }
            return;
        };
        let tex = ctx.load_texture(
            "sstv_full",
            crate::sstv::color_image(&rgb, w, h),
            egui::TextureOptions::NEAREST,
        );
        if let Some(r) = self.received.iter_mut().find(|r| r.entry.name == name) {
            r.full = Some(tex);
        }
        self.full_png = Some((name.to_string(), png.to_vec()));
        // A picture picked for a mosaic is kept until the mosaic is composed:
        // the editor asks for them one at a time and the answer lands here.
        if self.mosaic_picks.iter().any(|n| n == name) {
            self.mosaic_pngs.insert(name.to_string(), png.to_vec());
        }
    }

    /// Add a gallery entry in received order, skipping one already held.
    ///
    /// The listing and the just-arrived notification can name the same picture
    /// when one lands in the gap between the request and its answer; the name
    /// is what tells them apart.
    fn insert_entry(
        &mut self,
        entry: ImageEntry,
        full: Option<egui::TextureHandle>,
        ctx: &egui::Context,
    ) {
        if self.received.iter().any(|r| r.entry.name == entry.name) {
            return;
        }
        let Some((rgb, w, h)) = crate::sstv::decode_image(&entry.thumb) else { return };
        let thumb = ctx.load_texture(
            "sstv_thumb",
            crate::sstv::color_image(&rgb, w, h),
            egui::TextureOptions::LINEAR,
        );
        let at = self.received.partition_point(|r| r.entry.unix > entry.unix);
        self.received.insert(at, SstvRecv { entry, thumb, full });
        // The enlarged view is an index into this list, so anything inserted
        // above it moves what the operator is looking at.
        if let Some(v) = self.enlarged.as_mut() {
            if at <= *v {
                *v += 1;
            }
        }
        // A long session receives more pictures than this client will hold
        // textures for. The oldest go, not the newest — they are still in the
        // store, and `total` still counts them.
        self.received.truncate(GALLERY_MAX);
    }

    /// The overlay message for the slot currently being edited.
    pub(in crate::app) fn current_message(&self) -> &str {
        crate::sstv::message_shown(&self.msg_edit, &self.presets, self.selected_slot)
    }

    /// Rebuild the transmit preview when the size, slot, or message changed.
    /// `dims` is the transmitted picture size — the SSTV line format's, or the
    /// operator's chosen RIFP size.
    pub(in crate::app) fn ensure_preview(&mut self, dims: (u16, u16), ctx: &egui::Context) {
        if !self.preview_dirty {
            return;
        }
        self.preview_dirty = false;
        let message = self.current_message().to_string();
        match self.slots.get(self.selected_slot).and_then(|s| s.as_ref()) {
            Some(slot) => {
                let (rgb, w, h) = crate::sstv::compose(
                    dims.0,
                    dims.1,
                    &slot.src_rgb,
                    slot.sw,
                    slot.sh,
                    &message,
                    self.banner.as_ref(),
                    &self.style,
                );
                let ci = crate::sstv::color_image(&rgb, w, h);
                self.preview_tex =
                    Some(ctx.load_texture("sstv_preview", ci, egui::TextureOptions::NEAREST));
            }
            None => self.preview_tex = None,
        }
    }

    /// The composed PNG for the current selection, for transmit.
    pub(in crate::app) fn compose_png(&self, dims: (u16, u16)) -> Option<Vec<u8>> {
        let slot = self.slots.get(self.selected_slot).and_then(|s| s.as_ref())?;
        // **The composer draws the picture when it has something to say**, and
        // the banner and the message are then not drawn at all: two sets of text
        // on one picture is not a composition, it is a collision. A picture that
        // was flattened into this very slot already has the text in it, so the
        // document stands down for it.
        let own = crate::sstv_composer::shown(
            &self.composer_edit,
            &self.presets,
            self.selected_slot,
        );
        // **The station overlay goes underneath**, when this preset wants it: the
        // callsign every picture needs, with this picture's own layers over it.
        // Neither replaces the other, and a preset already carrying its
        // identification switches the overlay off for itself.
        let doc = if self.presets.slot(self.selected_slot).apply_overlay
            && self.overlay.is_active()
        {
            own.over(&self.overlay)
        } else {
            own.clone()
        };
        let doc = &doc;
        if doc.is_active() {
            let (rgb, w, h) = crate::sstv_composer::render(
                doc,
                dims.0,
                dims.1,
                Some((&slot.src_rgb, slot.sw, slot.sh)),
            );
            return crate::sstv::encode_png(&rgb, w, h);
        }
        let (rgb, w, h) = crate::sstv::compose(
            dims.0,
            dims.1,
            &slot.src_rgb,
            slot.sw,
            slot.sh,
            self.current_message(),
            self.banner.as_ref(),
            &self.style,
        );
        crate::sstv::encode_png(&rgb, w, h)
    }

    /// Send a picked file to the engine for a slot.
    ///
    /// Nothing is kept here: the engine scales the picture, stores it and
    /// announces the presets, and the slot fills in when that comes back. That
    /// round trip is the point — it is what makes the browser tab and the
    /// console show the same five pictures.
    pub(in crate::app) fn set_slot(
        &mut self,
        slot: usize,
        bytes: Vec<u8>,
        cmds: &mut Vec<Command>,
    ) {
        self.selected_slot = slot;
        // Checked here as well as engine-side: pushing forty megabytes down the
        // socket only to have it refused at the far end is a poor experience,
        // and it keeps a huge upload clear of the WebSocket's frame limit.
        if bytes.len() > IMAGE_UPLOAD_MAX {
            self.pick_error = Some(tr!(
                "sstv-picture-too-big",
                size = bytes.len() / 1_048_576,
                limit = IMAGE_UPLOAD_MAX / 1_048_576
            ));
            return;
        }
        self.pick_error = None;
        cmds.push(Command::ImageSetSlot { slot: slot as u8, bytes });
    }
}

/// One incoming RIFP transfer's chunk map: a lit cell per chunk received, dark
/// per chunk still missing. Beyond what fits, it degrades to a plain bar — the
/// point is to see *where* the holes are, and a thousand one-pixel cells show
/// nothing.
fn rifp_chunk_map(ui: &mut egui::Ui, session: &sdroxide_types::RifpSession) {
    let cells = session.total.max(session.have) as usize;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(120.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, crate::theme::gray(20));
    let have = |i: usize| session.map.get(i / 8).is_some_and(|b| b >> (i % 8) & 1 != 0);
    if cells > 0 && cells <= rect.width() as usize {
        let cw = rect.width() / cells as f32;
        for i in 0..cells {
            if !have(i) {
                continue;
            }
            let x = rect.left() + i as f32 * cw;
            p.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(cw.max(1.0), 10.0)),
                0.0,
                crate::theme::GREEN(),
            );
        }
    } else if session.total > 0 {
        let mut fill = rect;
        fill.set_width(rect.width() * (session.have as f32 / session.total as f32).clamp(0.0, 1.0));
        p.rect_filled(fill, 2.0, crate::theme::GREEN());
    }
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::gray(60)),
        egui::StrokeKind::Inside,
    );
    resp.on_hover_text(tr!("sstv-chunks-received-lit-still-missing"));
}

fn sstv_level_bar(ui: &mut egui::Ui, level: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, crate::theme::gray(20));
    // Log scale (~ -60..0 dBFS mean-abs) so weak-but-decodable signals still show.
    let db = 20.0 * level.max(1e-6).log10();
    let frac = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    let mut fill = rect;
    fill.set_width(rect.width() * frac);
    let col = if frac > 0.06 { crate::theme::GREEN() } else { crate::theme::gray(45) };
    p.rect_filled(fill, 2.0, col);
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::gray(60)),
        egui::StrokeKind::Inside,
    );
}

/// Split `total` into two columns by `frac` — the first gets `frac` of it — with
/// each side's preferred minimum honoured, **scaled down together when they
/// cannot both fit** so the pair never sums past `total`.
///
/// A fixed floor is what grew the image panel over its column when the column
/// was narrow (issue #643): two columns each wanting 300 pt summed to more than
/// the dock left, and the panel spilled over it. Scaling keeps the same
/// preference where there is room and still fits where there is not.
fn split_two(total: f32, frac: f32, min_first: f32, min_second: f32) -> (f32, f32) {
    let total = total.max(0.0);
    let wanted = min_first + min_second;
    let scale = if wanted > total && wanted > 0.0 { total / wanted } else { 1.0 };
    let (min_first, min_second) = (min_first * scale, min_second * scale);
    let first = (total * frac).clamp(min_first, (total - min_second).max(min_first));
    (first, total - first)
}

impl SdroxideApp {
    /// The image panel, shared by SSTV and RIFP: a live picture and a gallery
    /// on the left, a transmit compositor on the right, and a control strip
    /// that is the only part either mode owns alone.
    pub(in crate::app) fn image_panel(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        mode: Mode,
    ) {
        let ctx = ui.ctx().clone();
        let rifp = mode.is_rifp();
        // Say it where the operator is looking if the receiver is being driven
        // into its rails — a picture cannot decode through an overloaded front
        // end, and the panel would otherwise only ever say "hunting".
        self.front_end_overload_warning(ui);
        // The store is the engine's; list it once when the panel first opens.
        // Pictures that arrive afterwards are announced one at a time.
        if !self.sstv.listed {
            self.sstv.listed = true;
            self.sstv.page_pending = true;
            cmds.push(Command::ImageList {
                kind: ImageKind::Sstv,
                offset: 0,
                count: sdroxide_types::IMAGE_PAGE_MAX,
            });
        }
        // Drain a completed file-pick (only consume the target once bytes arrive).
        let picked = self.sstv.inbox.lock().ok().and_then(|mut g| g.take());
        if let Some(bytes) = picked {
            if let Some(target) = self.sstv.pick_target.take() {
                self.sstv.set_slot(target, bytes, cmds);
            }
        }
        // **A file path pasted into the panel, for systems without drag and
        // drop.** Wayland gives a client no way to receive a dropped file —
        // winit does not implement it there, so the drop above never arrives,
        // whatever this code does. A file manager copies a file as a `file://`
        // URI or a plain path, and Ctrl+V then means the same thing: load that
        // picture into the selected slot.
        let pasted: Option<String> = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Paste(s) => Some(s.clone()),
                _ => None,
            })
        });
        if let Some(text) = pasted {
            let path = text.trim();
            let path = path.strip_prefix("file://").unwrap_or(path);
            let looks_like_a_picture = [".png", ".jpg", ".jpeg", ".bmp", ".gif", ".webp"]
                .iter()
                .any(|ext| path.to_ascii_lowercase().ends_with(ext));
            if looks_like_a_picture
                && let Ok(bytes) = std::fs::read(path)
            {
                let target = self.sstv.selected_slot;
                self.sstv.set_slot(target, bytes, cmds);
            }
        }

        // **A picture dropped on the panel goes into the selected slot**, the
        // same path a picked file takes — and the engine is what stores it, so
        // another screen attached to the same radio sees it too. Accepted from a
        // path or from bytes, whichever the platform gave us.
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(bytes) = dropped.iter().find_map(|p| std::fs::read(p).ok()) {
            let target = self.sstv.selected_slot;
            self.sstv.set_slot(target, bytes, cmds);
        }

        // Fetch the selected slot's source when what is cached is not what the
        // engine says is stored — a slot loaded here, or changed from another
        // screen. Asked once, not once per frame.
        let sel = self.sstv.selected_slot;
        let want = self.sstv.presets.slot(sel);
        let have = self.sstv.slots.get(sel).and_then(|s| s.as_ref()).map(|s| s.version);
        if want.has_picture()
            && have != Some(want.version)
            && self.sstv.src_asked.get(&sel) != Some(&want.version)
        {
            self.sstv.src_asked.insert(sel, want.version);
            cmds.push(Command::ImageGetSlot(sel as u8));
        }
        // Keep the banner in sync with the operator config — the callsign it
        // prints, the two templates, the colours and the height.
        let banner = crate::sstv::Banner::from_config(&self.digi_cfg_edit);
        if self.sstv.banner != banner {
            self.sstv.banner = banner;
            self.sstv.preview_dirty = true;
        }
        // The station overlay composes under every preset's own document, so a
        // change to it — from this screen's editor, from another client, or from
        // a config that arrived late — invalidates both previews.
        if self.sstv.overlay != self.digi_cfg_edit.station_overlay {
            self.sstv.overlay = self.digi_cfg_edit.station_overlay.clone();
            self.sstv.preview_dirty = true;
            self.sstv.composer_dirty = true;
        }
        // ...and the style, which the message overlay needs whether or not
        // there is a banner.
        if self.sstv.style != self.digi_cfg_edit.sstv_style {
            self.sstv.style = self.digi_cfg_edit.sstv_style;
            self.sstv.preview_dirty = true;
        }
        // The transmitted size: SSTV's line format fixes it, RIFP leaves it to
        // the operator. Changing it invalidates the composed preview.
        let dims = if rifp {
            self.digi_cfg_edit.rifp_size.dimensions()
        } else {
            self.sstv.tx_mode.dimensions()
        };
        if self.sstv.preview_dims != dims {
            self.sstv.preview_dims = dims;
            self.sstv.preview_dirty = true;
        }
        self.sstv.ensure_preview(dims, &ctx);
        crate::repaint::after_ms(&ctx, 120);

        let st = self.sstv.status.clone();
        let (signal, tx_active, progress) = if rifp {
            (self.sstv.rifp.signal, self.sstv.rifp.tx_active, self.sstv.rifp.tx_progress)
        } else {
            (st.signal, st.tx_active, st.progress)
        };

        // Whole-panel size. The mode/signal/slant controls sit in a boxed strip
        // on the left above LIVE + RECEIVED; the transmit compositor spans the
        // full height on the right, reclaiming the space the old full-width
        // control rows used to leave empty at the top.
        let avail = ui.available_size();
        let full_h = avail.y;
        let handle_w = 7.0;
        // TRANSMIT (send) column takes a user-draggable fraction of the width; the
        // receive side (LIVE + RECEIVED) gets the rest. Each keeps a usable minimum.
        // A phone takes the receive side and the compositor in turns: each has a
        // 300 pt floor of its own, so together they want twice a phone's width
        // before either has drawn a picture.
        let pane = self.phone_pane(ui, self.state.rx[0].mode);
        let (left_w, tx_w) = match pane {
            // Whichever one is up takes the row.
            Some(0) => (avail.x, 0.0),
            Some(_) => (0.0, avail.x),
            None if self.swl_mode() => (avail.x, 0.0),
            None => {
                // The three children (receive column, split handle, TX
                // column) sit in the panel row with item spacing between
                // them, so the split has to leave room for those two gaps —
                // otherwise the columns sum to `avail.x` but the row lands
                // `2 × spacing` past it and the whole panel grows over the
                // dock (issue #643).
                //
                // Each column *wants* 300 pt, but the column can be narrower
                // than that (a wide dock, a small window) and a fixed floor
                // then sums past `avail.x` and overruns the dock again — so the
                // floor is scaled to fit, in [`split_two`].
                let sp = ui.spacing().item_spacing.x;
                let (tx, left_w) = split_two(
                    avail.x - handle_w - 2.0 * sp,
                    self.view.sstv_tx_fraction,
                    300.0,
                    300.0,
                );
                (left_w, tx)
            }
        };
        // LIVE takes the rest of the receive side; the RECEIVED gallery width is a
        // user-draggable fraction of it (min one thumbnail column). The LIVE /
        // handle / RECEIVED row has the same two inter-item gaps as the panel
        // row above, so both terms leave room for them (issue #643).
        let sp = ui.spacing().item_spacing.x;
        let (gallery_w, live_w) =
            split_two(left_w - handle_w - 2.0 * sp, self.view.sstv_gallery_fraction, 150.0, 160.0);

        ui.horizontal_top(|ui| {
            // A received thumbnail was clicked → enlarge it (applied after the row).
            let mut enlarge: Option<usize> = None;
            // The gallery asked for the next page.
            let mut more = false;
            // A picture was deleted from the gallery's context menu. By name,
            // not by index: the answer comes back asynchronously and the list
            // may have grown a picture at the front by then.
            let mut delete: Option<String> = None;

            // ── LEFT: boxed controls, then LIVE + RECEIVED ──
            if pane.is_none_or(|p| p == 0) {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, full_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Frame::new()
                        .fill(crate::theme::ROW_BG())
                        .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                        .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 7 })
                        .show(ui, |ui| {
                            ui.set_min_width(left_w - 18.0);
                            ui.set_max_width(left_w - 18.0);
                            if rifp {
                                self.rifp_controls(ui, cmds);
                                return;
                            }

                            // Mode selection: Auto + the per-mode chips.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new("SSTV")
                                        .size(12.0)
                                        .strong()
                                        .color(crate::theme::CYAN()),
            );
                                self.digi_freq_chip(ui, cmds);
                                // Whether choosing SSTV moves the dial onto the
                                // band's published frequency. Beside the picker
                                // because the two answer the same question: where
                                // the decoder listens.
                                ui.add_enabled_ui(self.digi_cfg_seeded, |ui| {
                                    let keep = self.digi_cfg_edit.sstv_keep_dial;
                                    if crate::chrome::chip(ui, keep, "KEEP DIAL")
                                        .on_hover_text(if keep {
                                            "On: choosing SSTV leaves the dial where you tuned, \
                                             so the decoder works on any frequency. The published \
                                             SSTV frequencies are still in the ⇵ picker. Click to \
                                             go back to landing on the band's SSTV frequency."
                                        } else {
                                            "Off: choosing SSTV moves the dial onto the band's \
                                             SSTV frequency when you are more than 3 kHz from \
                                             one. Click to keep the dial where you tuned instead, \
                                             and decode SSTV on any frequency. Tuning after \
                                             choosing SSTV is never undone either way."
                                        })
                                        .clicked()
                                    {
                                        self.digi_cfg_edit.sstv_keep_dial = !keep;
                                        cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                    }
                                });
                                let auto_label = if self.sstv.auto {
                                    format!("Auto ({})", self.sstv.tx_mode.label())
                                } else {
                                    "Auto".to_string()
                                };
                                if crate::chrome::chip(ui, self.sstv.auto, &auto_label).clicked() {
                                    self.sstv.auto = true;
                                    self.sstv.tx_mode = SstvMode::Martin1;
                                    self.sstv.preview_dirty = true;
                                    cmds.push(Command::SstvSetMode(None));
                                }
                                for m in SstvMode::ALL {
                                    let active = !self.sstv.auto && self.sstv.tx_mode == m;
                                    if crate::chrome::chip(ui, active, m.label()).clicked() {
                                        self.sstv.auto = false;
                                        self.sstv.tx_mode = m;
                                        self.sstv.preview_dirty = true;
                                        cmds.push(Command::SstvSetMode(Some(m)));
                                    }
                                }
                                // **The pane's way out**, at the end of this
                                // row: the one drawn glyph, the same picture the
                                // float's bar uses to come back. This panel had no
                                // way to float at all before it.
                                if super::window_chip(ui) {
                                    self.float_operating_panel();
                                }
                            });
                            ui.add_space(5.0);

                                // Mode selection: Auto + the per-mode chips.
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(tr!("sstv-sstv"))
                                            .size(12.0)
                                            .strong()
                                            .color(crate::theme::CYAN()),
                                    );
                                    self.digi_freq_chip(ui, cmds);
                                    let auto_label = if self.sstv.auto {
                                        tr!("sstv-auto-mode", mode = self.sstv.tx_mode.label())
                                    } else {
                                        tr!("sstv-auto").to_string()
                                    };
                                    if crate::chrome::chip(ui, self.sstv.auto, &auto_label)
                                        .clicked()
                                    {
                                        self.sstv.auto = true;
                                        self.sstv.tx_mode = SstvMode::Martin1;
                                        self.sstv.preview_dirty = true;
                                        cmds.push(Command::SstvSetMode(None));
                                    }
                                    for m in SstvMode::ALL {
                                        let active = !self.sstv.auto && self.sstv.tx_mode == m;
                                        if crate::chrome::chip(ui, active, m.label()).clicked() {
                                            self.sstv.auto = false;
                                            self.sstv.tx_mode = m;
                                            self.sstv.preview_dirty = true;
                                            cmds.push(Command::SstvSetMode(Some(m)));
                                        }
                                    }
                                });
                                ui.add_space(5.0);

                                // Signal meter + activity, and the TX-slant trim.
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(tr!("sstv-signal")).size(10.0).weak());
                                    sstv_level_bar(ui, signal);
                                    if tx_active {
                                        ui.label(
                                            RichText::new(format!("● TX {:.0}%", progress * 100.0))
                                                .size(11.0)
                                                .strong()
                                                .color(crate::theme::ALERT()),
                                        );
                                    } else if st.rx_active {
                                        ui.label(
                                            RichText::new(format!(
                                                "● RX {:.0}%",
                                                st.progress * 100.0
                                            ))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::GREEN()),
                                        );
                                    } else if let Some(bad) = st.unsupported.as_deref() {
                                        // A header came through cleanly for a mode
                                        // this build cannot draw. Without this the
                                        // panel says "listening…" beside a textbook
                                        // signal and the receiver looks broken
                                        // (issue #421).
                                        ui.label(
                                            RichText::new(tr!(
                                                "sstv-not-decoded",
                                                what = bad.to_string()
                                            ))
                                            .size(10.5)
                                            .strong()
                                            .color(crate::theme::ALERT()),
                                    )
                                    .on_hover_text(
                                        "A station is sending in a mode sdroxide does not have. \
                                         The signal and the tuning are fine — there is simply no \
                                         decoder for this one. Nothing to fix at your end.",
                                    );
                                } else if let Some(m) = st.detected {
                                    ui.label(
                                        RichText::new(format!("last: {}", m.label()))
                                            .size(10.0)
                                            .weak(),
                                    );
                                } else {
                                    ui.label(RichText::new("listening…").size(10.0).weak());
                                }

                                // Issue #397. A receiver that has locked on is
                                // committed for the whole length of the mode it
                                // locked on to, and Scottie DX is four and a
                                // half minutes — so a VIS misread as a slow
                                // mode costs every picture sent while it runs
                                // out. On QO-100, where one station follows
                                // another over the same transponder, that is
                                // the next few overs.
                                //
                                // Offered whether or not a picture is under
                                // way: re-arming an idle hunt costs nothing,
                                // and a chip that appears only once the mistake
                                // has been made is one the operator has to find
                                // in a hurry. The half-picture goes with it,
                                // here as well as in the decoder — leaving the
                                // abandoned frame on screen would say the
                                // button had not worked.
                                // Who sent it. The FSK ID arrives in tones a
                                // fraction of a second after the picture, which
                                // is exactly when the operator is looking at the
                                // frame and wondering whose it is — and unlike a
                                // banner drawn into the image, this is the
                                // station's own machine-readable identification.
                                if let Some(id) = st.rx_id.as_deref() {
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new(format!("ID {id}"))
                                            .size(11.0)
                                            .strong()
                                            .color(crate::theme::CYAN_DIM()),
                                    )
                                    .on_hover_text(
                                        "The callsign the last station sent as an FSK ID after \
                                         its picture.",
                                    );
                                }

                                if !self.swl_mode() {
                                    // Issue #643. Everything below sits inside
                                    // `add_enabled_ui`, a scope, and a scope in
                                    // a wrapping row takes its rect from the
                                    // space left at the cursor. When the
                                    // signal row has run to the right edge
                                    // that space is zero-width, and inside the
                                    // scope the wrap guard
                                    // (`max_rect.left() < cursor.left()`) can
                                    // never fire — both are the same point —
                                    // so the slider expands the scope to the
                                    // right instead of wrapping, growing the
                                    // whole left column and pushing the panel
                                    // over the band dock. Start a fresh line
                                    // while what is left cannot hold the
                                    // prefix and the slider.
                                    if ui.available_size_before_wrap().x < TX_TRIM_MIN_W {
                                        ui.end_row();
                                    }
                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.label(RichText::new("TX slant").size(10.0).weak()).on_hover_text(
                                        "Transmit clock trim (ppm) to remove slant on the far-end decoder",
                                    );
                                    ui.add_enabled_ui(self.digi_cfg_seeded, |ui| {
                                        ui.spacing_mut().slider_width = 130.0;
                                        let resp = crate::chrome::slider(ui, egui::Slider::new(
                                                &mut self.digi_cfg_edit.sstv_tx_ppm,
                                                -5000.0..=5000.0,
                                            )
                                            .suffix(" ppm")
                                            .fixed_decimals(0),
                                        );
                                        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        if ui
                                            .small_button("0")
                                            .on_hover_text("Reset to 0 ppm")
                                            .clicked()
                                        {
                                            self.digi_cfg_edit.sstv_tx_ppm = 0.0;
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        ui.separator();
                                        // The callsign in tones after the picture.
                                        // Beside the slant trim rather than in the
                                        // banner window: the banner identifies the
                                        // station to a person looking at the
                                        // picture, this identifies it to the
                                        // repeater decoding it, and the two are set
                                        // for different reasons.
                                        if crate::chrome::checkbox(
                                            ui,
                                            &mut self.digi_cfg_edit.sstv_fsk_id,
                                            "FSK ID",
                                        )
                                        .on_hover_text(
                                            "Send your callsign in tones after each picture — the \
                                             identification SSTV repeaters and other programs read. \
                                             Adds about 2.5 seconds, and sends nothing at all until \
                                             you have set a callsign.",
                                        )
                                        .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                        ui.separator();
                                        // Dead air before the calibration header, so
                                        // the rig is really on the air by the time
                                        // the VIS code goes out. Here rather than in
                                        // the setup window because it is the same
                                        // kind of per-station trim as the slant
                                        // beside it, and the operator who needs it
                                        // finds out by transmitting.
                                        ui.label(RichText::new("TX lead").size(10.0).weak());
                                        if ui
                                            .add(
                                                egui::DragValue::new(
                                                    &mut self.digi_cfg_edit.sstv_txdelay_ms,
                                                )
                                                .range(0..=3000)
                                                .speed(10.0)
                                                .suffix(" ms"),
                                            )
                                            .on_hover_text(
                                                "Silence sent after keying and before the picture's \
                                                 leader and VIS code. A decoder that misses any of \
                                                 that header shows no picture at all, so this covers \
                                                 the gap between asking a rig for PTT and it really \
                                                 being on the air. 0 for an SDR that keys instantly.",
                                            )
                                            .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                        }
                                    });
                                }
                            });
                            ui.add_space(5.0);
                            // Restart/Clear sit on their own line inside the
                            // box: in the signal row they grew that row's
                            // minimum width past the column and pushed the
                            // whole panel over the dock (#643). One row for the
                            // pair keeps them together and wraps.
                            ui.horizontal_wrapped(|ui| {
                                if crate::chrome::chip(ui, false, "Restart RX")
                                    .on_hover_text(
                                        "Abandon the picture being received and listen for the                                          next header. For a transmission that started decoding                                          in the wrong mode — the receiver is otherwise committed                                          until that mode runs out.",
                                    )
                                    .clicked()
                                {
                                    cmds.push(Command::SstvRestartRx);
                                    self.sstv.rx_color = None;
                                    self.sstv.rx_tex = None;
                                }

                                // Clear the picture currently shown in LIVE without
                                // touching the decoder: a completed frame that is
                                // still on screen is wiped so the next one can be
                                // seen arriving, and an in-progress decode keeps
                                // running (use Restart RX to abandon it).
                                if crate::chrome::chip(ui, false, "Clear Rx")
                                    .on_hover_text(
                                        "Clear the picture currently shown in the LIVE view. \
                                         The decoder keeps running — use Restart RX to abandon \
                                         an in-progress picture and listen for the next header.",
                                    )
                                    .clicked()
                                {
                                    self.sstv.rx_color = None;
                                    self.sstv.rx_tex = None;
                                }

                            });
                            // **Receive sensitivity, on a line of its own.**
                            // It is a label, a slider and a chip — broad enough
                            // that sharing the Restart/Clear row pushed that
                            // row, and with it the panel, over the dock at 960
                            // points: the #643 guard's exact report, *"Sstv at
                            // 960 pt: ink x 0..704 crosses the dock edge at
                            // 681"*. Its own wrapped row and a capped slider is
                            // what keeps it inside the column at every width.
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Sensitivity").size(10.0).weak());
                                {
                                    let mut v = self.digi_cfg_edit.sstv_rx_sensitivity;
                                    ui.spacing_mut().slider_width = 90.0;
                                    let resp = crate::chrome::slider(
                                        ui,
                                        egui::Slider::new(&mut v, -5..=5).fixed_decimals(0),
                                    )
                                    .on_hover_text(
                                        "Adjusts the signal level fed to the SSTV decoder. \
                                         0 = current automatic level (Auto). Negative values \
                                         attenuate, positive values boost. Click Auto to return \
                                         to the automatic level.",
                                    );
                                    if resp.changed() {
                                        self.digi_cfg_edit.sstv_rx_sensitivity = v;
                                        self.digi_cfg_edit.sstv_rx_auto = v == 0;
                                        cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                    }
                                }
                                if crate::chrome::chip(
                                    ui,
                                    self.digi_cfg_edit.sstv_rx_auto,
                                    "Auto",
                                )
                                .on_hover_text(
                                    "Returns to the automatic level (reference 0) and \
                                     disables manual adjustment. Highlighted while active.",
                                )
                                .clicked()
                                {
                                    self.digi_cfg_edit.sstv_rx_auto = true;
                                    self.digi_cfg_edit.sstv_rx_sensitivity = 0;
                                    cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
                                }

                            });
                        });
                    ui.add_space(6.0);

                    // LIVE + RECEIVED fill the remaining height of the left column.
                    //
                    // Stacked on a phone rather than side by side: halving 377
                    // points leaves two columns too narrow to show a picture in,
                    // and a received picture is the whole point of the mode.
                    let stack = pane.is_some();
                    let row_h = ui.available_height().max(160.0);
                    let (row_h, live_w, gallery_w) = if stack {
                        let w = ui.available_width();
                        ((row_h / 2.0).max(120.0), w, w)
                    } else {
                        (row_h, live_w, gallery_w)
                    };
                    let lay = if stack {
                        egui::Layout::top_down(egui::Align::Min)
                    } else {
                        egui::Layout::left_to_right(egui::Align::Min)
                    };
                    ui.with_layout(lay, |ui| {
                        // LIVE: the picture currently decoding, shown large.
                        sstv_section(ui, "LIVE", egui::vec2(live_w, row_h), |ui| {
                            ui.centered_and_justified(|ui| {
                                if let Some(tex) = &self.sstv.rx_tex {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .max_height(row_h - 34.0)
                                            .max_width(live_w - 16.0),
                                    );
                                } else {
                                    let msg = if rifp {
                                        // RIFP only paints live from the raw
                                        // raster; anything else appears whole.
                                        "waiting for a picture…"
                                    } else if signal > 0.0008 {
                                        // There *is* signal here — the meter is
                                        // above the same threshold that says so
                                        // two lines below, so this used to read
                                        // "waiting for a signal…" exactly when
                                        // one was present. That is the state an
                                        // operator is actually in when a
                                        // transmission is up and no picture has
                                        // appeared, and the old wording denied it
                                        // was the case: no audio at all, wrong
                                        // level, or a signal we cannot read all
                                        // look identical from in here.
                                        "audio present · no SSTV header yet"
                                    } else {
                                        "no / low audio"
                                    };
                                    ui.label(RichText::new(msg).size(11.0).weak());
                                }
                            });
                        });
                        // Draggable vertical divider between LIVE and RECEIVED.
                        if !stack {
                            let hresp =
                                crate::chrome::split_handle(ui, egui::vec2(handle_w, row_h), None);
                            if hresp.dragged() {
                                // Dragging right shrinks the gallery (grows LIVE).
                                let d = hresp.drag_delta().x / left_w.max(1.0);
                                self.view.sstv_gallery_fraction =
                                    (self.view.sstv_gallery_fraction - d).clamp(0.2, 0.6);
                            }
                        }

                        // RECEIVED: narrow multi-column gallery of decoded
                        // pictures, thumbnailed by the engine out of its own
                        // store — so it is the same gallery here, on a browser
                        // tab, and on a client dialled in from anywhere else.
                        sstv_section(ui, "RECEIVED", egui::vec2(gallery_w, row_h), |ui| {
                            if self.sstv.received.is_empty() {
                                let msg = if self.sstv.page_pending {
                                    "Reading the radio's pictures…".to_string()
                                } else if self.sstv.dir.is_empty() {
                                    "Decoded pictures collect here.".to_string()
                                } else {
                                    format!(
                                        "Decoded pictures are saved {} and collect here.",
                                        self.store_where(&self.sstv.dir),
                                    )
                                };
                                ui.label(RichText::new(msg).size(11.0).weak());
                                return;
                            }
                            let thumb = egui::vec2(112.0, 90.0);
                            let shown = self.sstv.received.len() as u32;
                            let older = self.sstv.total.saturating_sub(shown);
                            egui::ScrollArea::vertical()
                                .id_salt("sstv-gallery")
                                .max_height(row_h - 24.0)
                                .auto_shrink([false, false])
                                .show_themed(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                                        for (i, r) in self.sstv.received.iter().enumerate() {
                                            let resp = ui
                                                .add(
                                                    egui::Image::new(&r.thumb)
                                                        .fit_to_exact_size(thumb)
                                                        .corner_radius(2.0)
                                                        .sense(egui::Sense::click()),
                                                )
                                                .on_hover_text(
                                                    "Click to enlarge · right-click to delete",
                                                );
                                            if resp.clicked() {
                                                enlarge = Some(i);
                                            }
                                            // A menu rather than an ✕ on the
                                            // thumbnail: a picture is 112 points
                                            // across in a wrapped grid, and a
                                            // delete target that close to the
                                            // one for opening it would be pressed
                                            // by accident.
                                            resp.context_menu(|ui| {
                                                ui.label(
                                                    RichText::new(shorten(&r.entry.name, 32))
                                                        .size(10.0)
                                                        .weak(),
                                                );
                                                if ui.button("Delete this picture").clicked() {
                                                    delete = Some(r.entry.name.clone());
                                                    ui.close();
                                                }
                                            });
                                        }
                                    });
                                    // The rest of the store is a request away.
                                    // Explicit rather than automatic on scroll:
                                    // a page is a few hundred kilobytes and the
                                    // operator may be on a phone link.
                                    if older > 0 && self.sstv.received.len() >= GALLERY_MAX {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(tr!("sstv-last", mode = m.label()))
                                                .size(10.0)
                                                .weak(),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new(tr!("sstv-listening")).size(10.0).weak(),
                                        );
                                    }

                                    // Issue #397. A receiver that has locked on is
                                    // committed for the whole length of the mode it
                                    // locked on to, and Scottie DX is four and a
                                    // half minutes — so a VIS misread as a slow
                                    // mode costs every picture sent while it runs
                                    // out. On QO-100, where one station follows
                                    // another over the same transponder, that is
                                    // the next few overs.
                                    //
                                    // Offered whether or not a picture is under
                                    // way: re-arming an idle hunt costs nothing,
                                    // and a chip that appears only once the mistake
                                    // has been made is one the operator has to find
                                    // in a hurry. The half-picture goes with it,
                                    // here as well as in the decoder — leaving the
                                    // abandoned frame on screen would say the
                                    // button had not worked.
                                    if crate::chrome::chip(ui, false, tr!("sstv-restart-rx"))
                                        .on_hover_text(tr!(
                                            "sstv-abandon-picture-being-received-listen"
                                        ))
                                        .clicked()
                                    {
                                        cmds.push(Command::SstvRestartRx);
                                        self.sstv.rx_color = None;
                                        self.sstv.rx_tex = None;
                                    }

                                    // Who sent it. The FSK ID arrives in tones a
                                    // fraction of a second after the picture, which
                                    // is exactly when the operator is looking at the
                                    // frame and wondering whose it is — and unlike a
                                    // banner drawn into the image, this is the
                                    // station's own machine-readable identification.
                                    if let Some(id) = st.rx_id.as_deref() {
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(format!("ID {id}"))
                                                .size(11.0)
                                                .strong()
                                                .color(crate::theme::CYAN_DIM()),
                                        )
                                        .on_hover_text(tr!("sstv-callsign-last-station-sent-fsk"));
                                    }

                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.label(RichText::new(tr!("sstv-tx-slant")).size(10.0).weak())
                                        .on_hover_text(tr!("sstv-transmit-clock-trim-ppm-remove"));
                                    ui.add_enabled_ui(self.digi_cfg_seeded, |ui| {
                                        ui.spacing_mut().slider_width = 130.0;
                                        let resp = crate::chrome::slider(
                                            ui,
                                            egui::Slider::new(
                                                &mut self.digi_cfg_edit.sstv_tx_ppm,
                                                -5000.0..=5000.0,
                                            )
                                            .suffix(" ppm")
                                            .fixed_decimals(0),
                                        );
                                        if resp.drag_stopped()
                                            || (resp.changed() && !resp.dragged())
                                        {
                                            cmds.push(Command::SetDigiConfig(
                                                self.digi_cfg_edit.clone(),
                                            ));
                                        }
                                        if ui
                                            .small_button("0")
                                            .on_hover_text(tr!("sstv-reset-0-ppm"))
                                            .clicked()
                                        {
                                            self.digi_cfg_edit.sstv_tx_ppm = 0.0;
                                            cmds.push(Command::SetDigiConfig(
                                                self.digi_cfg_edit.clone(),
                                            ));
                                        }
                                        ui.separator();
                                        // The callsign in tones after the picture.
                                        // Beside the slant trim rather than in the
                                        // banner window: the banner identifies the
                                        // station to a person looking at the
                                        // picture, this identifies it to the
                                        // repeater decoding it, and the two are set
                                        // for different reasons.
                                        if crate::chrome::checkbox(
                                            ui,
                                            &mut self.digi_cfg_edit.sstv_fsk_id,
                                            tr!("sstv-fsk-id"),
                                        )
                                        .on_hover_text(tr!("sstv-send-your-callsign-tones-after"))
                                        .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(
                                                self.digi_cfg_edit.clone(),
                                            ));
                                        }
                                        ui.separator();
                                        // Dead air before the calibration header, so
                                        // the rig is really on the air by the time
                                        // the VIS code goes out. Here rather than in
                                        // the setup window because it is the same
                                        // kind of per-station trim as the slant
                                        // beside it, and the operator who needs it
                                        // finds out by transmitting.
                                        ui.label(
                                            RichText::new(tr!("sstv-tx-lead")).size(10.0).weak(),
                                        );
                                        if ui
                                            .add(
                                                egui::DragValue::new(
                                                    &mut self.digi_cfg_edit.sstv_txdelay_ms,
                                                )
                                                .range(0..=3000)
                                                .speed(10.0)
                                                .suffix(" ms"),
                                            )
                                            .on_hover_text(tr!(
                                                "sstv-silence-sent-after-keying-before"
                                            ))
                                            .changed()
                                        {
                                            cmds.push(Command::SetDigiConfig(
                                                self.digi_cfg_edit.clone(),
                                            ));
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);

                        // LIVE + RECEIVED fill the remaining height of the left column.
                        //
                        // Stacked on a phone rather than side by side: halving 377
                        // points leaves two columns too narrow to show a picture in,
                        // and a received picture is the whole point of the mode.
                        let stack = pane.is_some();
                        let row_h = ui.available_height().max(160.0);
                        let (row_h, live_w, gallery_w) = if stack {
                            let w = ui.available_width();
                            ((row_h / 2.0).max(120.0), w, w)
                        } else {
                            (row_h, live_w, gallery_w)
                        };
                        let lay = if stack {
                            egui::Layout::top_down(egui::Align::Min)
                        } else {
                            egui::Layout::left_to_right(egui::Align::Min)
                        };
                        ui.with_layout(lay, |ui| {
                            // LIVE: the picture currently decoding, shown large.
                            sstv_section(ui, &tr!("sstv-live"), egui::vec2(live_w, row_h), |ui| {
                                ui.centered_and_justified(|ui| {
                                    if let Some(tex) = &self.sstv.rx_tex {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .max_height(row_h - 34.0)
                                                .max_width(live_w - 16.0),
                                        );
                                    } else {
                                        let msg = if rifp {
                                            // RIFP only paints live from the raw
                                            // raster; anything else appears whole.
                                            tr!("sstv-waiting-picture")
                                        } else if signal > 0.0008 {
                                            tr!("sstv-waiting-signal")
                                        } else {
                                            tr!("sstv-no-low-audio")
                                        };
                                        ui.label(RichText::new(msg).size(11.0).weak());
                                    }
                                });
                            });
                            // Draggable vertical divider between LIVE and RECEIVED.
                            if !stack {
                                let hresp = crate::chrome::split_handle(
                                    ui,
                                    egui::vec2(handle_w, row_h),
                                    None,
                                );
                                if hresp.dragged() {
                                    // Dragging right shrinks the gallery (grows LIVE).
                                    let d = hresp.drag_delta().x / left_w.max(1.0);
                                    self.view.sstv_gallery_fraction =
                                        (self.view.sstv_gallery_fraction - d).clamp(0.2, 0.6);
                                }
                            }

                            // RECEIVED: narrow multi-column gallery of decoded
                            // pictures, thumbnailed by the engine out of its own
                            // store — so it is the same gallery here, on a browser
                            // tab, and on a client dialled in from anywhere else.
                            sstv_section(
                                ui,
                                &tr!("sstv-received"),
                                egui::vec2(gallery_w, row_h),
                                |ui| {
                                    if self.sstv.received.is_empty() {
                                        let msg = if self.sstv.page_pending {
                                            tr!("sstv-reading-radio-s-pictures").to_string()
                                        } else if self.sstv.dir.is_empty() {
                                            tr!("sstv-decoded-pictures-collect-here").to_string()
                                        } else {
                                            tr!(
                                                "sstv-saved-where",
                                                place = self.store_where(&self.sstv.dir)
                                            )
                                        };
                                        ui.label(RichText::new(msg).size(11.0).weak());
                                        return;
                                    }
                                    let thumb = egui::vec2(112.0, 90.0);
                                    let shown = self.sstv.received.len() as u32;
                                    let older = self.sstv.total.saturating_sub(shown);
                                    egui::ScrollArea::vertical()
                                        .id_salt("sstv-gallery")
                                        .max_height(row_h - 24.0)
                                        .auto_shrink([false, false])
                                        .show_themed(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(5.0, 5.0);
                                                for (i, r) in self.sstv.received.iter().enumerate()
                                                {
                                                    let resp = ui
                                                        .add(
                                                            egui::Image::new(&r.thumb)
                                                                .fit_to_exact_size(thumb)
                                                                .corner_radius(2.0)
                                                                .sense(egui::Sense::click()),
                                                        )
                                                        .on_hover_text(tr!(
                                                            "sstv-click-enlarge-right-click-delete"
                                                        ));
                                                    if resp.clicked() {
                                                        enlarge = Some(i);
                                                    }
                                                    // A menu rather than an ✕ on the
                                                    // thumbnail: a picture is 112 points
                                                    // across in a wrapped grid, and a
                                                    // delete target that close to the
                                                    // one for opening it would be pressed
                                                    // by accident.
                                                    resp.context_menu(|ui| {
                                                        ui.label(
                                                            RichText::new(shorten(
                                                                &r.entry.name,
                                                                32,
                                                            ))
                                                            .size(10.0)
                                                            .weak(),
                                                        );
                                                        if ui
                                                            .button(tr!("sstv-delete-picture"))
                                                            .clicked()
                                                        {
                                                            delete = Some(r.entry.name.clone());
                                                            ui.close();
                                                        }
                                                    });
                                                }
                                            });
                                            // The rest of the store is a request away.
                                            // Explicit rather than automatic on scroll:
                                            // a page is a few hundred kilobytes and the
                                            // operator may be on a phone link.
                                            if older > 0 && self.sstv.received.len() >= GALLERY_MAX
                                            {
                                                ui.add_space(4.0);
                                                ui.label(
                                                    RichText::new(tr!(
                                                        "sstv-older-in-store",
                                                        n = older
                                                    ))
                                                    .size(9.5)
                                                    .weak(),
                                                );
                                            } else if older > 0 {
                                                ui.add_space(4.0);
                                                let label = if self.sstv.page_pending {
                                                    tr!("sstv-loading").to_string()
                                                } else {
                                                    tr!("sstv-older-load-more", n = older)
                                                };
                                                if crate::chrome::chip(ui, false, label).clicked()
                                                    && !self.sstv.page_pending
                                                {
                                                    more = true;
                                                }
                                            }
                                        });
                                },
                            );
                        });
                    },
                );
            }

            // Draggable vertical divider between the receive side and the
            // TRANSMIT (send) column — mirrors the FT8 decode/QSO splitter.
            if !self.swl_mode() && pane.is_none() {
                let hresp = crate::chrome::split_handle(ui, egui::vec2(handle_w, full_h), None);
                if hresp.dragged() {
                    // Dragging right shrinks the TX column (grows the receive side).
                    let d = hresp.drag_delta().x / avail.x.max(1.0);
                    self.view.sstv_tx_fraction = (self.view.sstv_tx_fraction - d).clamp(0.22, 0.6);
                }
            }

            // ── RIGHT: transmit compositor, full height ──
            if !self.swl_mode() && pane.is_none_or(|p| p != 0) {
            ui.allocate_ui(egui::vec2(tx_w, full_h), |ui| {
                sstv_section(ui, "TRANSMIT", egui::vec2(tx_w, full_h), |ui| {
                    // The compositor is a fixed stack — the five slots, the
                    // load buttons, the preview, the message box and the
                    // transmit controls — and none of it can be dropped
                    // without dropping a control. On a screen with less height
                    // than the stack needs it used to run off the bottom edge
                    // and simply not be there (issue #231). Scrolled, the whole
                    // stack stays reachable, and where there is room the bar
                    // never appears.
                    egui::ScrollArea::vertical()
                        .id_salt("sstv-transmit")
                        .auto_shrink([false, false])
                        .show_themed(ui, |ui| {
                        let inner_w = tx_w - 18.0;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            // Five fixed 70 pt thumbs are 370 pt with their
                            // gaps — wider than a 300 pt TX column ever is,
                            // so at a narrow window the row used to drag the
                            // whole panel over the dock (issue #643). Shrink
                            // to fit the width the row actually has; at any
                            // width where 70 pt fits, this is 70 pt.
                            let n = IMAGE_SLOTS as f32;
                            let sw = ((ui.available_width() - 5.0 * (n - 1.0)) / n).clamp(40.0, 70.0);
                            let size = egui::vec2(sw, sw * 54.0 / 70.0);
                            for i in 0..IMAGE_SLOTS {
                                let sel = self.sstv.selected_slot == i;
                                let resp = if let Some(tex) =
                                    self.sstv.slot_thumbs.get(i).and_then(|t| t.as_ref())
                                {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .fit_to_exact_size(size)
                                            .corner_radius(2.0)
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text(tr!("sstv-what-printed-across-top-every"))
                                    .clicked()
                                    {
                                        self.sstv.banner_open = !self.sstv.banner_open;
                                    }
                                });
                                if let Some(err) = &self.sstv.pick_error {
                                    ui.label(
                                        RichText::new(err).size(10.0).color(crate::theme::YELLOW()),
                                    );
                                }
                                ui.add_space(6.0);

                                // Preview gets a capped share of the height; the message box
                                // grows to fill whatever's left above the buttons.
                                let btn_h = 42.0;
                                let gap = 6.0;
                                let preview_h = (ui.available_height() * 0.45).clamp(80.0, 260.0);
                                egui::Frame::new()
                                    .fill(crate::theme::gray(6))
                                    .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                                    .inner_margin(2.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(egui::vec2(inner_w, preview_h));
                                        ui.set_max_size(egui::vec2(inner_w, preview_h));
                                        ui.centered_and_justified(|ui| {
                                            if let Some(tex) = &self.sstv.preview_tex {
                                                ui.add(
                                                    egui::Image::new(tex)
                                                        .max_height(preview_h - 4.0)
                                                        .max_width(inner_w - 4.0),
                                                );
                                            } else {
                                                let waiting = self
                                                    .sstv
                                                    .presets
                                                    .slot(self.sstv.selected_slot)
                                                    .has_picture();
                                                ui.label(
                                                    RichText::new(if waiting {
                                                        tr!("sstv-fetching-slot-s-picture")
                                                    } else {
                                                        tr!("sstv-load-image-into-slot")
                                                    })
                                                    .size(11.0)
                                                    .weak(),
                                                );
                                            }
                                        });
                                    });
                                ui.add_space(gap);

                                // Overlay message for the active slot — fills the height
                                // above the buttons. The engine owns the text; this box
                                // claims the slot while the cursor is in it and hands it
                                // back when focus leaves, so an edit made on another screen
                                // still reaches every slot but this one.
                                let sel = self.sstv.selected_slot;
                                let msg_h = (ui.available_height() - btn_h - gap).max(48.0);
                                let mut buf = self.sstv.current_message().to_string();
                                let resp = ui
                                    .push_id(sel, |ui| {
                                        crate::chrome::field_sized(
                                            ui,
                                            egui::vec2(inner_w, msg_h),
                                            egui::TextEdit::multiline(&mut buf)
                                                .hint_text(tr!("sstv-drawn-slot-s-image")),
                                        )
                                    })
                                    .inner;
                                if resp.changed() {
                                    self.sstv.msg_edit = Some((sel, buf));
                                    self.sstv.preview_dirty = true;
                                } else if resp.gained_focus() {
                                    cmds.extend(crate::sstv::claim_message(
                                        &mut self.sstv.msg_edit,
                                        &self.sstv.presets,
                                        sel,
                                    ));
                                    self.sstv.selected_slot = i;
                                    self.sstv.preview_dirty = true;
                                    self.sstv.composer_dirty = true;
                                }
                                if resp.lost_focus() {
                                    cmds.extend(crate::sstv::commit_message(
                                        &mut self.sstv.msg_edit,
                                        &self.sstv.presets,
                                    ));
                                }
                                ui.add_space(gap);

                        // Load/replace/clear the active slot's picture.
                        ui.horizontal(|ui| {
                            let sel = self.sstv.selected_slot;
                            let has_img = self.sstv.presets.slot(sel).has_picture();
                            let label = if has_img { "Change image…" } else { "Load image…" };
                            if crate::chrome::chip(ui, false, label).clicked() {
                                self.sstv.pick_target = Some(sel);
                                pick_image(self.sstv.inbox.clone());
                            }
                            if has_img
                                && crate::chrome::chip(ui, false, "Clear")
                                    .on_hover_text("Empty this slot's picture (the message stays)")
                                    .clicked()
                            {
                                cmds.push(Command::ImageClearSlot(sel as u8));
                            }
                            // The banner belongs to the station, not to the slot:
                            // one editor, reached from whichever slot is in front.
                            if crate::chrome::chip(ui, self.sstv.banner_open, "Banner…")
                                .on_hover_text(
                                    "What is printed across the top of every picture this station \
                                     sends",
                                )
                                .clicked()
                            {
                                self.sstv.banner_open = !self.sstv.banner_open;
                            }
                            // The composer is the fuller answer to the same
                            // need: text you can put anywhere, not just a strip
                            // at an edge.
                            if crate::chrome::chip(ui, self.sstv.composer_open, "Editor…")
                                .on_hover_text(
                                    "Compose the picture itself: movable text, any face, a strip \
                                     across or down the frame",
                                )
                                .clicked()
                            {
                                self.sstv.composer_open = !self.sstv.composer_open;
                            }
                        });
                        if let Some(err) = &self.sstv.pick_error {
                            ui.label(RichText::new(err).size(10.0).color(crate::theme::YELLOW()));
                        }
                        ui.add_space(6.0);

                        // Preview gets a capped share of the height; the message box
                        // grows to fill whatever's left above the buttons.
                        let btn_h = 42.0;
                        let gap = 6.0;
                        let preview_h = (ui.available_height() * 0.45).clamp(80.0, 260.0);
                        egui::Frame::new()
                            .fill(crate::theme::gray(6))
                            .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                            .inner_margin(2.0)
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(inner_w - 6.0, preview_h));
                                ui.set_max_size(egui::vec2(inner_w - 6.0, preview_h));
                                ui.centered_and_justified(|ui| {
                                    if let Some(tex) = &self.sstv.preview_tex {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .max_height(preview_h - 4.0)
                                                .max_width(inner_w - 4.0),
                                        );
                                    } else {
                                        let waiting =
                                            self.sstv.presets.slot(self.sstv.selected_slot).has_picture();
                                        ui.label(
                                            RichText::new(if waiting {
                                                "Fetching this slot's picture…"
                                            } else {
                                                Command::SstvTx { mode: self.sstv.tx_mode, png }
                                            });
                                        }
                                    }
                                    ui.add_space(8.0);
                                    let abort = ui
                                        .add_enabled_ui(tx_active, |ui| {
                                            crate::chrome::chip(
                                                ui,
                                                false,
                                                RichText::new(tr!("sstv-abort-tx"))
                                                    .size(15.0)
                                                    .strong(),
                                            )
                                        })
                                        .inner;
                                    if abort.clicked() {
                                        cmds.push(Command::DigiAbortTx);
                                    }
                                });
                            });
                    });
                });
            }

            if let Some(i) = enlarge {
                self.sstv.enlarged = Some(i);
            }
            if more {
                self.sstv.page_pending = true;
                cmds.push(Command::ImageList {
                    kind: ImageKind::Sstv,
                    offset: self.sstv.received.len() as u32,
                    count: sdroxide_types::IMAGE_PAGE_MAX,
                });
            }
            // The gallery entry goes when the engine says the file has, not
            // here: the store is on the radio's machine, and a thumbnail that
            // vanished from a delete that then failed would be a lie.
            if let Some(name) = delete {
                cmds.push(Command::ImageDelete { kind: ImageKind::Sstv, name });
            }
        });

        self.banner_window(&ctx, cmds);
        self.composer_window(&ctx, cmds, mode);
        // **A pause settles the document to the engine.** A drag is a hundred
        // edits a second and the engine writes what it is given to disk, so the
        // write waits for the hand to stop; a drag's own release settles it
        // sooner, and this is the backstop for an edit that ended some other way
        // (typing, a pointer that left the window).
        // `Instant` is not available on wasm32 — `now()` panics there and the
        // browser editor froze (#262/#297). The wall clock in `crate::time`
        // works on both targets; a 700 ms debounce needs no monotonicity.
        const SETTLE_SECS: f64 = 0.7;
        if self.sstv.composer_open
            && self
                .sstv
                .composer_since
                .is_some_and(|t| crate::time::now_unix_f64() - t >= SETTLE_SECS)
        {
            self.composer_flush(cmds);
        }
        self.qsl_window(&ctx);

        // Enlarged view of a clicked received image (overlay window).
        if let Some(idx) = self.sstv.enlarged {
            let mut open = true;
            let mut save = false;
            let mut reupload = false;
            let mut email_qsl = false;
            let mut pressed_delete = false;
            if let Some(r) = self.sstv.received.get(idx) {
                // The full-size picture lives on the radio, and its *bytes* are
                // what "Save picture…" writes out. Ask for it the moment one is
                // opened — but ask while the bytes are missing, not merely
                // while the texture is, because a picture can be shown full
                // size from a texture the panel already held while its PNG
                // bytes have moved on (a newer fetch, or a promotion from the
                // just-received picture). Gating on the texture left the SAVE
                // chip missing until the operator closed and reopened the
                // session — the reported bug. Asked once per name, so a fetch
                // that fails is not a request every frame.
                let have_bytes =
                    self.sstv.full_png.as_ref().is_some_and(|(n, _)| *n == r.entry.name);
                if !have_bytes && self.sstv.full_asked.as_deref() != Some(&r.entry.name) {
                    self.sstv.full_asked = Some(r.entry.name.clone());
                    self.sstv.full_gone = false;
                    cmds.push(Command::ImageGet {
                        kind: ImageKind::Sstv,
                        name: r.entry.name.clone(),
                    });
                }
                // A picture can be stored before the first listing has come
                // back, so the directory is not always known to name.
                let del_hint = if self.sstv.dir.is_empty() {
                    tr!("sstv-delete-picture-store").to_string()
                } else {
                    tr!("sstv-delete-picture-from", place = self.store_where(&self.sstv.dir))
                };
                let r = &self.sstv.received[idx];
                let savable = self.sstv.full_png.as_ref().is_some_and(|(n, _)| *n == r.entry.name);
                let armed = self.sstv.confirm_delete.as_deref() == Some(&r.entry.name);
                egui::Window::new(tr!("sstv-received-image"))
                    .id(crate::layout::salted_id(&ctx, "Received image"))
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .default_size([
                        crate::layout::window_w(&ctx, 660.0),
                        crate::layout::window_h(&ctx, 528.0),
                    ])
                    .frame(crate::chrome::window_frame())
                    .show(&ctx, |ui| {
                        crate::chrome::window_body_bg(ui);
                        // Scale up to fill the window width (preserving aspect).
                        // A thumbnail stands in until the real one arrives, at
                        // the size the real one will be, so nothing jumps.
                        let tex = r.full.as_ref().unwrap_or(&r.thumb);
                        let native =
                            egui::vec2(f32::from(r.entry.width), f32::from(r.entry.height));
                        let native = if native.x > 0.0 { native } else { tex.size_vec2() };
                        let avail_w = ui.available_width().min(1000.0);
                        let scale = (avail_w / native.x.max(1.0)).clamp(1.0, 4.0);
                        ui.add(egui::Image::new(tex).fit_to_exact_size(native * scale));
                        // Wrapping, not a fixed row: at 360 pt the four chips
                        // and the label beside them need ~368 pt of a 344 pt
                        // window, so the row already overflowed before any of
                        // these chips existed — it was the one row on this
                        // window that could not fit, and nothing wrapped it
                        // because `horizontal` does not wrap. Measured rather
                        // than guessed: `Save picture…` 95.7, `Save image as…`
                        // 105.2, `Re-upload` 70.9, `Delete…` 63.8, plus 8 pt
                        // between each. The chips below are sized to keep any
                        // *one* of them inside the narrowest window, so the
                        // row breaking to a second line is the failure mode
                        // rather than content pushed off the edge.
                        ui.horizontal_wrapped(|ui| {
                            if self.sstv.full_gone {
                                ui.label(
                                    RichText::new(tr!("sstv-no-longer-store"))
                                        .size(10.0)
                                        .color(crate::theme::YELLOW()),
                                );
                            } else if r.full.is_none() {
                                ui.label(
                                    RichText::new(tr!("sstv-loading-full-size")).size(10.0).weak(),
                                );
                            } else if savable
                                && crate::chrome::chip(ui, false, "Save image as…")
                                    // Says what it does: it asks where. "Save
                                    // picture…" read as "it has saved one",
                                    // which is exactly how a picture's worth of
                                    // clicks go somewhere unexpected.
                                    .on_hover_text(
                                        "Save a copy on this computer — it asks which file",
                                    )
                                    .clicked()
                            {
                                save = true;
                            }
                            // **Sharing the picture, not a QSL.** SSTV has no
                            // QSL audience — the stations on it are amateurs and
                            // 11 m operators, reached through QRZ and the DX
                            // communities — so this is a friend being sent a
                            // funny picture, and it is named for that.
                            //
                            // Where the browser can hand a file to the share
                            // sheet, that is the whole of it: one tap and
                            // WhatsApp, Telegram, Signal and Mail are in the
                            // same list the phone uses for everything else.
                            // That call has to happen *here*, inside the click,
                            // because every browser refuses a share with no user
                            // gesture and refuses it silently — deferred to the
                            // next frame it would look like a dead button.
                            //
                            // Everywhere else — and in a browser with no file
                            // sharing — it falls back to writing the picture out
                            // and opening a message to send it from.
                            let qsl_ok = savable;
                            let direct = crate::download::can_share_file();
                            let share_tip = if !qsl_ok {
                                "The picture is still loading"
                            } else if direct {
                                "Send this picture to someone — WhatsApp, Telegram, or anything \
                 else on this device"
                            } else {
                                "Save this picture and open a message to send it"
                            };
                            let qsl = crate::chrome::chip(ui, false, "Share picture…")
                                .on_hover_text(share_tip);
                            if qsl_ok && qsl.clicked() {
                                let bytes = self.sstv.full_png.clone();
                                if let Some((name, png)) = bytes {
                                    // **Only ask for the sheet where the browser
                                    // said it can take a file.** `navigator.share`
                                    // returns a *promise*, and the call reports
                                    // only that the promise was created — a
                                    // rejection (files unsupported, no user
                                    // gesture, operator dismissed) is invisible
                                    // to it. So a click that "shared" and showed
                                    // nothing is exactly what happened on both
                                    // platforms (fork discussion #16). This is
                                    // the same `canShare` answer the hover text
                                    // already uses, and where it is no, the
                                    // composer opens instead of doing nothing.
                                    //
                                    // The call still comes *inside* the click:
                                    // every browser refuses a share with no user
                                    // gesture, and refuses it silently.
                                    if !direct
                                        || !crate::download::share_file(
                                            &name,
                                            &png,
                                            crate::download::Mime::Png,
                                        )
                                    {
                                        email_qsl = true;
                                    }
                                }
                            }
                            // The relay convention: a picture you could not copy
                            // is sent back out so the stations who missed it can
                            // pick it up. It loads the transmit slot and stops
                            // there — arming a transmitter from a click in a
                            // picture window would key up on a misclick, and
                            // every other route onto the air in this program is
                            // a deliberate press of TX.
                            // Composing a picture is worth doing on a receiver; *sending* it is
                            // not, so on a listen-only radio the chip is greyed
                            // with the reason rather than offered and
                            // refused — the same rule the TX button follows.
                            let reupload_tip = if self.tx_capable() {
                                format!(
                                    "Send this picture back out — loads transmit slot {}",
                                    self.sstv.selected_slot + 1
                                )
                            } else {
                                "This radio cannot transmit".to_string()
                            };
                            if savable {
                                let ru = self.tx_capable();
                                if crate::chrome::chip(ui, ru, "Re-upload")
                                    .on_hover_text(reupload_tip)
                                    .clicked()
                                {
                                    reupload = true;
                                }
                            }
                            // Two presses, the second one red: the file goes
                            // from the radio's disk and there is nothing to
                            // undo it with.
                            let del = if armed {
                                crate::chrome::chip_accent(
                                    ui,
                                    true,
                                    tr!("sstv-delete-sure"),
                                    crate::theme::PINK(),
                                    crate::theme::INK_ON_CYAN(),
                                )
                                .on_hover_text(tr!("sstv-click-again-delete-good"))
                            } else {
                                crate::chrome::chip(ui, false, tr!("sstv-delete"))
                                    .on_hover_text(&del_hint)
                            };
                            if del.clicked() {
                                pressed_delete = true;
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{} · {}×{}",
                                    r.entry.name, r.entry.width, r.entry.height
                                ))
                                .size(10.0)
                                .weak(),
                            );
                        });
                        // RIFP knows where a picture came from and how it was
                        // carried; SSTV knows none of that, and says nothing.
                        if let Some(m) = &r.entry.rifp {
                            ui.add_space(4.0);
                            let from = m.sender.as_deref().unwrap_or("unidentified");
                            ui.label(
                                RichText::new(format!(
                                    "{from} · {} · {}×{} {}-bit · {} / {} · {} octets in {} chunks \
                                     ({} first pass) · session {}",
                                    m.filename,
                                    m.width,
                                    m.height,
                                    m.bits_per_pixel,
                                    m.media_type,
                                    m.content_encoding,
                                    m.encoded_size,
                                    m.chunk_count,
                                    m.chunks_first_pass,
                                    m.session,
                                ))
                                .size(10.5)
                                .weak(),
                            );
                            if let Some(hint) = &m.hint {
                                ui.label(RichText::new(hint).size(11.0).italics());
                            }
                        }
                    });
            } else {
                open = false;
            }
            if save {
                if let Some((name, png)) = &self.sstv.full_png {
                    crate::download::save_as(name, png, crate::download::Mime::Png);
                }
            }
            // Opening the QSL composer, not sending anything: the message is
            // composed here and the mail client opens from it.
            if email_qsl && let Some((name, _)) = &self.sstv.full_png {
                let name = name.clone();
                let station_line = self.station_line(&name);
                let qsl = SstvQsl::default();
                self.sstv.qsl = Some(SstvQsl {
                    name: name.clone(),
                    // Prefilled from what the station knows: a QSL that has to
                    // be typed out from scratch is a QSL that does not get
                    // sent. The address is the one thing never guessed — a
                    // wrong recipient is worse than an empty field.
                    subject: format!("SSTV picture — {name}"),
                    body: default_qsl_body(&qsl, &name, &station_line),
                    // The report's date is the reception's, not the moment the
                    // window opened — a picture received at 02:14 gets a QSL
                    // saying 02:14 whatever time the reply is written.
                    date: Self::reception_time_text(&name),
                    ..Default::default()
                });
            }
            // Loaded into the slot, not keyed. The slot is already selected, so
            // the picture appears in the TRANSMIT column on the other side of
            // the divider and the operator sends it from there — which is the
            // same two deliberate acts every other transmission takes.
            if reupload && let Some((_, png)) = &self.sstv.full_png {
                let slot = self.sstv.selected_slot;
                let png = png.clone();
                self.sstv.set_slot(slot, png, cmds);
            }
            // First press arms the chip, second sends it. The gallery entry
            // stays until the engine confirms the file is gone.
            if pressed_delete {
                if let Some(name) = self.sstv.received.get(idx).map(|r| r.entry.name.clone()) {
                    if self.sstv.confirm_delete.as_deref() == Some(name.as_str()) {
                        self.sstv.confirm_delete = None;
                        cmds.push(Command::ImageDelete { kind: ImageKind::Sstv, name });
                    } else {
                        self.sstv.confirm_delete = Some(name);
                    }
                }
            }
            if !open {
                self.sstv.enlarged = None;
                self.sstv.full_asked = None;
                self.sstv.full_gone = false;
                self.sstv.confirm_delete = None;
            }
        }
    }

    /// The editor for the banner drawn across the top of every transmitted
    /// picture: the two texts, the two colours, and how tall the strip is.
    ///
    /// A window rather than another row in the transmit column, because this is
    /// set once for the station and then left alone, and the column it would
    /// have gone in is already the narrow half of a split panel.
    /// The send window: an address, a subject and a message, pre-filled — the
    /// fallback for where the browser cannot hand a file to the share sheet.
    ///
    /// A window rather than a chip that acts immediately, because a message
    /// names a person and says something in your own voice — two things that
    /// must not be guessed or fired off in one press. The picture is written out
    /// and the mail client opened from here; **attaching it stays the operator's
    /// own click**, and the window says so rather than implying it was sent.
    ///
    /// Nothing here is persisted. A mail address is not ours to write into a
    /// settings file, and a half-written message is not worth keeping across a
    /// session; the station's own relay fields carry the lasting record.
    fn qsl_window(&mut self, ctx: &egui::Context) {
        let Some(mut qsl) = self.sstv.qsl.clone() else {
            return;
        };
        let mut open = true;
        let mut send = false;
        let mut close = false;
        // A browser cannot attach a file to a message, so there the honest
        // offer is "the picture is saved; attach it yourself" rather than a
        // button that opens a composer with nothing in it. Said in the window
        // too, so the limit is visible before anyone is surprised by it.
        let can_attach = !cfg!(target_arch = "wasm32");
        egui::Window::new("Send this picture")
            .id(crate::layout::salted_id(ctx, "Send this picture"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 460.0))
            .frame(crate::chrome::window_frame())
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.label(RichText::new(&qsl.name).size(10.0).weak());
                // What the last send actually did. A button that worked and a
                // button that quietly did nothing look identical until the
                // operator waited for a mail window that never came.
                if let Some(note) = self.sstv.qsl_note.as_deref() {
                    ui.label(RichText::new(note).size(10.0).color(crate::theme::YELLOW()));
                }
                ui.add_space(4.0);
                ui.label(RichText::new("To").size(10.5).strong());
                ui.text_edit_singleline(&mut qsl.to);
                ui.label(RichText::new("Subject").size(10.5).strong());
                ui.text_edit_singleline(&mut qsl.subject);
                ui.label(RichText::new("Message").size(10.5).strong());
                ui.add(
                    egui::TextEdit::multiline(&mut qsl.body)
                        .desired_width(ui.available_width())
                        .desired_rows(6),
                );
                ui.add_space(6.0);
                // The exchange, in the order it reads on the card. A QSL
                // somebody can pin up carries the report and the date; without
                // these the card is a photograph with a callsign on it.
                ui.label(RichText::new("The card").size(10.5).strong());
                ui.label(
                    RichText::new(
                        "These go on a printed card beside the picture. Leave any of them \
                         blank and it is simply not drawn — a card never reports a report \
                         nobody gave.",
                    )
                    .size(10.0)
                    .weak(),
                );
                egui::Grid::new("qsl-exchange").num_columns(2).spacing([12.0, 4.0]).show(
                    ui,
                    |ui| {
                        ui.label(RichText::new("Date (UTC)").size(10.0));
                        ui.text_edit_singleline(&mut qsl.date);
                        ui.end_row();
                        ui.label(RichText::new("RST sent").size(10.0));
                        ui.text_edit_singleline(&mut qsl.rst_tx);
                        ui.end_row();
                        ui.label(RichText::new("RST received").size(10.0));
                        ui.text_edit_singleline(&mut qsl.rst_rx);
                        ui.end_row();
                        ui.label(RichText::new("Note").size(10.0));
                        ui.text_edit_singleline(&mut qsl.comment);
                        ui.end_row();
                    },
                );
                ui.add_space(4.0);
                let addressed = qsl.to.contains('@') && !qsl.to.contains(char::is_whitespace);
                if !addressed {
                    ui.label(
                        RichText::new("Add an address to send this.")
                            .size(10.0)
                            .color(crate::theme::YELLOW()),
                    );
                }
                if can_attach {
                    ui.label(
                        RichText::new(
                            "Saves the picture and opens your mail client — attaching it is \
                             one click there.",
                        )
                        .size(10.0)
                        .weak(),
                    );
                } else {
                    ui.label(
                        RichText::new(
                            "A browser tab cannot attach a file to a message, so this saves the \
                             picture and opens the message; add the attachment by hand.",
                        )
                        .size(10.0)
                        .weak(),
                    );
                }
                ui.horizontal(|ui| {
                    // The label says what the button does, and it does the same
                    // thing on **both** platforms: writes the picture out and
                    // opens a mail composer. What `can_attach` changes is only
                    // what happens next — a browser cannot pre-attach the file,
                    // so the operator adds it by hand — and that is the
                    // tooltip's job to say, not the label's.
                    //
                    // It used to read "Save picture" in a browser, which
                    // *understated* it: the composer still opened, because the
                    // send below runs `open_external` either way. A button that
                    // does more than it says is the same fault as one that does
                    // less — the operator gets a mail window they did not ask
                    // for and cannot account for.
                    let send_label = "Save picture & open mail";
                    if crate::chrome::chip(ui, addressed, send_label).clicked() && addressed {
                        send = true;
                        close = true;
                    }
                    if crate::chrome::chip(ui, false, "Cancel").clicked() {
                        close = true;
                    }
                });
            });
        // Keep what was typed even if the window is dragged shut, so a
        // mis-click does not cost a typed QSL.
        if !close && open {
            self.sstv.qsl = Some(qsl);
        } else if !send {
            self.sstv.qsl = None;
        } else {
            // The link is built from the fields *before* the struct is stored,
            // because storing it moves them — and the fields are the thing the
            // link is made of.
            let link = mailto(&qsl.to, &qsl.subject, &qsl.body);
            let body = qsl.body.clone();
            // Written before the client opens, so the file is on disk when the
            // operator gets to the attachment field.
            let written = self.write_qsl_picture(&qsl);
            self.sstv.qsl = Some(SstvQsl { written: written.clone(), ..qsl });
            if written.is_some() {
                // eframe drops `open_url` on the desktop, so the message was never
                // opened in the native build. The picture is already saved by
                // this point and the text goes on the clipboard regardless, so
                // nothing is lost when there is no mail handler either.
                ctx.copy_text(body);
                if !crate::download::open_external(&link) {
                    self.sstv.qsl_note = Some(
                        "Picture saved and the message copied — paste it into your mail \
                         program. No mail handler could be opened."
                            .into(),
                    );
                }
            }
        }
    }

    /// "A 0.003.572.950 Hz · 80M · SSTV" — where the picture was heard, for the
    /// reception report. Empty when there is no picture name to read it from.
    ///
    /// From the panel's own state rather than from the picture's file name, so
    /// it says where the receiver was at the time it was looking, which is what
    /// a reception report is about.
    fn station_line(&self, name: &str) -> String {
        let _ = name;
        let hz = self.state.rx_freq_hz();
        let mode = self.state.rx[0].mode;
        let band = self.state.band;
        let mut parts = vec![format!("{:.3} MHz", hz / 1e6), mode.label().to_string()];
        let b = band.label();
        if !b.is_empty() {
            parts.push(b.to_string());
        }
        parts.join(" · ")
    }

    /// When the picture was received, as `YYYY-MM-DD HH:MM UTC` — read out of
    /// the file name, which the engine writes the reception time into.
    ///
    /// The reception's time, not the moment the QSL window opened: a picture
    /// copied at 02:14 still reports 02:14 however long the reply takes to
    /// write. `None` when the name is not one of ours, and the report then says
    /// nothing about the time rather than guessing.
    fn reception_time_text(name: &str) -> String {
        match sdroxide_types::received_at(sdroxide_types::ImageKind::Sstv, name) {
            Some(unix) if unix >= 0 => {
                let (y, mo, d, h, mi) = {
                    let (y, mo, d, h, mi, _s) = sdroxide_types::utc_ymd_hms(unix);
                    (y, mo, d, h, mi)
                };
                format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
            }
            _ => String::new(),
        }
    }

    /// Write the picture out and say where it went.
    ///
    /// **The picture itself, not a composed card.** A QSL by mail from SSTV is
    /// the picture the other station sent, with the report typed underneath it.
    /// Laying the exchange over the picture was a different feature and it did
    /// not belong here.
    ///
    /// Returns the path, or `None` when there is nothing to write.
    fn write_qsl_picture(&mut self, qsl: &SstvQsl) -> Option<String> {
        // Both taken by value: the mail link is built from the fields and the
        // bytes are handed to a thread, and a borrow would be live across both.
        let (name, png) = self.sstv.full_png.clone()?;
        if qsl.name != name {
            // The window is open for a picture the panel has since moved on
            // from; writing the wrong one would be worse than doing nothing.
            return None;
        }
        crate::download::save_as(&name, &png, crate::download::Mime::Png);
        Some(crate::app::panels::sstv::qsl_default_path(&name))
    }

    /// **Recompose the composer's preview** when the document moved, the base
    /// changed or the transmit size changed.
    ///
    /// Composed at the transmit size rather than at a convenient size and
    /// scaled, because what the operator drags is what will go out: an outline
    /// one pixel thick at 320 pixels is not one pixel thick at 640.
    fn composer_preview(&mut self, ctx: &egui::Context, dims: (u16, u16)) {
        if !self.sstv.composer_dirty {
            return;
        }
        self.sstv.composer_dirty = false;
        let base = self
            .sstv
            .slots
            .get(self.sstv.selected_slot)
            .and_then(|s| s.as_ref())
            .map(|s| (s.src_rgb.clone(), s.sw, s.sh));
        let src = base.as_ref().map(|(rgb, w, h)| (rgb.as_slice(), *w, *h));
        let doc = crate::sstv_composer::shown(
            &self.sstv.composer_edit,
            &self.sstv.presets,
            self.sstv.selected_slot,
        );
        let (rgb, w, h) = crate::sstv_composer::render(doc, dims.0, dims.1, src);
        let img = crate::sstv::color_image(&rgb, w, h);
        self.sstv.composer_tex =
            Some(ctx.load_texture("sstv_composer", img, egui::TextureOptions::NEAREST));
    }

    /// Claim the selected slot's document for editing, taking the engine's as
    /// the starting point and seeding a preset nobody has composed from what it
    /// sends today.
    fn composer_claim(&mut self, dims: (u16, u16)) {
        let slot = self.sstv.selected_slot;
        if !crate::sstv_composer::claim(&mut self.sstv.composer_edit, &self.sstv.presets, slot) {
            return;
        }
        self.sstv.composer_sel = None;
        self.sstv.composer_dirty = true;
        self.sstv.preview_dirty = true;
        let seeded = self.sstv.composer_edit.as_ref().is_some_and(|(_, d)| !d.is_active());
        if !seeded {
            return;
        }
        // **QSO-sized, not banner-sized.** The strip the station sends today is
        // 16 pixels tall with 11-pixel letters, which is the complaint that
        // started this: the editor opens with a line that can be read after a
        // noisy decode, and the operator can shrink it.
        let frame_h = dims.1.max(1);
        let caption = self.sstv.presets.slot(slot).message.clone();
        let seed = match crate::sstv::Banner::from_config(&self.digi_cfg_edit) {
            Some(b) => {
                use sdroxide_types::{Face, Strip};
                let px = (f32::from(b.height) * 11.0 / 16.0
                    * (f32::from(b.text_pct.max(25)) / 100.0))
                .round()
                .max(f32::from(frame_h) / 10.0)
                .max(18.0) as u16;
                let thickness = (u32::from(px) * 4 / 3).max(u32::from(b.height)) as u16;
                let strip = Strip { thickness, fill: b.fill, fill2: b.fill2 };
                sdroxide_types::Composer::seeded(
                    &b.left,
                    &b.right,
                    &caption,
                    Face::ChakraBold,
                    px,
                    Some(strip),
                    frame_h,
                )
            }
            None => sdroxide_types::Composer::seeded(
                "",
                "",
                &caption,
                sdroxide_types::Face::ChakraBold,
                (frame_h / 10).max(18),
                None,
                frame_h,
            ),
        };
        if let Some((_, doc)) = self.sstv.composer_edit.as_mut() {
            *doc = seed;
        }
        self.sstv.composer_sel = self
            .sstv
            .composer_edit
            .as_ref()
            .and_then(|(_, d)| (!d.layers.is_empty()).then_some(0));
        self.sstv.composer_dirty = true;
    }

    /// Hand an edited document over on the way out of the editor: a slot switch
    /// or a closed window.
    ///
    /// Only an **edited** one is written. Opening the editor seeds a draft from
    /// what the preset sends today, and merely looking at a preset must not
    /// change what it sends.
    fn composer_settle(&mut self, cmds: &mut Vec<Command>) {
        if self.sstv.composer_since.is_some() {
            self.composer_flush(cmds);
        }
    }

    /// Send the document to the engine when it differs from what the engine
    /// holds, once — not on every dragged pixel, because the engine writes it to
    /// disk and every other screen adopts it.
    fn composer_flush(&mut self, cmds: &mut Vec<Command>) {
        let cmd = crate::sstv_composer::pending(
            &self.sstv.composer_edit,
            &self.sstv.presets,
            &self.sstv.composer_sent,
        );
        self.sstv.composer_since = None;
        if let Some(cmd) = cmd {
            self.sstv.composer_sent = self.sstv.composer_edit.clone();
            cmds.push(cmd);
        }
        // The station overlay is part of the operator's config, and a config is
        // written whole — the same road the banner editor takes.
        if self.sstv.editing_overlay {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
    }

    /// **The picture editor**: the selected preset's picture with its document
    /// drawn over it, each line draggable, each with its own face, size, colour
    /// and strip — and the buttons that turn it into a picture.
    ///
    /// The document is the preset's own, held by the engine, so a second screen
    /// attached to the same radio draws the same picture and the layout survives
    /// a restart. What is edited here is the claimed working copy; the write goes
    /// out when it settles.
    fn composer_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>, mode: Mode) {
        use sdroxide_types::{Strip, TextLayer};
        if !self.sstv.composer_open {
            return;
        }
        let rifp = mode.is_rifp();
        let dims = if rifp {
            self.digi_cfg_edit.rifp_size.dimensions()
        } else {
            self.sstv.tx_mode.dimensions()
        };
        // A different slot in front means a different document: hand the old one
        // back first, so its last edit is not lost to the switch.
        if self.sstv.composer_edit.as_ref().is_some_and(|(i, _)| *i != self.sstv.selected_slot) {
            self.composer_settle(cmds);
            self.sstv.composer_edit = None;
        }
        self.composer_claim(dims);
        self.composer_preview(ctx, dims);
        let (fw, fh) = (f32::from(dims.0.max(1)), f32::from(dims.1.max(1)));
        let editing_overlay = self.sstv.editing_overlay;
        let mut doc = if editing_overlay {
            self.digi_cfg_edit.station_overlay.clone()
        } else {
            self.sstv.composer_edit.as_ref().map(|(_, d)| d.clone()).unwrap_or_default()
        };
        // What the preview shows when the editor is on the station overlay: the
        // overlay under this slot's own document, which is what the picture will
        // actually say.
        if editing_overlay {
            let own = crate::sstv_composer::shown(
                &self.sstv.composer_edit,
                &self.sstv.presets,
                self.sstv.selected_slot,
            );
            let combined = own.over(&doc);
            let base = self
                .sstv
                .slots
                .get(self.sstv.selected_slot)
                .and_then(|s| s.as_ref())
                .map(|s| (s.src_rgb.clone(), s.sw, s.sh));
            let src = base.as_ref().map(|(rgb, w, h)| (rgb.as_slice(), *w, *h));
            let (rgb, w, h) = crate::sstv_composer::render(&combined, dims.0, dims.1, src);
            let img = crate::sstv::color_image(&rgb, w, h);
            self.sstv.composer_tex =
                Some(ctx.load_texture("sstv_composer", img, egui::TextureOptions::NEAREST));
        }
        let mut open = true;
        let mut dirty = false;
        let mut send_now = false;
        let mut new_slot: Option<usize> = None;
        let mut note: Option<String> = None;
        egui::Window::new("Picture editor")
            .id(crate::layout::salted_id(ctx, "Picture editor"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(crate::layout::window_w(ctx, 840.0))
            .frame(crate::chrome::window_frame())
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                // **Two documents live here**: this slot's own and the station's
                // overlay. The switch says which one the list edits, because a
                // layer list with no label is a list that silently belongs to
                // something else.
                ui.horizontal(|ui| {
                    if crate::chrome::chip(ui, !editing_overlay, &format!(
                        "Slot {}", self.sstv.selected_slot + 1
                    ))
                    .on_hover_text("This transmit slot's own layers")
                    .clicked()
                    {
                        self.sstv.editing_overlay = false;
                        dirty = true;
                    }
                    if crate::chrome::chip(ui, editing_overlay, "Station overlay")
                        .on_hover_text(
                            "One set of layers every slot draws underneath its own — the \
                             callsign, in the corner you like. Set once, composed with each \
                             slot's own text rather than replacing it.",
                        )
                        .clicked()
                    {
                        self.sstv.editing_overlay = true;
                        dirty = true;
                    }
                    ui.add_space(10.0);
                    let mut on = self.sstv.presets.slot(self.sstv.selected_slot).apply_overlay;
                    let resp = ui
                        .add_enabled_ui(!editing_overlay, |ui| {
                            crate::chrome::checkbox(ui, &mut on, "draw it under this slot")
                        })
                        .inner
                        .on_hover_text(
                            "Draw the station overlay under this slot's own layers. Off for a \
                             picture that already carries the identification — a retransmitted \
                             mosaic, usually — so it is not stamped twice.",
                        );
                    if resp.changed() {
                        cmds.push(Command::ImageSetOverlay {
                            slot: self.sstv.selected_slot as u8,
                            on,
                        });
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(if editing_overlay {
                        "The station overlay: every slot draws these layers under its own. \
                         Drag a line on the picture to move it."
                    } else {
                        "This slot's own text, drawn where you put it. Drag a line on the \
                         picture to move it; a line's strip moves with it."
                    })
                    .size(10.5)
                    .weak(),
                );
                ui.add_space(6.0);
                let canvas_w = (ui.available_width() * 0.46).clamp(260.0, 420.0);
                ui.horizontal_top(|ui| {
                    // ── the picture, and the drag ─────────────────────────
                    ui.vertical(|ui| {
                        ui.set_width(canvas_w);
                        let shown = egui::vec2(canvas_w, canvas_w * fh / fw);
                        let (rect, resp) =
                            ui.allocate_exact_size(shown, egui::Sense::click_and_drag());
                        if let Some(tex) = &self.sstv.composer_tex {
                            ui.painter().image(
                                tex.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                Color32::WHITE,
                            );
                        }
                        if !self.sstv.presets.slot(self.sstv.selected_slot).has_picture() {
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "No picture in this slot — load one below",
                                egui::FontId::proportional(12.0),
                                crate::theme::gray(150),
                            );
                        }
                        // The selected layer's box, so "the one I am editing"
                        // is visible on the picture rather than only in a list.
                        if let Some(i) = self.sstv.composer_sel
                            && let Some((x0, y0, x1, y1)) =
                                crate::sstv_composer::layer_box(&doc, i, dims.0, dims.1)
                        {
                            let to_screen = |x: i32, y: i32| {
                                egui::pos2(
                                    rect.min.x + x as f32 / fw * rect.width(),
                                    rect.min.y + y as f32 / fh * rect.height(),
                                )
                            };
                            let r = egui::Rect::from_min_max(to_screen(x0, y0), to_screen(x1, y1));
                            ui.painter().rect_stroke(
                                r,
                                2.0,
                                egui::Stroke::new(1.0, crate::theme::CYAN()),
                                egui::StrokeKind::Outside,
                            );
                        }
                        // Pointer → fraction of the frame. One conversion, used
                        // by the hit test and by the move alike.
                        let frac = |p: egui::Pos2| {
                            (
                                ((p.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
                                ((p.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
                            )
                        };
                        if resp.drag_started()
                            && let Some(p) = ui.ctx().pointer_latest_pos()
                        {
                            let (nx, ny) = frac(p);
                            if let Some(i) =
                                crate::sstv_composer::layer_at(&doc, nx * fw, ny * fh, dims.0, dims.1)
                            {
                                let l = &doc.layers[i];
                                self.sstv.composer_drag = Some((i, l.x - nx, l.y - ny));
                                self.sstv.composer_sel = Some(i);
                            } else {
                                self.sstv.composer_drag = None;
                            }
                        }
                        if resp.dragged()
                            && let Some((i, ox, oy)) = self.sstv.composer_drag
                            && let Some(p) = ui.ctx().pointer_latest_pos()
                            && let Some(l) = doc.layers.get_mut(i)
                        {
                            let (nx, ny) = frac(p);
                            let (x, y) = ((nx + ox).clamp(0.0, 1.0), (ny + oy).clamp(0.0, 1.0));
                            if (l.x - x).abs() > 0.0005 || (l.y - y).abs() > 0.0005 {
                                l.x = x;
                                l.y = y;
                                dirty = true;
                            }
                        }
                        if resp.drag_stopped() {
                            self.sstv.composer_drag = None;
                            // A drag is over: hand the document to the engine now
                            // rather than waiting for the pause timer, because
                            // the operator's next move may be to press SEND.
                            send_now = true;
                        }
                        ui.add_space(6.0);

                        // ── what is under the text ────────────────────────
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Base").size(10.5).weak())
                                .on_hover_text("Which slot's picture these layers are drawn over");
                            for i in 0..IMAGE_SLOTS {
                                let sel = self.sstv.selected_slot == i;
                                let has = self.sstv.presets.slot(i).has_picture();
                                if crate::chrome::chip(ui, sel, &format!("{}", i + 1))
                                    .on_hover_text(if has {
                                        "Compose over this slot's picture"
                                    } else {
                                        "This slot is empty"
                                    })
                                    .clicked()
                                    && !sel
                                {
                                    new_slot = Some(i);
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            let sel = self.sstv.selected_slot;
                            if crate::chrome::chip(ui, false, "Picture…")
                                .on_hover_text("Load a picture into this slot")
                                .clicked()
                            {
                                self.sstv.pick_target = Some(sel);
                                pick_image(self.sstv.inbox.clone());
                            }
                            // **Reply to the picture that just came in.** The
                            // QSO flow is: his picture, my call over it, back on
                            // the air — and that is this button.
                            let received = self
                                .sstv
                                .fresh
                                .as_ref()
                                .map(|(_, png)| png.clone())
                                .or_else(|| self.sstv.full_png.as_ref().map(|(_, png)| png.clone()));
                            let use_received = ui
                                .add_enabled_ui(received.is_some(), |ui| {
                                    crate::chrome::chip(ui, false, "Use received")
                                })
                                .inner
                                .on_hover_text(
                                    "Put the picture that just arrived under these layers, so a \
                                     reply carries his picture and your call",
                                );
                            if use_received.clicked()
                                && let Some(png) = received
                            {
                                self.sstv.set_slot(sel, png, cmds);
                                note = Some("the received picture is now the base".into());
                            }
                        });
                    });
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // ── the layers ────────────────────────────────────────
                    ui.vertical(|ui| {
                        ui.set_min_width(300.0);
                        ui.horizontal(|ui| {
                            if crate::chrome::chip(ui, false, "＋ Text").clicked() {
                                doc.layers.push(TextLayer {
                                    text: "TEXT".into(),
                                    px: 28,
                                    ..Default::default()
                                });
                                self.sstv.composer_sel = Some(doc.layers.len() - 1);
                                dirty = true;
                            }
                            if crate::chrome::chip(ui, false, "＋ Strip").clicked() {
                                // A strip is a line with its own band and no
                                // text: the plain banner, for a picture that
                                // wants colour behind something else.
                                doc.layers.push(TextLayer {
                                    text: String::new(),
                                    px: 24,
                                    band: Some(Strip::default()),
                                    ..Default::default()
                                });
                                self.sstv.composer_sel = Some(doc.layers.len() - 1);
                                dirty = true;
                            }
                        });
                        ui.add_space(4.0);
                        // **How many lines the layout holds**, so a document that
                        // has grown past the visible four is still countable at a
                        // glance — the list scrolls, and a scrollbar alone does not
                        // say whether there are five or fifteen.
                        ui.label(
                            RichText::new(format!(
                                "{} line{}",
                                doc.layers.len(),
                                if doc.layers.len() == 1 { "" } else { "s" }
                            ))
                            .size(9.5)
                            .color(crate::theme::CYAN_DIM()),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("composer-layers")
                            .max_height(130.0)
                            .show(ui, |ui| {
                                for i in 0..doc.layers.len() {
                                    ui.horizontal(|ui| {
                                        let mut shown = !doc.layers[i].hidden;
                                        if ui
                                            .checkbox(&mut shown, "")
                                            .on_hover_text("Draw this line")
                                            .changed()
                                        {
                                            doc.layers[i].hidden = !shown;
                                            dirty = true;
                                        }
                                        let text = doc.layers[i].text.clone();
                                        let label = if text.trim().is_empty() {
                                            "(strip)".to_string()
                                        } else {
                                            crate::app::util::shorten(&text, 26).to_string()
                                        };
                                        let sel = self.sstv.composer_sel == Some(i);
                                        if ui.selectable_label(sel, label).clicked() {
                                            self.sstv.composer_sel = Some(i);
                                        }
                                    });
                                }
                            });
                        ui.add_space(4.0);

                        // The selected layer's own controls.
                        if let Some(i) = self.sstv.composer_sel
                            && i < doc.layers.len()
                        {
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(i > 0, egui::Button::new("↑"))
                                    .on_hover_text("Draw this line later than the ones above it")
                                    .clicked()
                                {
                                    doc.layers.swap(i, i - 1);
                                    self.sstv.composer_sel = Some(i - 1);
                                    dirty = true;
                                }
                                if ui
                                    .add_enabled(
                                        i + 1 < doc.layers.len(),
                                        egui::Button::new("↓"),
                                    )
                                    .on_hover_text("Draw this line earlier")
                                    .clicked()
                                {
                                    doc.layers.swap(i, i + 1);
                                    self.sstv.composer_sel = Some(i + 1);
                                    dirty = true;
                                }
                                if crate::chrome::chip(ui, false, "Copy").clicked() {
                                    let mut l = doc.layers[i].clone();
                                    l.y = (l.y + 0.08).min(1.0);
                                    doc.layers.insert(i + 1, l);
                                    self.sstv.composer_sel = Some(i + 1);
                                    dirty = true;
                                }
                                if crate::chrome::chip(ui, false, "Delete").clicked() {
                                    doc.layers.remove(i);
                                    self.sstv.composer_sel = if doc.layers.is_empty() {
                                        None
                                    } else {
                                        Some(i.min(doc.layers.len() - 1))
                                    };
                                    dirty = true;
                                }
                            });
                            ui.add_space(4.0);
                            let l = &mut doc.layers[i];
                            egui::Grid::new("composer-layer-grid")
                                .num_columns(2)
                                .spacing([10.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Text");
                                    if crate::chrome::field(
                                        ui,
                                        egui::TextEdit::singleline(&mut l.text)
                                            .desired_width(250.0)
                                            .hint_text("what the line says"),
                                    )
                                    .changed()
                                    {
                                        dirty = true;
                                    }
                                    ui.end_row();

                                    ui.label("Face");
                                    egui::ComboBox::from_id_salt("composer-face")
                                        .selected_text(l.face.label())
                                        .width(180.0)
                                        .show_ui(ui, |ui| {
                                            for f in sdroxide_types::Face::ALL {
                                                if ui
                                                    .selectable_value(&mut l.face, f, f.label())
                                                    .clicked()
                                                {
                                                    dirty = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Size");
                                    let resp = crate::chrome::slider(
                                        ui,
                                        egui::Slider::new(&mut l.px, 8..=96).suffix(" px"),
                                    );
                                    dirty |= resp.changed();
                                    ui.end_row();

                                    ui.label("Colour");
                                    ui.horizontal(|ui| {
                                        dirty |= ui
                                            .color_edit_button_srgb(&mut l.ink)
                                            .on_hover_text("The letters")
                                            .changed();
                                        let mut outlined = l.outline.is_some();
                                        if crate::chrome::checkbox(ui, &mut outlined, "outline")
                                            .changed()
                                        {
                                            l.outline = outlined.then_some([0, 0, 0]);
                                            dirty = true;
                                        }
                                        if let Some(oc) = &mut l.outline {
                                            dirty |= ui
                                                .color_edit_button_srgb(oc)
                                                .on_hover_text("The edge around them")
                                                .changed();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Placement");
                                    if crate::chrome::checkbox(ui, &mut l.vertical, "down the frame")
                                        .on_hover_text(
                                            "Turn the line a quarter turn, so it reads downwards — \
                                             a call down the side of the picture",
                                        )
                                        .changed()
                                    {
                                        dirty = true;
                                    }
                                    ui.end_row();

                                    ui.label("Strip");
                                    ui.horizontal(|ui| {
                                        let mut banded = l.band.is_some();
                                        if crate::chrome::checkbox(ui, &mut banded, "behind")
                                            .on_hover_text(
                                                "A strip of colour behind the line, across the \
                                                 picture or down it, centred on the line",
                                            )
                                            .changed()
                                        {
                                            l.band = banded.then(|| Strip {
                                                thickness: 28,
                                                fill: [160, 16, 16],
                                                fill2: None,
                                            });
                                            dirty = true;
                                        }
                                        if let Some(b) = &mut l.band {
                                            dirty |= ui
                                                .color_edit_button_srgb(&mut b.fill)
                                                .on_hover_text("The strip, at its first edge")
                                                .changed();
                                            let mut grad = b.fill2.is_some();
                                            if crate::chrome::checkbox(ui, &mut grad, "fade")
                                                .changed()
                                            {
                                                b.fill2 = grad.then_some([0, 0, 0]);
                                                dirty = true;
                                            }
                                            if let Some(c) = &mut b.fill2 {
                                                dirty |= ui.color_edit_button_srgb(c).changed();
                                            }
                                        }
                                    });
                                    ui.end_row();
                                    if let Some(b) = &mut l.band {
                                        ui.label("Thickness");
                                        let resp = crate::chrome::slider(
                                            ui,
                                            egui::Slider::new(&mut b.thickness, 4..=120)
                                                .suffix(" px"),
                                        );
                                        dirty |= resp.changed();
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);
                        }

                        // ── the layout library ────────────────────────────
                        // **Saved here, in the engine**, so the console, a browser
                        // tab and next week's session all have the same layouts.
                        ui.label(RichText::new("LAYOUTS").size(9.5).strong().color(
                            crate::theme::CYAN_DIM(),
                        ));
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.sstv.template_name)
                                    .desired_width(150.0)
                                    .hint_text("name this layout"),
                            );
                            let named = !self.sstv.template_name.trim().is_empty();
                            let save = ui
                                .add_enabled_ui(named && doc.is_active(), |ui| {
                                    crate::chrome::chip(ui, false, "Save")
                                })
                                .inner
                                .on_hover_text("Keep this layout under that name");
                            if save.clicked() {
                                let new = sdroxide_types::LayoutTemplate::clean_name(
                                    &self.sstv.template_name,
                                );
                                // **A rename is a save and a forget**, and both
                                // are issued here so the library never holds the
                                // old name beside the new one.
                                if let Some(old) = self.sstv.template_renaming.take()
                                    && old != new
                                {
                                    cmds.push(Command::ImageDeleteTemplate { name: old });
                                    note = Some(format!("renamed to {new:?}"));
                                } else {
                                    note = Some(format!("saved as {new:?}"));
                                }
                                cmds.push(Command::ImageSaveTemplate {
                                    name: new,
                                    composer: doc.clone(),
                                });
                                self.sstv.template_name.clear();
                            }
                            if self.sstv.template_renaming.is_some()
                                && crate::chrome::chip(ui, false, "Cancel").clicked()
                            {
                                self.sstv.template_renaming = None;
                                self.sstv.template_name.clear();
                                note = Some("rename cancelled".into());
                            }
                        });
                        if self.sstv.presets.templates.is_empty() {
                            ui.label(
                                RichText::new("No saved layouts yet — name one above.")
                                    .size(10.0)
                                    .weak(),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            let templates = self.sstv.presets.templates.clone();
                            for t in templates {
                                let load = crate::chrome::chip(ui, false, &t.name)
                                    .on_hover_text("Load this layout into what you are editing");
                                if load.clicked() {
                                    doc = t.composer.clone();
                                    dirty = true;
                                    send_now = true;
                                    note = Some(format!("loaded {:?}", t.name));
                                }
                                if ui
                                    .add(egui::Button::new("✎").small())
                                    .on_hover_text(format!("Rename {:?}", t.name))
                                    .clicked()
                                {
                                    // Into the name field, with the old name kept:
                                    // Save then renames rather than duplicating.
                                    self.sstv.template_name = t.name.clone();
                                    self.sstv.template_renaming = Some(t.name.clone());
                                    note = Some(format!("renaming {:?}", t.name));
                                }
                                if ui
                                    .add(egui::Button::new("⧉").small())
                                    .on_hover_text(format!("Duplicate {:?}", t.name))
                                    .clicked()
                                {
                                    // A copy under a free name, so nothing is
                                    // overwritten by a duplicate.
                                    let base = format!("{} copy", t.name);
                                    let mut name = base.clone();
                                    let mut n = 2u32;
                                    while self.sstv.presets.templates.iter().any(|x| x.name == name) {
                                        name = format!("{base} {n}");
                                        n += 1;
                                    }
                                    cmds.push(Command::ImageSaveTemplate {
                                        name: sdroxide_types::LayoutTemplate::clean_name(&name),
                                        composer: t.composer.clone(),
                                    });
                                    note = Some(format!("duplicated as {name:?}"));
                                }
                                if ui
                                    .add(egui::Button::new("✕").small())
                                    .on_hover_text(format!("Forget the layout {:?}", t.name))
                                    .clicked()
                                {
                                    cmds.push(Command::ImageDeleteTemplate {
                                        name: t.name.clone(),
                                    });
                                    note = Some(format!("forgot {:?}", t.name));
                                }
                            }
                        });
                        ui.add_space(6.0);

                        // ── the mosaic ───────────────────────────────────
                        // **Several received pictures into one frame** — the case
                        // the operator asks for: the four the other station
                        // missed, retransmitted together.
                        egui::CollapsingHeader::new("Mosaic from received pictures")
                            .id_salt("composer-mosaic")
                            .show(ui, |ui| {
                                let names: Vec<String> = self
                                    .sstv
                                    .received
                                    .iter()
                                    .take(12)
                                    .map(|r| r.entry.name.clone())
                                    .collect();
                                if names.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "Nothing received yet — a mosaic is built from \
                                             pictures in the gallery.",
                                        )
                                        .size(10.0)
                                        .weak(),
                                    );
                                    return;
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "Picked {} of {}, laid out in this order — the order \
                                         is the grid position. Each picture keeps its shape; \
                                         the gaps are black.",
                                        self.sstv.mosaic_picks.len(),
                                        crate::sstv_composer::MOSAIC_MAX
                                    ))
                                    .size(10.0)
                                    .weak(),
                                );
                                // **The order, and how to change it.** The grid follows
                                // the count, and a picture's position in it is its
                                // place in this list: two arrows per pick move it
                                // between cells, which is the whole of what a drag
                                // would do, done in a way that cannot drop a picture
                                // somewhere unintended.
                                let picks = self.sstv.mosaic_picks.clone();
                                ui.horizontal_wrapped(|ui| {
                                    for (i, name) in picks.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{}", i + 1))
                                                    .size(10.0)
                                                    .color(crate::theme::CYAN_DIM()),
                                            );
                                            if ui
                                                .add_enabled(i > 0, egui::Button::new("◀").small())
                                                .on_hover_text("Move this picture one cell earlier")
                                                .clicked()
                                            {
                                                self.sstv.mosaic_picks.swap(i, i - 1);
                                            }
                                            if ui
                                                .add_enabled(
                                                    i + 1 < picks.len(),
                                                    egui::Button::new("▶").small(),
                                                )
                                                .on_hover_text("Move this picture one cell later")
                                                .clicked()
                                            {
                                                self.sstv.mosaic_picks.swap(i, i + 1);
                                            }
                                            if crate::chrome::chip(ui, true, name).clicked() {
                                                self.sstv.mosaic_picks.retain(|n| n != name);
                                            }
                                        });
                                    }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    for name in &names {
                                        let picked = self.sstv.mosaic_picks.iter().any(|n| n == name);
                                        let label = crate::app::util::shorten(name, 18);
                                        if crate::chrome::chip(ui, picked, label).clicked() {
                                            if picked {
                                                self.sstv.mosaic_picks.retain(|n| n != name);
                                            } else if self.sstv.mosaic_picks.len()
                                                < crate::sstv_composer::MOSAIC_MAX
                                            {
                                                self.sstv.mosaic_picks.push(name.clone());
                                                cmds.push(Command::ImageGet {
                                                    kind: ImageKind::Sstv,
                                                    name: name.clone(),
                                                });
                                            } else {
                                                self.sstv.mosaic_note = Some(format!(
                                                    "{} is the most a mosaic holds",
                                                    crate::sstv_composer::MOSAIC_MAX
                                                ));
                                            }
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    let ready = !self.sstv.mosaic_picks.is_empty()
                                        && self
                                            .sstv
                                            .mosaic_picks
                                            .iter()
                                            .all(|n| self.sstv.mosaic_pngs.contains_key(n));
                                    let go = ui
                                        .add_enabled_ui(ready, |ui| {
                                            crate::chrome::chip(ui, false, "Compose into this slot")
                                        })
                                        .inner
                                        .on_hover_text(
                                            "Build the mosaic and load it into this slot as its \
                                             picture; the received originals are untouched",
                                        );
                                    if go.clicked() {
                                        let mut cells = Vec::new();
                                        for n in &self.sstv.mosaic_picks {
                                            if let Some(png) = self.sstv.mosaic_pngs.get(n)
                                                && let Some((rgb, w, h)) =
                                                    crate::sstv::decode_image(png)
                                            {
                                                cells.push((rgb, w, h));
                                            }
                                        }
                                        let refs: Vec<(&[u8], u16, u16)> =
                                            cells.iter().map(|(r, w, h)| (r.as_slice(), *w, *h)).collect();
                                        // **Twice the frame**, so the four
                                        // pictures keep the detail they have
                                        // before the transmit path scales them
                                        // down to the mode's size.
                                        let scale = crate::sstv_composer::MOSAIC_SCALE;
                                        let (rgb, w, h) = crate::sstv_composer::mosaic(
                                            dims.0 * scale,
                                            dims.1 * scale,
                                            &refs,
                                        );
                                        if let Some(png) = crate::sstv::encode_png(&rgb, w, h) {
                                            let slot = self.sstv.selected_slot;
                                            self.sstv.set_slot(slot, png, cmds);
                                            note = Some(format!(
                                                "{} pictures composed into slot {}",
                                                refs.len(),
                                                slot + 1
                                            ));
                                        }
                                    }
                                    if crate::chrome::chip(ui, false, "Clear picks").clicked() {
                                        self.sstv.mosaic_picks.clear();
                                    }
                                });
                                if let Some(n) = &self.sstv.mosaic_note {
                                    ui.label(
                                        RichText::new(n).size(10.0).color(crate::theme::YELLOW()),
                                    );
                                }
                            });
                        ui.add_space(6.0);

                        // ── what to do with it ────────────────────────────
                        // The picture the buttons send is the one on screen,
                        // edits from this very frame included, so the working
                        // copy is put back before anything is composed from it.
                        if let Some((_, d)) = self.sstv.composer_edit.as_mut() {
                            *d = doc.clone();
                        }
                        let has_picture =
                            self.sstv.presets.slot(self.sstv.selected_slot).has_picture();
                        ui.horizontal(|ui| {
                            if crate::chrome::chip(ui, false, "Save picture…").clicked() {
                                if let Some(png) = self.sstv.compose_png(dims) {
                                    crate::download::save_as(
                                        "sstv-composed.png",
                                        &png,
                                        crate::download::Mime::Png,
                                    );
                                    note = Some("saved".into());
                                }
                            }
                            let tx_ok = self.tx_capable();
                            let send = ui
                                .add_enabled_ui(tx_ok && has_picture && doc.is_active(), |ui| {
                                    crate::chrome::chip(ui, false, "Send")
                                })
                                .inner
                                .on_hover_text("Transmit this composition now");
                            if send.clicked()
                                && let Some(png) = self.sstv.compose_png(dims)
                            {
                                // The engine gets the document first, so a client
                                // that composes its own copy sends the same
                                // picture as this one.
                                send_now = true;
                                cmds.push(if rifp {
                                    Command::RifpTx { png }
                                } else {
                                    Command::SstvTx { mode: self.sstv.tx_mode, png }
                                });
                                note = Some("sending".into());
                            }
                            if crate::chrome::chip(ui, false, "Forget")
                                .on_hover_text(
                                    "Empty this preset's document — the banner strip and the plain \
                                     caption are back in charge of its picture",
                                )
                                .clicked()
                            {
                                doc.layers.clear();
                                self.sstv.composer_sel = None;
                                dirty = true;
                                send_now = true;
                                note = Some("cleared — the banner and the caption are back".into());
                            }
                        });
                        // **Save the composition into another spot.** The document
                        // *is* the composition, so this copies the document and
                        // the picture under it: the other slot then sends the
                        // same picture, and can still be edited there.
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Copy to slot").size(10.5).weak())
                                .on_hover_text(
                                    "Give another slot this picture and this text, so it can be \
                                     sent from there",
                                );
                            for n in 0..IMAGE_SLOTS {
                                if n != self.sstv.selected_slot
                                    && crate::chrome::chip(ui, false, &format!("{}", n + 1))
                                        .clicked()
                                {
                                    cmds.push(Command::ImageSetComposer {
                                        slot: n as u8,
                                        composer: doc.clone(),
                                    });
                                    if let Some(png) = self.sstv.compose_png(dims) {
                                        self.sstv.set_slot(n, png, cmds);
                                    }
                                    note = Some(format!("copied into slot {}", n + 1));
                                }
                            }
                        });
                        if let Some(n) = &note {
                            ui.add_space(4.0);
                            ui.label(RichText::new(n).size(10.5).color(crate::theme::YELLOW()));
                        }
                    });
                });
            });
        // Write the working copy back, then settle it to the engine if this
        // frame changed anything: instantly for a control, and on a drag's
        // release rather than on every pixel of it.
        if editing_overlay {
            // The station overlay is station-wide by definition, so it is written
            // to the operator's config rather than to a preset. `DigiConfig`
            // rides whole and the engine stores it, which is what makes one
            // overlay serve every slot, the browser and the next session.
            if self.digi_cfg_edit.station_overlay != doc {
                self.digi_cfg_edit.station_overlay = doc;
                dirty = true;
            }
        } else if let Some((_, d)) = self.sstv.composer_edit.as_mut() {
            *d = doc;
        }
        if dirty {
            self.sstv.composer_dirty = true;
            self.sstv.preview_dirty = true;
            self.sstv.composer_since = Some(crate::time::now_unix_f64());
        }
        if send_now {
            // The document must be in the engine before the picture that used
            // it, or another screen would compose a different one.
            self.composer_flush(cmds);
        }
        if let Some(i) = new_slot {
            self.composer_settle(cmds);
            self.sstv.composer_edit = None;
            self.sstv.selected_slot = i;
            self.sstv.preview_dirty = true;
            self.sstv.composer_dirty = true;
            self.composer_claim(dims);
        }
        if !open {
            self.composer_settle(cmds);
            self.sstv.composer_edit = None;
        }
        self.sstv.composer_open = open;
    }

    fn banner_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.sstv.banner_open {
            return;
        }
        let seeded = self.digi_cfg_seeded;
        let mut open = true;
        let mut changed = false;
        egui::Window::new(tr!("sstv-image-banner"))
            .id(crate::layout::salted_id(ctx, "Image banner"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(crate::layout::window_w(ctx, 420.0))
            .frame(crate::chrome::window_frame())
            .show(ctx, |ui| {
                crate::chrome::window_body_bg(ui);
                ui.label(
                    RichText::new(tr!("sstv-drawn-into-every-picture-station")).size(10.5).weak(),
                );
                ui.add_space(6.0);
                let cfg = &mut self.digi_cfg_edit;
                ui.add_enabled_ui(seeded, |ui| {
                    if crate::chrome::checkbox(ui, &mut cfg.sstv_banner, tr!("sstv-draw-banner"))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.add_space(4.0);
                    ui.add_enabled_ui(cfg.sstv_banner, |ui| {
                        // The placeholder list is the whole documentation for
                        // these two fields, so it hangs off both of them.
                        let hint: String = std::iter::once(
                            tr!("sstv-substituted-when-picture-composed").to_string(),
                        )
                        .chain(
                            crate::sstv::BANNER_PLACEHOLDERS
                                .iter()
                                .map(|(k, what)| format!("  {k} — {what}")),
                        )
                        .collect::<Vec<_>>()
                        .join("\n");
                        egui::Grid::new("sstv-banner-grid")
                            .num_columns(2)
                            .spacing([10.0, 7.0])
                            .show(ui, |ui| {
                                ui.label(tr!("sstv-top-left"));
                                let resp = crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut cfg.sstv_banner_left)
                                        .desired_width(240.0)
                                        .hint_text("{call}"),
                                )
                                .on_hover_text(&hint);
                                // On focus loss, like the RIFP caption: this is
                                // persisted engine-side and a config write per
                                // keystroke would be absurd. The preview still
                                // follows every keystroke, because it is
                                // recomposed from `digi_cfg_edit` and not from
                                // what the engine has heard about.
                                if resp.lost_focus() && resp.changed() {
                                    changed = true;
                                }
                                ui.end_row();

                                ui.label(tr!("sstv-top-right"));
                                let resp = crate::chrome::field(
                                    ui,
                                    egui::TextEdit::singleline(&mut cfg.sstv_banner_right)
                                        .desired_width(240.0)
                                        .hint_text("SDRoxide v{version}"),
                                )
                                .on_hover_text(&hint);
                                if resp.lost_focus() && resp.changed() {
                                    changed = true;
                                }
                                ui.end_row();

                                ui.label(tr!("sstv-colours"));
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgb(&mut cfg.sstv_banner_fill)
                                        .on_hover_text(tr!("sstv-strip-top-edge-fades-black"))
                                        .changed();
                                    ui.label(RichText::new("strip").size(10.5).weak());
                                    ui.add_space(8.0);
                                    changed |= ui
                                        .color_edit_button_srgb(&mut cfg.sstv_banner_ink)
                                        .on_hover_text(tr!("sstv-both-texts"))
                                        .changed();
                                    ui.label(RichText::new("text").size(10.5).weak());
                                });
                                ui.end_row();

                                ui.label(tr!("sstv-gradient"));
                                ui.horizontal(|ui| {
                                    changed |= crate::chrome::checkbox(
                                        ui,
                                        &mut cfg.sstv_style.banner_gradient,
                                        tr!("sstv-fade"),
                                    )
                                    .on_hover_text(tr!("sstv-fade-strip-top-colour-second"))
                                    .changed();
                                    ui.add_enabled_ui(cfg.sstv_style.banner_gradient, |ui| {
                                        changed |= ui
                                            .color_edit_button_srgb(
                                                &mut cfg.sstv_style.banner_fill2,
                                            )
                                            .changed();
                                    });
                                });
                                ui.end_row();

                                ui.label(tr!("sstv-text-outline"));
                                ui.horizontal(|ui| {
                                    changed |= crate::chrome::checkbox(
                                        ui,
                                        &mut cfg.sstv_style.banner_outline,
                                        "in",
                                    )
                                    .on_hover_text(tr!("sstv-draw-outline-around-banner-text"))
                                    .changed();
                                    ui.add_enabled_ui(cfg.sstv_style.banner_outline, |ui| {
                                        changed |= ui
                                            .color_edit_button_srgb(
                                                &mut cfg.sstv_style.banner_outline_ink,
                                            )
                                            .changed();
                                    });
                                });
                                ui.end_row();

                                ui.label(tr!("sstv-text-gradient"));
                                ui.horizontal(|ui| {
                                    changed |= crate::chrome::checkbox(
                                        ui,
                                        &mut cfg.sstv_style.banner_ink_gradient,
                                        tr!("sstv-fade"),
                                    )
                                    .on_hover_text(tr!("sstv-fade-banner-text-colour-second"))
                                    .changed();
                                    ui.add_enabled_ui(
                                        cfg.sstv_style.banner_ink_gradient
                                            && !cfg.sstv_style.rainbow_text,
                                        |ui| {
                                            changed |= ui
                                                .color_edit_button_srgb(
                                                    &mut cfg.sstv_style.banner_ink2,
                                                )
                                                .changed();
                                        },
                                    );
                                });
                                ui.end_row();

                                ui.label(tr!("sstv-rainbow-text"));
                                changed |= crate::chrome::checkbox(
                                    ui,
                                    &mut cfg.sstv_style.rainbow_text,
                                    tr!("sstv-all-text-overriding-colours-above"),
                                )
                                .on_hover_text(tr!("sstv-draw-every-text-picture-banner"))
                                .changed();
                                ui.end_row();

                                ui.label(tr!("sstv-message"));
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgb(&mut cfg.sstv_style.message_ink)
                                        .on_hover_text(tr!("sstv-slot-message-s-text-colour"))
                                        .changed();
                                    ui.label(RichText::new("text").size(10.5).weak());
                                    ui.add_space(8.0);
                                    changed |= crate::chrome::checkbox(
                                        ui,
                                        &mut cfg.sstv_style.message_outline,
                                        tr!("sstv-outline"),
                                    )
                                    .changed();
                                    ui.add_enabled_ui(cfg.sstv_style.message_outline, |ui| {
                                        changed |= ui
                                            .color_edit_button_srgb(
                                                &mut cfg.sstv_style.message_outline_ink,
                                            )
                                            .changed();
                                    });
                                });
                                ui.end_row();

                                ui.label(tr!("sstv-height"))
                                    .on_hover_text(tr!("sstv-how-tall-strip-pixels-transmitted"));
                                ui.spacing_mut().slider_width = 180.0;
                                let resp = crate::chrome::slider(
                                    ui,
                                    egui::Slider::new(&mut cfg.sstv_banner_height, 8..=64)
                                        .suffix(" px"),
                                );
                                changed |=
                                    resp.drag_stopped() || (resp.changed() && !resp.dragged());
                                ui.end_row();
                                // **Where the strip sits, and how big the text
                                // is inside it.** A QSO picture wants the call
                                // legible on the far end, and a short strip is
                                // the wrong lever for that: it eats picture. The
                                // strip also belongs at whichever edge the
                                // operator's picture has least of.
                                ui.label("Text size").on_hover_text(
                                    "The banner text as a percentage of the strip's own size. \
                                     100% is what it has always been; a QSO needs the call \
                                     readable after a noisy decode, and this scales the \
                                     lettering without making the strip taller.",
                                );
                                let resp = crate::chrome::slider(
                                    ui,
                                    egui::Slider::new(&mut cfg.sstv_banner_text_pct, 50..=250)
                                        .suffix(" %"),
                                );
                                changed |=
                                    resp.drag_stopped() || (resp.changed() && !resp.dragged());
                                ui.end_row();
                                ui.label("Placement").on_hover_text(
                                    "Which edge the strip is drawn along. The message overlay \
                                     starts at the opposite end, so a banner at the bottom \
                                     leaves the top of the picture for text.",
                                );
                                ui.horizontal(|ui| {
                                    let mut bottom = cfg.sstv_banner_bottom;
                                    if crate::chrome::chip(ui, !bottom, "Top").clicked() {
                                        bottom = false;
                                    }
                                    if crate::chrome::chip(ui, bottom, "Bottom").clicked() {
                                        bottom = true;
                                    }
                                    if bottom != cfg.sstv_banner_bottom {
                                        cfg.sstv_banner_bottom = bottom;
                                        changed = true;
                                    }
                                });
                                ui.end_row();
                            });
                    });
                    ui.add_space(6.0);
                    if ui
                        .button(tr!("sstv-reset"))
                        .on_hover_text(tr!("sstv-back-callsign-left-version-right"))
                        .clicked()
                    {
                        let d = sdroxide_types::DigiConfig::default();
                        cfg.sstv_banner = d.sstv_banner;
                        cfg.sstv_banner_left = d.sstv_banner_left;
                        cfg.sstv_banner_right = d.sstv_banner_right;
                        cfg.sstv_banner_fill = d.sstv_banner_fill;
                        cfg.sstv_banner_ink = d.sstv_banner_ink;
                        cfg.sstv_banner_height = d.sstv_banner_height;
                        cfg.sstv_style = d.sstv_style;
                        changed = true;
                    }
                    if !seeded {
                        ui.label(
                            RichText::new(tr!("sstv-waiting-station-s-digital-mode"))
                                .size(10.0)
                                .weak(),
                        );
                    }
                });
            });
        if changed {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
        self.sstv.banner_open = open;
    }

    /// The RIFP half of the image panel's control strip: profile, picture size
    /// and encoding, robustness, the transfer readout, and the sessions being
    /// reassembled.
    fn rifp_controls(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let st = self.sstv.rifp.clone();
        let seeded = self.digi_cfg_seeded;
        let mut changed = false;

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr!("sstv-rifp")).size(12.0).strong().color(crate::theme::CYAN()),
            );
            // Outside the enabled scope: which frequency to sit on has nothing
            // to do with whether the operator's digi config has loaded yet.
            self.digi_freq_chip(ui, cmds);
            ui.add_enabled_ui(seeded, |ui| {
                for p in RifpProfile::ALL {
                    let active = self.digi_cfg_edit.rifp_profile == p;
                    if crate::chrome::chip(ui, active, p.label())
                        .on_hover_text(format!(
                            "{} — {:.0} baud CPFSK, ±{:.0} Hz, {:.0} kHz occupied bandwidth",
                            p.name(),
                            p.symbol_rate(),
                            p.deviation_hz(),
                            p.bandwidth_hz() / 1000.0,
                        ))
                        .clicked()
                        && !active
                    {
                        self.digi_cfg_edit.rifp_profile = p;
                        changed = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new(tr!("sstv-size")).size(10.0).weak());
                for s in RifpSize::ALL {
                    let active = self.digi_cfg_edit.rifp_size == s;
                    if crate::chrome::chip(ui, active, s.label()).clicked() && !active {
                        self.digi_cfg_edit.rifp_size = s;
                        self.sstv.preview_dirty = true;
                        changed = true;
                    }
                }
            });
        });
        ui.add_space(4.0);

        // The bandwidth warning, and a jump to the calling frequency. RIFP
        // itself is band-agnostic; what is legal is not.
        let dial = self.state.rx_freq_hz();
        ui.horizontal_wrapped(|ui| {
            let profile = self.digi_cfg_edit.rifp_profile;
            if profile.fits_at(dial) {
                ui.label(
                    RichText::new(tr!(
                        "sstv-rifp-profile",
                        profile = profile.name(),
                        khz = format!("{:.0}", profile.bandwidth_hz() / 1000.0)
                    ))
                    .size(10.5)
                    .weak(),
                );
            } else {
                ui.label(
                    RichText::new(tr!(
                        "sstv-rifp-too-wide",
                        profile = profile.name(),
                        khz = format!("{:.0}", profile.bandwidth_hz() / 1000.0)
                    ))
                    .size(10.5)
                    .strong()
                    .color(crate::theme::ALERT()),
                )
                .on_hover_text(tr!(
                    "sstv-rifp-no-frequency",
                    khz = format!("{:.0}", profile.bandwidth_hz() / 1000.0),
                    segments = profile.wide_segments_text()
                ));
            }
            if (dial - sdroxide_types::RIFP_CALLING_HZ).abs() > 1.0
                && crate::chrome::chip(ui, false, "433.920")
                    .on_hover_text(tr!("sstv-calling-frequency-draft-names"))
                    .clicked()
            {
                cmds.push(Command::SetVfo {
                    vfo: self.state.active_vfo,
                    hz: sdroxide_types::RIFP_CALLING_HZ,
                });
            }
        });
        ui.add_space(5.0);

        // Encoding and depth: what the picture is turned into before framing.
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new(tr!("sstv-encode")).size(10.0).weak())
                    .on_hover_text(tr!("sstv-how-picture-encoded-into-object"));
                for e in RifpEncoding::TX_MENU {
                    let active = self.digi_cfg_edit.rifp_encoding == e;
                    let hover = match e.manifest_pair() {
                        Some((mt, ce)) => format!("{mt} / {ce}"),
                        None => tr!("sstv-try-every-encoding-send-smallest"),
                    };
                    if crate::chrome::chip(ui, active, e.label()).on_hover_text(hover).clicked()
                        && !active
                    {
                        self.digi_cfg_edit.rifp_encoding = e;
                        changed = true;
                    }
                }
                ui.separator();
                ui.label(RichText::new(tr!("sstv-gray")).size(10.0).weak())
                    .on_hover_text(tr!("sstv-grayscale-depth-rifp-s-raster"));
                for bits in [1u8, 2, 4, 8] {
                    let active = self.digi_cfg_edit.rifp_bits_per_pixel == bits;
                    if crate::chrome::chip(ui, active, &format!("{bits}b")).clicked() && !active {
                        self.digi_cfg_edit.rifp_bits_per_pixel = bits;
                        changed = true;
                    }
                }
                let mut dither = self.digi_cfg_edit.rifp_dither;
                if crate::chrome::chip(ui, dither, tr!("sstv-dither"))
                    .on_hover_text(tr!("sstv-diffuse-quantisation-error-worth-below"))
                    .clicked()
                {
                    dither = !dither;
                    self.digi_cfg_edit.rifp_dither = dither;
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);

        // The content hint: a caption carried in the manifest itself, so it
        // reaches a receiver as text rather than as pixels they have to read.
        // Distinct from the slot message, which is drawn into the picture.
        ui.horizontal(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new(tr!("sstv-caption")).size(10.0).weak())
                    .on_hover_text(tr!("sstv-sent-manifest-content-hint-shown"));
                let resp = crate::chrome::field(
                    ui,
                    egui::TextEdit::singleline(&mut self.digi_cfg_edit.rifp_content_hint)
                        .desired_width(f32::INFINITY)
                        .hint_text(tr!("sstv-what-picture")),
                );
                // On focus loss rather than per keystroke: this is persisted
                // engine-side, and a config write per character is absurd.
                if resp.lost_focus() && resp.changed() {
                    changed = true;
                }
            });
        });
        ui.add_space(5.0);

        // Robustness: RIFP has no repair requests, so repetition is the only
        // recovery there is.
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(seeded, |ui| {
                ui.label(RichText::new(tr!("sstv-repeat-data")).size(10.0).weak())
                    .on_hover_text(tr!("sstv-send-every-data-frame-many"));
                ui.spacing_mut().slider_width = 90.0;
                changed |= crate::chrome::slider(
                    ui,
                    egui::Slider::new(&mut self.digi_cfg_edit.rifp_data_repeats, 1..=4),
                )
                .drag_stopped();
                ui.label(RichText::new(tr!("sstv-chunk")).size(10.0).weak())
                    .on_hover_text(tr!("sstv-payload-octets-per-data-frame"));
                changed |= crate::chrome::slider(
                    ui,
                    egui::Slider::new(&mut self.digi_cfg_edit.rifp_chunk_size, 32..=1024)
                        .step_by(16.0),
                )
                .drag_stopped();
            });
            ui.separator();
            if st.tx_active {
                ui.label(
                    RichText::new(tr!(
                        "sstv-rifp-tx-frame",
                        frame = st.tx_frame,
                        frames = st.tx_frames,
                        secs = st.tx_remaining_s
                    ))
                    .size(11.0)
                    .strong()
                    .color(crate::theme::ALERT()),
                );
            }
            if let Some(enc) = st.tx_encoding {
                ui.label(
                    RichText::new(tr!(
                        "sstv-rifp-sent-as",
                        encoding = enc.label(),
                        octets = st.tx_bytes
                    ))
                    .size(10.0)
                    .weak(),
                );
            }
        });
        ui.add_space(5.0);

        // Counters and the sessions being reassembled.
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr!(
                    "sstv-rifp-rx-stats",
                    frames = st.rx_frames,
                    bad = st.rx_bad_frames,
                    pictures = st.rx_objects
                ))
                .size(10.0)
                .weak(),
            )
            .on_hover_text(tr!("sstv-valid-frames-frames-failed-crc"));
            if st.sessions.is_empty() {
                ui.label(RichText::new(tr!("sstv-no-transfer-progress")).size(10.0).weak());
            }
            for s in &st.sessions {
                ui.separator();
                let from = s.sender.as_deref().unwrap_or_else(|| shorten(&s.session, 8));
                let label = if s.total > 0 {
                    format!("{from} {}/{}", s.have, s.total)
                } else {
                    format!("{from} {}", s.have)
                };
                let colour =
                    if s.have_manifest { crate::theme::GREEN() } else { crate::theme::YELLOW() };
                ui.label(RichText::new(label).size(10.5).strong().color(colour)).on_hover_text(
                    if s.have_manifest {
                        tr!(
                            "sstv-rifp-session-idle",
                            session = s.session.to_string(),
                            secs = s.idle_s
                        )
                    } else {
                        tr!(
                            "sstv-rifp-session-waiting",
                            session = s.session.to_string(),
                            secs = s.idle_s
                        )
                    },
                );
                rifp_chunk_map(ui, s);
                if crate::chrome::chip(ui, false, "✕")
                    .on_hover_text(tr!("sstv-forget-incomplete-transfer"))
                    .clicked()
                {
                    cmds.push(Command::RifpDropSession(s.session.clone()));
                }
            }
        });
        if let Some(err) = &st.last_error {
            ui.label(RichText::new(err).size(10.0).color(crate::theme::YELLOW()));
        }
        if changed && seeded {
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
    }

    /// How to name a directory the engine reported.
    ///
    /// A path is only a path if the operator can walk to it. Driving a radio on
    /// another machine, it names a directory over there, and saying so is the
    /// difference between a useful hint and a wrong one.
    pub(in crate::app) fn store_where(&self, dir: &str) -> String {
        if self.ctrl.engine_is_remote() {
            tr!("sstv-store-on-radio", dir = dir.to_string())
        } else {
            tr!("sstv-store-in", dir = dir.to_string())
        }
    }
}

/// One QSL message being composed for a received picture.
///
/// Session-only by construction: it holds an address and a half-written
/// sentence, neither of which is ours to keep once the window closes. See
/// [`SstvUi::qsl`].
#[derive(Debug, Clone, Default, PartialEq)]
pub(in crate::app) struct SstvQsl {
    /// Which received picture this is about, by store name. The bytes are
    /// refetched rather than held: a message can sit open while other
    /// pictures arrive, and keeping every one of them would be the gallery's
    /// worth of megabytes the `full_png` doc says not to keep.
    pub name: String,
    pub to: String,
    /// The exchange, for the card's panel. All optional and all free-text: the
    /// card draws a field only when it has a value, so an operator who knows
    /// nothing but the callsign sends a card with the callsign on it.
    pub date: String,
    pub rst_tx: String,
    pub rst_rx: String,
    pub subject: String,
    pub body: String,
    /// The free line at the foot of the card — the one a person writes.
    pub comment: String,
    /// Set once the picture has been written out and the mail client opened,
    /// so the window can say what is waiting rather than offering to do it
    /// again.
    pub written: Option<String>,
}

/// The `mailto:` for a QSL message: the address, the subject and the body,
/// percent-encoded as the scheme requires.
///
/// Built here rather than handed to a string-concatenation at the call site
/// because **every** part of it is operator-typed. An ampersand in the body is
/// the obvious one — it separates the header fields, so a message reading
/// "R-01 & 73" would silently truncate and put the rest in the subject. A
/// newline would end a header line outright. Percent-encoding all of them is
/// the difference between a QSL arriving and a QSL arriving wrong.
pub(in crate::app) fn mailto(to: &str, subject: &str, body: &str) -> String {
    fn enc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.as_bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(*b as char)
                }
                other => out.push_str(&format!("%{other:02X}")),
            }
        }
        out
    }
    format!("mailto:{}?subject={}&body={}", enc(to), enc(subject), enc(body))
}

/// Where a QSL picture is suggested to be written: the pictures folder the
/// store already uses, with the received picture's own name.
///
/// Only a *suggestion* — `download::save_as` opens the dialog and the operator
/// chooses, so this names the default rather than writing anywhere itself. It
/// exists so the window can say where the file went after the dialog closes,
/// which a relative or empty path could not do.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn qsl_default_path(name: &str) -> String {
    match sdroxide_config::image_rx_dir("sstv") {
        Ok(dir) => dir.join(name).to_string_lossy().into_owned(),
        // No pictures folder to name (an unusual home layout). Saying so beats
        // inventing a path that does not exist.
        Err(_) => format!("{name} (saved where you chose)"),
    }
}

/// The browser has no filesystem to name — `sdroxide-config` is native-only, and
/// deliberately so, since it reads the user's home directory. A browser tab
/// downloads to wherever the browser is set to put downloads, which only the
/// browser knows, so the honest wording says that rather than guessing.
#[cfg(target_arch = "wasm32")]
pub(in crate::app) fn qsl_default_path(name: &str) -> String {
    format!("{name} — downloaded to this browser's download folder")
}

/// A short reception note above the operator's message.
///
/// **Only the fields that have something in them.** A report that reads "RST
/// sent: 59" when nobody said 59 is worse than a shorter one: the other station
/// will believe it. So an empty field is left out entirely rather than blanked
/// or filled with a dash.
///
/// Plain text on purpose. This is read by a person on a mail client of unknown
/// vintage, and it has to survive a plain-text part.
pub(in crate::app) fn reception_report(report: &SstvQsl, station_line: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    if !station_line.trim().is_empty() {
        lines.push(format!("Heard: {station_line}"));
    }
    if !report.date.trim().is_empty() {
        lines.push(format!("Date/Time (UTC): {}", report.date.trim()));
    }
    if !report.rst_tx.trim().is_empty() || !report.rst_rx.trim().is_empty() {
        lines.push(format!(
            "RST sent {} / received {}",
            if report.rst_tx.trim().is_empty() { "-" } else { report.rst_tx.trim() },
            if report.rst_rx.trim().is_empty() { "-" } else { report.rst_rx.trim() },
        ));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!("Reception report\n{}", lines.join("\n"))
}

/// The body offered pre-written: the reception note, then room for the
/// operator's own sentence.
///
/// The picture's name is in it because that is what identifies the exchange to
/// somebody looking at their own picture folder later, and it is the one fact
/// the operator cannot be expected to remember.
pub(in crate::app) fn default_qsl_body(
    report: &SstvQsl,
    picture: &str,
    station_line: &str,
) -> String {
    let rep = reception_report(report, station_line);
    format!("Thanks for the picture.\n\n{rep}\n\nPicture: {picture}\n\n73")
}

#[cfg(test)]
mod qsl_tests {
    use super::*;

    /// The reason this function exists. An ampersand in a QSL body is ordinary
    /// — "R-01 & 73" is what people write — and unencoded it ends the field,
    /// so the rest of the message lands in the subject line and the body
    /// arrives cut short. This is the whole reason for building the link here.
    #[test]
    fn an_ampersand_in_a_qsl_does_not_truncate_the_message() {
        let link = mailto("g0abc@example.org", "QSL 14.230", "Thanks for the picture. R-01 & 73");
        assert!(
            link.ends_with("body=Thanks%20for%20the%20picture.%20R-01%20%26%2073"),
            "the body must arrive whole: {link}"
        );
        // One `subject=`, one `body=` — the separator did not survive as a
        // second field.
        assert_eq!(link.matches("subject=").count(), 1, "{link}");
        assert_eq!(link.matches("body=").count(), 1, "{link}");
    }

    /// A newline would end a header line outright, which is a message that
    /// silently loses whatever followed it.
    #[test]
    fn a_newline_cannot_end_a_header_line() {
        let link = mailto("a@b.c", "S\r\nBcc: someone@else", "body");
        assert!(!link.contains('\r') && !link.contains('\n'), "{link}");
        assert!(link.contains("%0D%0A"), "encoded, not stripped: {link}");
        assert!(
            link.matches("subject=").count() == 1 && link.matches("body=").count() == 1,
            "no header was injected: {link}"
        );
    }

    /// A space in an address is the one case that must not be papered over:
    /// percent-encoding it produces a link that cannot address anyone, which
    /// is better than an injection and worse than a message. The empty case
    /// is the honest one to ship — an unset address cannot become a link.
    #[test]
    fn an_address_is_only_used_when_it_looks_like_one() {
        assert!(mailto("", "s", "b").starts_with("mailto:?subject="), "no address, no recipient");
        for bad in ["not an address", "a@b", "@example.org", "a b@example.org"] {
            let to = bad.trim();
            let looks_like_an_address =
                to.contains('@') && !to.contains(char::is_whitespace) && !to.starts_with('@');
            if !looks_like_an_address {
                assert!(
                    !mailto(to, "s", "b").starts_with(&format!("mailto:{bad}?"))
                        || bad.contains(char::is_whitespace),
                    "{bad:?} must not become a recipient"
                );
            }
        }
    }

    /// A default subject and body naming the picture, so the common QSL is one
    /// press rather than a blank form. The picture's own name is the useful
    /// fact — it is what identifies the exchange on the far end.
    #[test]
    fn a_new_qsl_is_prefilled_rather_than_blank() {
        let qsl =
            SstvQsl { name: "2026-10-05-120000-14.230-SSTV.png".into(), ..Default::default() };
        assert_eq!(qsl.to, "", "the address is the operator's, never guessed");
        assert!(qsl.written.is_none());
    }

    /// Once written, the window says where the file went rather than offering
    /// to write it again — the same rule the banner editor and the other
    /// "done" states in this program follow.
    #[test]
    fn a_written_qsl_remembers_where_it_went() {
        let qsl = SstvQsl {
            written: Some("/home/operator/Pictures/qsl.png".into()),
            ..Default::default()
        };
        assert_eq!(qsl.written.as_deref(), Some("/home/operator/Pictures/qsl.png"));
    }
}

#[cfg(test)]
mod reupload_tests {
    use super::*;
    use sdroxide_types::Command;

    /// The Re-upload chip's whole job: the picture the operator is looking at
    /// becomes the transmit slot's picture, so it can be sent back out for the
    /// stations who could not copy it.
    #[test]
    fn a_received_picture_can_be_loaded_into_the_transmit_slot() {
        let mut sstv = SstvUi::default();
        sstv.selected_slot = 2;
        let png = b"\x89PNG-received".to_vec();
        let mut cmds: Vec<Command> = Vec::new();

        sstv.set_slot(sstv.selected_slot, png.clone(), &mut cmds);

        assert_eq!(sstv.selected_slot, 2, "the slot it was aimed at is the one set");
        assert!(sstv.pick_error.is_none(), "an ordinary picture is not a complaint");
        assert_eq!(
            cmds.len(),
            1,
            "one command, not a stream of them: a 40 MB picture pushed per frame \
             would be the very thing the limit below exists to stop"
        );
        match &cmds[0] {
            Command::ImageSetSlot { slot, bytes } => {
                assert_eq!(*slot, 2);
                assert_eq!(bytes, &png, "the bytes are the received picture's own");
            }
            other => panic!("expected the slot to be filled, got {other:?}"),
        }
    }

    /// A picture over the limit is refused **before** anything is sent, and it
    /// says which limit — otherwise Re-upload appears to work and the
    /// transmit column keeps showing the old picture, which is the fault this
    /// fork treats as a bug rather than a shrug.
    #[test]
    fn a_picture_too_big_to_upload_is_refused_with_the_limit_named() {
        let mut sstv = SstvUi::default();
        let mut cmds: Vec<Command> = Vec::new();
        let huge = vec![0u8; sdroxide_types::IMAGE_UPLOAD_MAX + 1];

        sstv.set_slot(0, huge, &mut cmds);

        assert!(cmds.is_empty(), "nothing may go to the engine");
        let err = sstv.pick_error.as_deref().expect("an oversize picture is a complaint");
        assert!(
            err.contains(&format!("{}", sdroxide_types::IMAGE_UPLOAD_MAX / 1_048_576)),
            "the limit is named, so the operator can see what to do: {err}"
        );
    }

    /// And the complaint is cleared by the next picture that fits, so a refusal
    /// does not sit on the panel warning about a load that has since worked.
    #[test]
    fn the_next_picture_that_fits_clears_an_earlier_refusal() {
        let mut sstv = SstvUi::default();
        let mut cmds: Vec<Command> = Vec::new();

        sstv.set_slot(0, vec![0u8; sdroxide_types::IMAGE_UPLOAD_MAX + 1], &mut cmds);
        assert!(sstv.pick_error.is_some());

        sstv.set_slot(0, b"\x89PNG-small".to_vec(), &mut cmds);
        assert!(sstv.pick_error.is_none());
        assert_eq!(cmds.len(), 1, "only the picture that fit was sent");
        assert!(
            matches!(&cmds[0], Command::ImageSetSlot { .. }),
            "the slot is filled by the one command that does it"
        );
    }
}
/// The chip row must not be able to overflow the window, whatever it holds.
///
/// Measured, because this was the fault: at 360 pt the four chips and the
/// label beside them needed ~368 pt of a 344 pt window, and `horizontal` does
/// not wrap. The row wraps now; this pins that no *single* chip can exceed the
/// narrowest window either, which is the other half of "the controls are on
/// screen" — a chip wider than its window is one that cannot be pressed at all.
#[test]
fn no_single_chip_is_wider_than_the_narrowest_window() {
    let ctx = egui::Context::default();
    // The narrowest the Received-image window can be is a 360 pt phone.
    let avail = 360.0 - 16.0;
    let mut worst: Option<(f32, &'static str)> = None;
    let _ = ctx
        .run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(360.0, 800.0),
                )),
                ..Default::default()
            },
            |ui| {
                for label in [
                    "Save image as…",
                    "Re-upload",
                    "Share picture…",
                    "Delete…",
                    "Delete — sure?",
                    "Send picture & open mail",
                ] {
                    let w = crate::chrome::chip_width(ui, label, None);
                    if worst.is_none_or(|(prev, _)| w > prev) {
                        worst = Some((w, label));
                    }
                }
            },
        )
        .drop_without_applying_deltas();
    let (w, label) = worst.expect("measured something");
    assert!(
        w <= avail,
        "{label:?} is {w:.1} pt wide and the window has {avail:.1}: a chip that wide \
         cannot be pressed, which is the fault this row had"
    );
}
