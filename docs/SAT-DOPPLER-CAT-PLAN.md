
## SP9TKN's answers (2026-10-29)

1. **VFO selection (Icom):** select VFO A explicitly, once at lock (VFO mode, then
   `0x07 00`). Not before every write — the rig may have been left on VFO B.
2. **Rate limit:** no fixed interval. 1 Hz resolution (both rigs accept it over
   CAT). Pace writes by link acknowledgment: send next only after rig acknowledges
   (Icom `FB` reply), or after a short gap on Yaesu (no ack). Leave room for
   the regular CAT poll. One mechanism for all modes, not separate FM/SSB thresholds.
3. **Echo recognition:** tight tolerance (1 Hz writes → exact match expected).
4. **TX in step 1:** existing key-down behaviour stays (VFO A borrowed for
   uplink, then back). No TX changes in step 1. SP9TKN will avoid TX testing.
5. **TX in step 2 (future):** HRD-style — VFO B on corrected uplink, split on,
   key-down is just PTT.

### Updated thresholds
- Resolution: 1 Hz (not 1 kHz / 10 Hz)
- Rate limit: by acknowledgment, not fixed timer
- No separate FM/SSB thresholds

## CAT-thread architectuur (step 1: RX Doppler)

### Huidige architectuur
```
Engine (audio_mode)          CatHandle (channels)          CAT thread
  │                              │                            │
  │ source.set_center_hz(dial)   │                            │
  │ (sound card only)            │                            │
  │                              │                            │
  │                              │  cmd_tx.send(Freq(hz))     │
  │                              │ ──────────────────────►    │
  │                              │                            │ process Freq → protocol.set_freq(hz)
  │                              │                            │ → rig VFO set
  │                              │                            │
  │                              │  event_rx ◄──────────────── │
  │                              │  (rig reports freq back)   │
```

### Wat er bij moet voor SAT Doppler
```
Engine                          CatHandle                      CAT thread
  │                              │                            │
  │ (sat lock active)            │                            │
  │ compute corrected = dial +   │                            │
  │   doppler_rx_hz              │                            │
  │                              │                            │
  │ [rate limit check]           │                            │
  │ [never during TX]            │                            │
  │                              │                            │
  │ cat.sat_set_freq_vfo_a(      │                            │
  │     corrected)                │                            │
  │                              │  cmd_tx.send(             │
  │                              │    CatCmd::SatFreqVfoA(    │
  │                              │      corrected))           │
  │                              │ ──────────────────────►    │
  │                              │                            │ process → protocol.set_freq(corrected)
  │                              │                            │ (Yaesu: FA<corrected>;)
  │                              │                            │ (Icom: 0x07 00, 0x05, 0x07 00)
  │                              │                            │ → rig VFO A set
  │                              │                            │
  │                              │  event_rx ◄──────────────── │
  │                              │  (rig reports corrected)   │
  │                              │                            │
  │ [echo recognition]           │                            │
  │ good_vfo_hz = corrected      │                            │
  │ → rig's echo matches ours    │                            │
  │ → not adopted as dial move   │                            │
```

### Nieuwe CatCmd varianten
- `SatFreqVfoA(f64)` — set VFO A to corrected frequency (step 1)
- `SatFreqVfoB(f64)` — set VFO B to corrected uplink (step 2)
- `SplitOn` / `SplitOff` — rig split control (step 2)

### Rate-limiting (SP9TKN's suggestie)
- No fixed timer. Pace by acknowledgment:
  - Icom: send next only after rig acknowledges (FB reply)
  - Yaesu: send next after a short gap (no ack)
- Leave room for regular CAT poll
- 1 Hz resolution (both rigs accept it over CAT)

### Echo recognition
- `good_vfo_hz` = corrected frequency (not nominal dial)
- When rig reports back, compare against `good_vfo_hz`
- If within tolerance (1-2 Hz), treat as "ours" → do not adopt as dial move
- Account for BCD rounding on Yaesu

### Engine changes
1. `start_sat_lock()`: remove "CAT rig: Doppler needs IQ" notice. For CAT rigs
   with cfg.doppler, note that Doppler will be applied via CAT.
2. `poll_sat_track()`: set `corrections_active = lock.cfg.doppler` (remove
   `&& !self.audio_mode`).
3. `update_tuning()` audio_mode branch: when sat lock active and corrections_active,
   compute corrected freq, send via cat_handle (rate-limited, never during TX).
4. Track `last_sat_cat_freq: f64` and `last_sat_cat_time: Instant` for rate limiting.
5. `tx_begin()` / `tx_end()`: no changes in step 1 (existing behavior stays).

### Testing without hardware
- CAT command builders: ✅ tested (Yaesu + Icom frames)
- Rate-limiting logic: can be unit-tested as pure function
- Echo recognition: needs hardware
- Engine integration: needs hardware

### Step 2b/2c: VFO B + split (implemented 2026-10-29)
- VFO B set to corrected uplink at lock start and updated (rate-limited, > 1 Hz)
- Split enabled at lock start (`FT3;` / `0x0F 01`)
- Split disabled on unlock (`FT2;` / `0x0F 00`)
- Mode and CTCSS on VFO B: NOT yet implemented (requires VFO selection +
  mode set + tone set sequence). Rig retains last mode/tone on VFO B.
