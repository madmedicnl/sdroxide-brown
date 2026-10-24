# Autonomy journal

The running record of work done without the operator at the screen. **Newest
first**, one entry per work item, written when the item ends — a session can be
closed without warning, so nothing is held back for a tidy write-up at the end.

The rules for working this way are under "Working on your own" in
[`../../AGENTS.md`](../../AGENTS.md). The queue is [`../../TODO.md`](../../TODO.md)
and the view is [`PROJECT-BOARD.md`](PROJECT-BOARD.md). This file is not a
narrative and not a changelog: it is the state of play, so the next session (or
the operator after a night away) can start from it in one read.

Each entry, in this order:

- **Summary** — three or four plain sentences for the operator: what changed for
  him, what he can look at, what is still not done, what comes next. No crate
  names, no file paths, no jargon.
- **Done** — the item, the commits, and the commands with their numbers.
- **Not done / blocked** — what stopped, and the single question or dependency.
- **Next** — the first step of the next item.

---

## 2026-10-24 (later) — 8PSK: the instrument is sound, so the files are the fault

**Summary.** The question from the entry below is answered. Built a synthetic
8PSK of the right shape and measured it with the same instrument that reads the
test files: the synthetic scores **0.99** where the files score 0.44–0.58. So the
measurement is not what was wrong and neither is the width — **the four 8PSK test
recordings do not contain a clean differential-8PSK constellation**, and no
receiver written against them could have worked. The mode's slicer is proven on
our own synthetic; what it needs now is one genuine recording, or being shipped as
"proven synthetically" like the others.

**Done.**
- Synthetic 8PSK at 125 baud, symbols shaped by a 201-tap low-pass at 0.5 × baud
  (the α≈0 shape the corpus's own −3 dB width implies), measured by the same
  chain used on the corpus (mix, 300 Hz LPF, decimate to 8 sps, differential
  phase): **8-fold concentration 0.99**, 2-fold 0.04.
- The same chain on `samples/fldigi/8PSK125.wav`: 0.44–0.58 at every symbol
  phase, collapsing onto 0°/180° — a closed eye, not an 8PSK.
- Matched filtering (one-symbol box, raised cosine α = 0.35) does not help the
  file: 0.33 and 0.35 against 0.47 unfiltered. It is not a filter-order fault.

**Not done / blocked.** What the four files are, and whether our receiver reads
the **shaped** synthetic (the next run: write it to a WAV and feed `PskRx` with
`set_psk8(true)`; the existing test uses rectangular symbols and passes).

**Next.** Run the shaped synthetic through the real receiver, then wire 8PSK as a
mode on a branch — the wire change — with "proven synthetically, no working
recording" in the manual, exactly as ALE and CIS-11 are stated.

---

## 2026-10-24 — 8PSK: the recordings do not say what we thought

**Summary.** No change in the program. The mode that has been stuck — the
8-phase PSK family, which fldigi transmits and we cannot read — turned out to be
stuck for a reason we had wrong: the notes said those test recordings are only
30 Hz wide, which would mean they cannot carry the mode at all. They are not.
Measured properly they are exactly the width the mode should be, at a normal
level and the right speed, so the fault is not a band-limited recording. What is
still true is that their phase never settles for us, and our own decoder reads
nothing from them either. Before another receiver gets written, the next step is
to build the signal ourselves the way fldigi does and compare the two by the same
measurement — or to get one real recording of the mode.

**Done.**
- Re-measured the corpus 8PSK files (`samples/fldigi/8PSK*.wav`):
  **8PSK125 is 124.5 Hz wide at −3 dB** (BPSK125: 125.0 Hz), carrier 1806.6 Hz,
  in-band SNR ≈ 28 dB — the same level and SNR as the BPSK and QPSK files beside
  it. **Baud line 125.0 Hz**, harmonics at 250 and 374 Hz (spectrum of the phase
  derivative). The old "20–46 Hz wide" note was a faulty width measurement
  (thresholds read above the peak instead of beside it).
- Differential-phase concentration at the right rate: BPSK125 **0.87** (2-fold),
  QPSK125 **0.74** (4-fold), every 8PSK file **0.44–0.58** at any symbol phase.
- Our own BPSK decoder reads **nothing** from `8PSK125.wav` (PskRx at 125 baud over
  1650–1950 Hz), so the 0°/180° histogram is a **closed eye**, not a BPSK file.
- `docs/internal/MODE-PROGRESS.md` carries the corrected table and the two
  candidate explanations; commit `4e7abd93`.

**Not done / blocked.** What the 8PSK files actually are. Two answers would
settle it and neither is a guess this bench can make: a **synthetic 8PSK built the
way fldigi builds one**, compared against the file by the same statistic, or a
**reference recording** of the mode. Nothing is wired to the panel.

**Next.** Build the synthetic comparison (it is a Rust test next to the existing
`a_synthetic_8psk_transmission_reads_back`, which already proves the slicer on our
own waveform — the missing half is doing it under fldigi's shaping; if the
synthetic settles and the file does not, the file is the fault and the item waits
for a recording).

---

## 2026-10-24 — CIS-11: a new mode, measured from the recordings before it was built

**Summary.** There is a new mode in the DIGITAL row, **CIS-11** — the Russian
meteorological teleprinter that listeners meet between 4 and 18 MHz. Leave the
dial on a channel and it finds the two tones by itself and prints what it locked
to, with a count of how many characters passed their parity check; on a real
recording that is 121 of 123. What it cannot yet do is turn those characters into
readable letters — that layer of the protocol is not known, and the traffic on
these frequencies is encrypted anyway. It is receive only. It is on a branch as
PR #173 because it changes the protocol version, so it is waiting on your merge.

**Done.**
- The receiver (`sdroxide-dsp::cis11`): finds its own tones, tracks the character
  clock, decodes the 11-bit character. Commit `be6090ca` plus the wiring in
  `a0c56a2f`; branch `fork/cis11`; **PR #173**.
- The **framing measured, not assumed.** The two catalogues say "5 data +
  parity"; the idle word looks like 5 + 5 + 1. Scored every grouping from 4 to 16
  bits and every rotation by parity consistency: **11 bits at 85 %**, repeated
  character **13 %**, and the traffic's halves agree 5 % — so it is **ten data
  bits and one odd-parity bit**.
- **Off-air**, idle recording: tones 742/1242 Hz, **123 characters, 121
  parity-valid**; traffic: 199 characters, 123 clean. Command:
  `SDROXIDE_CIS11_SAMPLE=/tmp/opencode/cis11/idle.wav cargo test --release -p
  sdroxide-dsp --lib cis11 -- --ignored`.
- Wiring: `Mode::Cis11` (discriminant 62), `Cis11Controller`, the shared
  text-modem panel (no new UI), manual §3.30, changelog.
- Wire: **`PROTO_VERSION` 240 → 241, `VERSION_BYTE` 0x3C → 0x3D**, register entry.
- Gate: workspace check silent; types 792, digi 542, ui 961 green; rustfmt clean.
- Two pins fixed on the way: `DominoEx` and `Dtmf` were never pinned, and the
  `Mode::ALL` count test took its last discriminant from `Dtmf`.

**Not done / blocked.** The character layer: how the ten data bits divide into
letters. Needs a **clear** transmission (Russian meteo is sometimes sent in clear
M2 Cyrillic) or a reference decoder to compare against; the encrypted traffic
cannot settle it. Also unmeasured: the 400 Hz shift variant, which the receiver
searches for but no recording of it has been tested.

**Next.** PR #173 waits on the operator. Then the queue in `TODO.md`: the modes
that did not finish in 2.1.0 (8PSK's vestigial filter, the four wide PSK banks)
and ALE's recording ask.


## 2026-10-24 — The standing order for working unattended

**Summary.** Nothing in the program changed. The notes the agent works from now
say to keep going on its own when the goal and the boundaries are clear, to write
what happens in this journal as it happens, and to keep the TODO list and the
project board current. When an item is finished you will get a short plain
summary without having to ask for it.

**Done.**
- `AGENTS.md`: new section "Working on your own — the standing order" (the four
  things that make a task clear, the list of things that stop an item, the
  journal, the TODO/board updates, and the completion summary), and the header
  now names this journal.
- `docs/internal/AUTONOMY-JOURNAL.md`: this file, with its format.
- `docs/internal/README.md`: added to the file table.
- Docs only — no crate, no wire, no build. Nothing to run; `cargo check` not
  needed for a markdown change.

**Not done / blocked.** Nothing.

**Next.** The live queue is unchanged: the mode work, STANAG 4285 first —
[`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md) and
[`MODE-PROGRESS.md`](MODE-PROGRESS.md).
