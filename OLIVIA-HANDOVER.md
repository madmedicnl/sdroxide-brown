# Olivia — status: both directions work; the one check left is on the air

**This is a status note, not a handover.** The work is on `main`
(`cd668ab5` receive, `5464e7a9` transmit); there is no branch and no WIP. Kept
because the polarity finding is not re-derivable from the code, and because the
remaining gap is the kind that looks finished.

## What is done

- **Receive** is confirmed **off the air** against a real recording, and asserts
  the *content* rather than "something printable came out":
  ```
  SDROXIDE_OLIVIA_SAMPLE=/tmp/opencode/cq_swnet.wav \
    cargo test -p sdroxide-dsp --release --lib -- --ignored --nocapture \
    an_off_air_capture_decodes
  → "CQ  SouthWest NET  CQ SouthWest NET  CQ SouthWest NET
     de  G7LEE  G7LEE  G7LEE"
  ```
  313 s. That content assertion is what caught the Walsh bug; the old version
  passed on garbage, which is how it shipped looking ready.
- **Transmit** round-trips through our own receiver. `loopback_32_1000`,
  `loopback_8_250` and `every_character_reads_back_unchanged` all pass, and all
  three fail on the previous polarity with the same `@@@@@`.
- The scrambler, the interleave, the (64,7) Walsh codeword, the Gray tone
  assignment, tone spacing = symbol rate and the 64-symbol block are all
  **fldigi's**, taken from `src/include/jalocha/pj_mfsk.h` in `w1hkj/fldigi`.

## The polarity, and why it looks like a bug

fldigi's `EncodeBlock` sets the symbol bit where its codeword is **negative**
(`if (FHT_Buffer[TimeBit] < 0)`), and its `SoftDecode` votes **negative** for a
set bit. Those two are an exact pair, and a standalone copy of its loops
round-trips under them.

**Our receiver votes positive** — and it is the one that reads the recording
above correctly. The conventions are exact negatives, so they cannot both be the
air.

A C++ dumper of fldigi's own code settled which is which:
`/tmp/opencode/ref_dump.cpp` prints every scrambled codeword plane and the
`OutputBlock`. With `f < 0` our output was **byte-for-byte fldigi's
`OutputBlock`**; with `f > 0` it is its exact complement. The first fails every
round-trip test, the second passes them all, and the off-air decode is identical
either way — the receiver is untouched by the choice.

So `encode_block` sets the bit where the codeword is **positive**, and says so
where the bit is decided. **Do not "fix" that back to `< 0.0`.** An earlier
version of the handover blamed the scramble order or the bit placement for the
same symptom; it was this sign, and no amount of reading the reference harder
would have found it — the answer was in the one artifact only an air recording
could supply.

## What is NOT done

**Nothing has ever been transmitted to another station.** Item 2 is now closed
(see below). In the order the rest will bite:

1. ~~**No sync tones.**~~ **START TONES NOW SENT; the tail is still missing.**
   fldigi brackets every transmission with a pair of tones at the band edges,
   and that is how it finds a frame at all. Our receiver never needed them
   because its block-grid lock free-runs — which is exactly why the loopbacks
   passed while a real decoder may never lock, and why the omission had nothing
   to do with polarity.
   - **What is sent now**, reproduced from `olivia::send_tones()`
     (`src/olivia/olivia.cxx`): `TONE_DURATION = SCBLOCKSIZE * 16` = **8192**
     samples in **four** quarters of `SR4` = 2048, alternating low/high/low/high
     at `txbasefreq ∓ bandwidth/2`. `SCBLOCKSIZE` is 512
     (`src/include/sound.h`); the quarters come from `src/include/olivia.h`, and
     **not** from the sample rate — so the burst is 8192 samples whatever the
     audio rate is.
   - fldigi gates it on `olivia_start_tones`, **default true** ("Send
     start/stop tones"), so a default transmission carries them.
   - **The edges sit half a tone spacing outside the outermost data tone.** Our
     bank runs `base_hz … base_hz + (tones-1)*spacing`, so the edges are
     `base_hz - spacing/2` and `base_hz + tones*spacing - spacing/2`. For
     32 tones at 1500 Hz centre / 1000 Hz bandwidth that is exactly
     **1000 Hz and 2000 Hz**. `Geom::edge_hz` is the one place this is decided.
   - Two details that are not cosmetic: fldigi's `ampshape` ramps `SR4/8` = 256
     samples at **both ends of every quarter** (a raised cosine), and the tone
     phase is **zeroed at the start of each burst** (`preamblephase = 0`) but
     carried *across* quarters. Drop the ramp and each of the four tone changes
     is a click — three splatter bursts inside one frame.
   - fldigi then puts **exactly one idle character** in after the tones ("the
     Olivia Transmitter class requires at least character"). It is pushed with
     **no source index**, so it is a frame to lock to and never a character of
     the message — `total_chars`/`sent_chars` do not see it.
   - Pinned by `the_start_tones_bracket_the_tone_bank` (placement, read back by
     zero-crossing count), `each_start_tone_quarter_is_ramped_at_both_ends`,
     `the_start_tones_are_phase_continuous_across_a_tone_change`,
     `the_idle_character_after_the_start_tones_is_not_sent_text` and
     `a_clear_re_arms_the_start_tones`. Each was verified to **fail** against the
     broken version — moving the tones inside the bank fails the first, deleting
     the ramp fails the second and the third.
   - **What is still missing: the tail.** fldigi sends the same `send_tones()`
     again at the end (`postamblesent`) and then `SCBLOCKSIZE` samples of
     silence. **It cannot simply be appended inside `OliviaTx`**: the caller
     stops asking for audio the moment `sent_chars` reaches `total_chars`
     (`text_modem.rs`), so tones written into the block buffer would never be
     drained. This needs a change to the transmit-active contract in the digi
     engine — deliberately not smuggled in alongside the start tones.
3. **No frequency search**, where fldigi searches ±8 tone spacings — its
   `SyncMargin = 8` (`src/olivia/olivia.cxx`), giving
   `FreqOffsets = 2 * SyncMargin + 1` = 17 candidate offsets (`pj_mfsk.h`).
2. ~~**No on-air proof of the polarity.**~~ **CLOSED 2026-10-05** — fork
   discussion #5: kevin2008-01 sent a capture whose text MultiPSK had already
   shown, and our decoder reads it on his own radio at 32/1000
   (`"Wikipedia, the free encyclopedia that anyone can edit"`). See AGENTS.md §3.
   The polarity was already settled from our side by `cq_swnet.wav`; this
   confirms it from the air, from somebody with no stake in our reasoning.
   **Keep the distinction: our *receiver* is proven, our *transmitter* is not.**
   Nobody has yet decoded our transmission with fldigi, and item 1's tail is
   part of what that would need.

The mode's doc, the Olivia settings row and the mode-chip hover all say this in
the operator's words rather than promising an answer that may not come.

## The samples and the harnesses

- **`/tmp/opencode/cq_swnet.wav`** — the one that decodes. 8 kHz mono, 50.9 s,
  Olivia 16/500, comb 1243.75 + k·31.25 Hz, 256 samples/symbol, from the Avalon
  SW Net article's own `<source>` tags and **tested there in fldigi**.
  https://www.avalonarc.org.uk/2020/12-14-sw-data-net.html
- `/tmp/opencode/mx0ioa.wav` — 11.6 s, known text `MX0IOA`. Too short to be a gate.
- `/tmp/opencode/ref_dump.cpp` — fldigi's `EncodeBlock` + `ScramblingCode` as a
  standalone dumper, every scrambled plane and the `OutputBlock`, byte for byte.
- `/tmp/opencode/ref_probe.cpp` — the same loops plus `SoftDecode`: proves
  fldigi round-trips its own air under "set bit → negative", and that
  `FHT(IFHT(δ)) = nδ` is positive identity.
- `/tmp/opencode/olivia_polarity.py` — Python twin of the whole chain (transform,
  scrambler, interleave, Gray), for checking a convention in seconds.
- `/tmp/opencode/fl/fldigi-master/` — the whole tree, for `pj_mfsk.h` and
  `pj_gray.h`. GitLab is Cloudflare-blocked and SourceForge serves HTML;
  `raw.githubusercontent.com/w1hkj/fldigi/master/...` works and Debian ships the
  tarball at `deb.debian.org/debian/pool/main/f/fldigi/`.

Do **not** use `~/Downloads/kiwi-farnham_…wav`: weak, ~2.4 kHz off, and it does
not decode in fldigi either.