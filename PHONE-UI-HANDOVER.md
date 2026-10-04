# Phone UI — handover and todo (2026-10-04)

For a fresh session or a different model. **Read this top to bottom before
touching the phone layout.** Written after context ran out mid-investigation;
nothing is half-changed on disk.

Repo `/home/druid/sdroxide`, branch **`main`** at `158714a0`, pushed. Fork-only
work; upstream is `dividebysandwich/sdroxide`. Style rules in `AGENTS.md`
(run tests on touched crates; do not rustfmt unrelated files).

---

## 0. The two things to fix, in order

1. **Find the 1.9.17 phone crash** (bounded, testable — do this first).
2. **Nested (☰) phone menu** — responsive, Phone tier only.

Both come from **fork discussion #9**
(`madmedicnl/sdroxide-brown`, "Bug - UI phone problem Persistant (in progress)
1.9.17", reporter kevin2008-01).

---

## 1. What Kevin reports (exact, with the screenshots)

- Original 2026-10-03 post: **phone layout broken on BOTH the Brown and the
  official (upstream) build** — not centred, and the **spectrum section
  missing**. He filed upstream **dividebysandwich/sdroxide#516**; maintainer
  partly answered in fork commit **`1fa0f29`**; it **persists**. Real phone
  geometry **1440×3200 (WQHD)**, Android Chrome.
  - Screenshots: `postimg.cc/zfjPKyMW` (layout), and upstream #516's own two.
- **NEW 2026-10-04 19:26: "1.9.17 => Crash and totally unusable"**
  (Brown web client). Screenshot: `postimg.cc/KvxcDZP2` (360×800). Local copy
  this session: `/tmp/opencode/d9_crash.jpg`.
  - What the shot shows: the **top chip row overlapped into a mess**
    ("WOLF", "HACK RF", "LINK", partial "P…" on top of each other), a big
    **black void** in the middle, and the lower control panel (A/B, band,
    S-meter, RATE/SPLIT/SUB/TONE, Vol, SQL, 3D/SPEC) rendered **detached** below
    it. Version reads **1.9.17**.

**Key framing the operator gave:** upstream chose **fixed-width chips**
(reserved widths); the operator prefers a **responsive** layout that adapts to
the screen. On a narrow phone those collide — fixed widths that fit a desktop
strip overflow 360 px, which is the overlap/void; and the crash is likely that
overflow reaching an arithmetic/index that goes bad.

---

## 2. THE TODO (follow this order)

### Step 1 — reproduce / locate the crash (do first)

The crash is a *bug*, separate from the layout decision. A phone over-stuff may
be fixed by the menu, but do not assume the panic disappears with it.

1. **Find every `Tier::Phone` branch and the phone sizing math:**
   - `crates/sdroxide-ui/src/app/frame.rs`: ~`204` (`tier_for`), `675`, `923`
     (`let phone = tier == Advisory…`).
   - `crates/sdroxide-ui/src/app/top_bar.rs`: `664`, `704`, `1236`, `1473`,
     `6917`, and the **`phone_tail` / `plan_phone_tail`** planner the code
     references near line ~700.
   - `crates/sdroxide-ui/src/layout.rs` (or wherever `tier_for`,
     `short_tablet`, `compact` live) — look for any **division** by a
     width/height that can be 0, any **`-`** that can go negative and then be
     cast to `usize`, any **index** into a row/chip list.
2. **The existing phone layout tests are the harness** — find them and add a
   test at **360×800** (Kevin's smallest) and **1440×3200** that lays out the
   strip. A panic in that test is the crash, reproduced without a browser.
   - Search: `grep -rn 'Tier::Phone\|phone_tail\|phone_pane\|desktop_ctx\|run_ui'
     crates/sdroxide-ui/src --include=*.rs`.
   - There are layout tests that build an egui context off-screen and run a
     frame (the `cat_rig_strip_boxes` / `the_desktop_strip_packs…` family in
     `top_bar.rs`); reuse that pattern.
3. **Compare the fork's Phone-tier chip set against what upstream draws** —
   the fork added GRID, SIG ID, ISM/ISL, ENIGMA, HFDL and the listener chips
   (`EQ`, `LOG`, `NIGHT`, `SIG ID`…). Any of these rendered at Phone tier can
   be the extra width/row that overflows. `git log v1.9.16_brown..v1.9.17_brown
   -- crates/sdroxide-ui/` to see what changed in the release that crashes.
4. **A crash that is new in 1.9.17 and not in 1.9.16 is a fork regression.**
   Bisect 1.9.16→1.9.17 UI commits if the test reproduces it.

Whole-workspace note: `sdroxide-ui` is native+wasm; a `cargo check --target
wasm32-unknown-unknown -p sdroxide-ui` must also pass (browser path).

### Step 2 — the nested ☰ phone menu (the actual fix)

Operator's instruction: **a nested menu for the phone layout** — show a few
primary controls, put the rest behind one ☰, grouped (Band · Mode · System ·
Decode windows · Extras). This is the **responsive** answer, the opposite of
upstream's fixed widths.

- **Phone tier only.** Do **not** change the desktop/tablet strips — they are
  upstream's fixed-width design and changing them costs at every merge.
- There is precedent to copy, not invent: the existing **`menu_bar` / SYS
  menu** (`top_bar.rs` `menu_bar`, `windows_controls`, `menu_chip`) and the
  phone pane splits (`frame.rs` `phone_pane`, `phone_tail`). Build the ☰ out of
  those.
- Nothing removed — every chip the fork draws must stay reachable from the
  menu. (The operator likes the extras; the answer is "reachable", not "gone".)
- Watch `PHONE_SMETER_H`, `phone_tail`/`plan_phone_tail` (the meter is what
  gives width) — the menu must coexist with that planner, or replace it.
- Test at 360×800 and 1440×3200 with the off-screen harness.

### Step 3 — hand off

Cannot be verified here: there is **no browser / display** in this environment
(egui+WASM SPA). Build it, unit-test the layout at the two sizes, then ask the
operator (and Kevin) to confirm on a real phone. Say plainly that the browser
check was not run here.

---

## 3. How to build a browser-capable server (for when a browser IS available)

The local `--server` binary has no web client unless told to. To make one:
```sh
cd crates/sdroxide-web && trunk build --release      # writes dist/
cd ../.. && cargo build --release -p sdroxide --features embed-web --bin sdroxide
```
`trunk` is installed; `wasm-pack` is not needed. Without this, `--server`
prints "Build the web client with `trunk build`… or rebuild with embed-web".

---

## 4. What is NOT the phone bug (already checked this session — do not redo)

- **FTx server decode is NOT related and NOT broken** (fork discussion #8,
  different thread). Bench-proven: the 1.9.17 `--server` engine decodes fine —
  25 batches with no client, **280 stations in ~2.5 min with a native
  `--connect` client**. So the server/decode path is sound; Kevin's FTx failure
  is **browser- or environment-specific**. See `PROJECT-BOARD.md` "Discussion
  #8 — BENCH RESULT". Do not conflate it with the phone crash.
- The phone **base** layout bug (not centred, spectrum missing) is **shared
  with upstream** (#516) — proven by Kevin running the official build. Only the
  **1.9.17 crash** may be a fork regression.

---

## 5. State of the tree (so nothing surprises you)

- `main` clean, `158714a0`, pushed. Working tree has one untracked scratch
  file: `crates/sdroxide-digi/examples/fst4w_probe.rs` (FST4W work, ignore).
- Instrumentation from the FTx bench was **reverted** (`engine.rs` clean).
- `crates/sdroxide-web/dist/` was built this session (untracked build output).
- The full UI suite passes: `cargo test --release -p sdroxide-ui --lib`
  (694 tests).

---

## 6. Source of the reports

- Fork discussions (issues are disabled): list with
  `gh api graphql -f query='{repository(owner:"madmedicnl",name:"sdroxide-brown"){discussions(first:20,orderBy:{field:UPDATED_AT,direction:DESC}){nodes{number title updatedAt}}}}'`
  — **#9 is this one**, #8 FTx, #7 SSTV, #5 Olivia, #4 settings.
- Kevin edits his messages in place and says "press F5" — re-fetch the thread
  body/comments (GraphQL `discussion(number:9){body comments…}`), not just the
  notification.
