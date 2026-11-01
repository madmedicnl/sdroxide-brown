# Instructions for agents working in this repository

## Every piece of interface text is translated

The interface speaks English, French, Spanish, German, Dutch, Italian, Brazilian Portuguese, Russian and Turkish. **Any change that
adds or edits text a person reads on screen must add or update the message in
all nine languages in the same change.** That covers labels, buttons, hover
text, window titles, notes, status lines, error messages shown to the operator
and the names of values (`label()` of an enum drawn on screen).

How:

1. Never write display text as a string literal in `crates/sdroxide-ui`.
   Use `tr!("some-id")` (`use crate::i18n::tr;`), or `tr!("some-id", n = 3)`
   for a message with `{ $n }` placeables, or `tr!("some-id", "hint")` for an
   attribute. `tr!` returns a `String`; pass `&tr!(..)` where `&str` is wanted.
2. Add the message to every language with the helper (it locks the files, so it
   is safe to run in parallel):

   ```bash
   python3 tools/i18n_add.py <<'JSON'
   {"id": "area-thing", "en": "Thing", "fr": "Chose", "es": "Cosa", "de": "Ding", "nl": "Ding", "it": "Cosa", "pt-BR": "Coisa", "ru": "Вещь", "tr": "Şey"}
   JSON
   ```

   Ids are `area-words-of-the-text` in lower case with hyphens. Keep capitals
   where the English has them (captions in capitals stay in capitals).
   Keep `{ $name }` placeables as they are. Do not translate ids, file names,
   protocol names, mode names (FT8, SSTV…) or units.
3. For a value an enum shows on screen, add the arm in
   `crates/sdroxide-ui/src/i18n/labels.rs` (a `match` that must stay
   exhaustive) rather than calling `label()` from the interface.
4. `const`/`static` items cannot call `tr!`. Keep the English identity there and
   translate when drawing (see `tab_label` in `app/settings/mod.rs`).
5. Run `cargo test -p sdroxide-ui --lib i18n` before pushing. It checks that
   every message exists in English, that every placeable matches, and that no
   message is unused.

Read `crates/sdroxide-ui/i18n/README.md` for the Fluent syntax and the rules for
translators. A change that adds a literal to the interface and no messages is
not finished.

## CTR on = the frequency is always in the middle

With the CTR chip lit the red line is at the centre of the waterfall on every
frame — wheel, pinch, keyboard zoom, pan, fit, mode or decoder change, every
radio, the whole bandwidth, zoom in or out, fast or slow. No drift, no jitter,
no off/on to re-centre. Modes keep their preset bandwidth; CTR only moves the
line. Touching waterfall or zoom code: keep the `ctr_zoom_sim` tests in
`widgets/spectrum_view.rs` green and add the gesture.

## Working rules

- Answer and plan in French, short and direct. No automatic approval window
  (do not call ExitPlanMode): ask questions as plain text in the chat.
- Compact the conversation as often as possible.
- If less than 10 GB of disk is free, free space first (build caches, unused
  artifacts) before building.
