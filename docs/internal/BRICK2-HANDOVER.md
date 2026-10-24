# Brick2 — the two reports, and what was taken from upstream

The operator's instruction, 2026-10-24 (fork discussion #169, "Brick 2 for
jarvis"): work fork issues **#133** and **#132**, look at what upstream has for
"brick", and make a PR. It is the second Brick2 round; the first ended with the
Swap I/Q switch for SDR sources (fork PR #160).

Nothing here has been on a Brick2. There is none on this bench — the two radios
are an RSP1 and a CRT SS9900v — so every claim below is a code or test claim and
is labelled as one.

## What upstream had

Two commits, both taken by hand (never `git merge upstream/main`):

| upstream | what | why it matters here |
|---|---|---|
| `31ebe99e` (#659) | HPSDR Protocol 2: pace the DUC stream at one datagram per 1250 µs | our tree sent the whole transmit ring as fast as the socket took it; the jitter cushion lived in the radio's TX FIFO, which on a Brick2 is shallower, so a burst dropped and the over came out choppy |
| `c8c4c274` (#578) | recognise a Hermes-Lite 2 in Hermes-emulation gateware by its HL2-only discovery fields | a Brick2 is a Hermes-Lite in a box; reported as a Hermes, its PA is never switched on and its gain register never written — it keys and makes no power at the antenna jack |

The upstream issue search also has **#399** (open — Brick2 microphone gain, the
board's own notes say a Hermes-Lite works) and **#440** (closed, "Fixed!" —
garbled TX audio, which the pacing above is the fix for). Neither is a report
this fork has had.

## What landed in the fork

- `crates/sdroxide-hpsdr/src/protocol2.rs`: `DUC_INTERVAL`, the keyed/idle read
  timeouts, the paced send, the stale-tail drain on unkey, and the resync after
  a stall. Guard: `crates/sdroxide-hpsdr/tests/duc_pacing.rs`, whose first
  assertion is that 100 ms of I/Q does not leave in a burst — verified failing
  against the old loop (`100 ms of I/Q went out in 705 µs`).
- `crates/sdroxide-hpsdr/src/discovery.rs`: `looks_like_hl2` and the
  "Hermes-Lite 2 (Hermes emulation)" name. Guard:
  `parse_p1_response_hl2_in_hermes_emulation`, verified failing with the branch
  forced off (the board comes back named "Hermes (ANAN-10/10E/100/100B)").

Both are wire-neutral: no `PROTO_VERSION` bump, no new field, no `Command`.

Plus the one-switch-per-radio fix below, which is the generic `swap_iq` folded
into the same rule: `Backend::inverts_iq_itself`, `RadioConfig::swaps_iq`, the
engine reading it, and the Radio settings page drawing the checkbox only where
it is the switch that does something. Also wire-neutral.

## The two issues, and where each stands

**#133 "Must set USB to receive LSB on 40 meters" — a switch, plus one fault on the way.**
The Brick2's I/Q is the other way round from the Hermes-Lite default, so LSB is
heard as USB and the spectrum is mirrored about the dial. Settings → Radio →
HPSDR → **Invert spectrum** ("Swap I/Q") is the cure and has been there since
the Hermes-Lite work.

The fault: the generic **Swap I/Q** added in fork PR #160 sits one level up, on
the engine's input, and on an HPSDR radio it ran *on top of* the board's own
inversion — two conjugations cancel exactly, so an operator following "tick Swap
I/Q" from #133 saw nothing happen. The same was true of a CAT rig on a sound card
(`CatConfig::invert_spectrum`) and of SmartSDR (`SmartSdrConfig::swap_iq`).

Fixed by making it one switch per radio: `Backend::inverts_iq_itself` names the
three that conjugate in their own layer, `RadioConfig::swaps_iq` is the one
place that resolves the stored flag against it, and both the engine and the
Radio settings page read that. The checkbox is drawn only where it is the switch
that does something; where it is not, the row says where the other one is, so
nothing silently disappears. The stored flag is left alone, so a `radio.json`
that named a SoapySDR before and an HPSDR after keeps its answer for the next one.

Guard: `a_radio_that_inverts_its_own_iq_is_not_inverted_twice`, verified failing
with the resolver reduced to `self.swap_iq`.

**#132 "Wrong frequency display" — one datum still missing, already asked.**
Thetis 7.183 vs this program 7.223 is 40 kHz, with the spectrum going unstable
while a MIDI knob turns. The comment on the issue asks the one question that
splits it: is the number the **dial readout** or the **spectrum centre**? A
mirrored sideband (#133) puts a station on the wrong side of the dial and reads
as a frequency error; a stale or mis-scaled axis reads as one too. Nothing in
the HPSDR frequency path is shell-verified here, and guessing at a 40 kHz offset
without the reporter's answer is how a correct reading gets "fixed".

If it does turn out to be the axis, the thing to measure next is the **rate the
board actually streams** against the rate we asked for: the panadapter's
frequency axis is built from `HpsdrConfig::sample_rate_hz`, and a board that
quietly answers with a lower rate would put every signal out by the ratio.
