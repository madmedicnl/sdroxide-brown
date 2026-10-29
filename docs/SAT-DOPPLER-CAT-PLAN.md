# SAT Doppler for CAT rigs — plan (step 1: RX Doppler only)

## Goal
Steer a CAT rig's receive VFO with the Doppler-corrected downlink frequency
during a satellite pass, instead of leaving it on the nominal downlink.

## Constraints
- No hardware on the bench (FT-991A, IC-7100). All engine integration
  needs live testing by SP9TKN.
- Step 1 = RX Doppler only (no uplink, no split, no TX changes).
- Rate-limited writes (never during TX).
- Rig's echo must be recognised as ours, not as a dial move.

## What exists
- `doppler_rx_hz()`, `doppler_tx_hz()` — already computed in `poll_sat_track`
- `corrections_active = lock.cfg.doppler && !self.audio_mode` — false for CAT
- `update_tuning()` audio_mode branch sets VFO to `state.active_freq_hz()` (nominal)
- `note_frequency_taken(hz)` records `good_vfo_hz` for echo recognition
- CAT `set_freq` sets the selected VFO's frequency
- Yaesu `FA…;` / `FB…;`, Icom `0x05` (selected VFO)

## Step 1 changes needed

### 1. `start_sat_lock()` (engine.rs ~14307)
- Remove the "CAT rig: satellite tracking runs, but Doppler correction needs
  an IQ front end" notice, or change it to note CAT Doppler is active.
- For CAT rigs, set `corrections_active = lock.cfg.doppler` (remove `&& !self.audio_mode`).

### 2. `poll_sat_track()` (engine.rs ~14475)
- Change `t.corrections_active = lock.cfg.doppler && !self.audio_mode` to
  `t.corrections_active = lock.cfg.doppler`.
- This makes the Doppler correction computed even for CAT rigs.

### 3. `update_tuning()` audio_mode branch (engine.rs ~18145)
- When sat lock is active and corrections_active:
  - Compute `corrected = state.active_freq_hz() + sat_rx_doppler_hz()`
  - Send `corrected` to the rig via CAT (set_freq on VFO A)
  - Set `good_vfo_hz = corrected` (so the rig's echo is recognised as ours)
  - Rate-limit: only send if `|corrected - last_sent| > threshold`
  - Never during TX: check `!self.tx_active`
- Track `last_sat_cat_freq: f64` and `last_sat_cat_time: Instant` for rate limiting.

### 4. Thresholds
- FM: 1 kHz (1000 Hz)
- SSB/CW: 10 Hz (or tens of Hz)
- Rate limit: at most one write per 2 seconds (to avoid flooding CAT)

### 5. Echo recognition
- When the rig reports back a frequency, compare against `good_vfo_hz`
  (which is now the corrected frequency). If within tolerance (e.g. 5 Hz),
  treat as "ours" — do not adopt as a dial move.
- The tolerance needs to account for BCD rounding on Yaesu.

### 6. `tx_begin()` / `tx_end()`
- No changes for step 1 (uplink pre-correction is step 2).
- The rig stays on the corrected downlink during TX (no frequency change).

## Open questions for SP9TKN
- VFO selection: Yaesu `FA…;` sets VFO A regardless of selected VFO. Icom `0x05`
  sets the selected VFO. Should I select VFO A first on Icom, or assume VFO A
  is already selected?
- Rate limit: is 2 seconds between writes OK, or should it be faster/slower?
- Threshold: 1 kHz for FM, 10 Hz for SSB/CW — reasonable?

## Files to touch
- crates/sdroxide-radio/src/engine.rs (start_sat_lock, poll_sat_track, update_tuning)
- crates/sdroxide-cat/src/yaesu.rs (already has set_freq_vfo_b, split frames)
- crates/sdroxide-cat/src/civ.rs (already has select_vfo_a/b, split on/off)
- crates/sdroxide-types/src/satlock.rs (maybe add cat_doppler flag?)

## Not in scope (step 2+)
- Uplink pre-correction
- VFO B / split
- Mode and CTCSS on VFO B
- Unlock restore
