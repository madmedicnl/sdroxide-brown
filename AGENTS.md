# Agent notes — SDR Oxide Brown, the CB and SWL fork

Read this first. It is the live brief: who is who, what must not be touched, the
house rules, **how to keep working when the operator walks away**, the wire, the
gate, releases, and where the work stands. Where a number here can be
re-measured, re-measure it rather than trusting it.

The detail behind any rule — the measurements, the wrong turns, the session that
found it — is in [`docs/internal/SESSIONS.md`](docs/internal/SESSIONS.md), kept
verbatim as a record. Handovers for anything still live are in
[`docs/internal/`](docs/internal/README.md). The day's queue is
[`TODO.md`](TODO.md), the three-column view is
[`docs/internal/PROJECT-BOARD.md`](docs/internal/PROJECT-BOARD.md), and the
running record of unattended work is
[`docs/internal/AUTONOMY-JOURNAL.md`](docs/internal/AUTONOMY-JOURNAL.md).

## Who is who

- **madmedicnl** — the operator and maintainer of this fork. Amateur callsign
  **19DCG373**; "mad medic" on the forum. He decides what is built and when a
  release is cut, and he merges. Address him by name, and **not** as F4NRM.
- **F4NRM (Kevin, historically `kevin2008-01`)** — the collaborator and the
  fork's most valuable tester: three radios of his own (PlutoSDR, HackRF,
  RTL-SDR) on one station behind Caddy, Chrome on Android as an installed PWA
  and a desktop browser, plus an RSP1A on a Mac. He opens pull requests here
  from Claude sessions, and he owns whole areas of the program — see the
  forbidden list below. He answers precisely and attaches screenshots; believe
  him over a recollection, and ask him one thing at a time.
- **Upstream** is [`dividebysandwich/sdroxide`](https://github.com/dividebysandwich/sdroxide),
  the original program. Everything here is upstream's work plus this fork's; the
  relationship is described under "Fork first, upstream read-only".

Two public discussions carry the working agreement and are worth reading once:
[#73](https://github.com/madmedicnl/sdroxide-brown/discussions/73) (the rules,
and why upstream is never merged) and
[#168](https://github.com/madmedicnl/sdroxide-brown/discussions/168) (how the
forum is run: **IDEA** / **FEED** / **REQ** in the subject, bugs go to Issues
with the version from the frequency window).

## ⛔ F4NRM's list — his to change, and a PR to touch

The operator's list, verbatim (discussion
[#155](https://github.com/madmedicnl/sdroxide-brown/discussions/155)):

> Décoder: SSTV, DAB, NAVTEX, APRS, WFM, DCS, OLIVIA, RTTY, WEEFAX, FT8, RDS
> Radios: RSP1, HACK RF, PLUTO

**RSP1 is not on it any more**, and the operator struck it off himself in the same
discussion (2026-10-24): *"RSP1 is not possible my friend, I have an RSP1 and if I
have issues I'm going to have to do something about it.. it's the only wideband
receiver I have."* It is the bench's receiver and his only wideband one, so RSP1
work is ours and is not something to ask permission for. The list as it stands is
therefore the decoders **SSTV, DAB, NAVTEX, APRS, WFM, DSC, Olivia, RTTY, WEFAX,
FT8, RDS** and the **HackRF and PlutoSDR** backends.

**What "his" means is a workflow, not a wall.** He is working in them, and a
change nobody asked for is how a mode a whole radio group has confirmed stops
working — so no refactor, no "improvement", and no re-opening their journal
entries as work items. But a *report* against one is a new report about a
specific fault: start from the report and reproduce it, not from a tidy-up.

And anything that does have to touch one — a fault that has to be fixed, a wire
change that reaches it, a panel of his that a shared change drags in — goes up
**as a PR, never straight to `main`**, which is the operator's own instruction in
that discussion: *"put any wire change or item that touches these to put up as a
PR and not implement them directly without you having the chance to do look at
them."* Branch, tests, PR, and he reads it before it lands. The same shape as the
wire rule below, for the same reason: the number, the mode or the panel is one
counter between two people, and a diff is how one of them sees the other's claim.

Two of these are also confirmed on the air by his radio group and must stay
exactly as they are: **WEFAX, DAB, SSTV and DSC**.

**Detachable windows are his too** (discussion
[#166](https://github.com/madmedicnl/sdroxide-brown/discussions/166)): he is
making as many panels detachable/stackable as he can. So when a change of ours
touches a panel, **make that panel detachable or stackable and add it to the
Detachable page in Settings**, as his own PRs do — do not leave a new panel
docked-only.

## What this repository is

A fork of `sdroxide` tuned for the **11 m citizens band**, **shortwave
listening** and **decoding**. All three live in one program: the CB band used to
the full — voice and the digital modes, a two-way band with WSJT-CB
interoperability and channel plans — plus the listener's tools: the broadcast
schedule, the SWL log, time-shift replay, scheduled recordings, ECSS, the receive
tone, scan bands, the wideband decoders. CB is a first-class citizen, not a
receive-only extra, and CB and amateur radio are neighbours on the same spectrum
rather than rivals.

The listener-only fork that used to carry this (`madmedicnl/sdroxide-swl`) is
retired and merged in. Everything is on `main`; `origin` is
`madmedicnl/sdroxide-brown`, and it is the only repository to push to.

The build calls itself **`SDR Oxide Brown`** where a person reads it (native
window title, solar viewport, web `<title>`, Settings → General header,
`--version`), release tags carry `_brown`, and the config directory is
`~/.config/sdroxide-brown` (`org.sdroxide.sdroxide-brown`) so the two builds can
run side by side without eating each other's settings. `CARGO_PKG_VERSION` stays
plain (`2.1.0`) because `2.1.0_brown` is not valid semver and other people's
software parses it. The executable name, the macOS bundle id and the release
asset names stay shared with upstream on purpose; the Windows MSI has its own
`UpgradeCode` because sharing one made the two installs replace each other.
Do not re-introduce "fork", a bare lowercase `sdroxide`, or the old "CB/SWL"
label into a user-facing title — `FLAVOR` is the one string.

## Fork first, upstream read-only

- **Work happens here.** Offering something upstream is goodwill, never the
  reason for a decision. When a choice is between a ham workflow and a CB or
  listening one, **this fork takes the CB/listening one**.
- ⛔ **Never merge `upstream/main` into this fork.** Upstream is *behind* us
  (measured 2026-10-12: fork 2.0.9 / `PROTO_VERSION` 205, upstream 1.6.10 / 179,
  983 commits ahead and 68 "behind" — i.e. we have moved past them). A trial
  merge produced 47 conflicted files and would have un-done fork work: dropped
  the vendored `mfsk-core` patch (the FT8 subtract panic that kills the decode
  worker), rewound `version` and rewound 26 released wire versions.
- **Reading upstream is safe and encouraged**: `git log main..upstream/main`,
  `git show <sha>`, `git diff`. **Taking a specific commit is done by hand**, one
  commit at a time, into our tree — never by merging the branch. The most recent
  examples are in [`docs/internal/UPSTREAM-PICKS.md`](docs/internal/UPSTREAM-PICKS.md)
  and [`docs/internal/BRICK2-HANDOVER.md`](docs/internal/BRICK2-HANDOVER.md).
- If a change is **derived from an upstream issue** (a fix or feature someone
  asked for there), it may also be offered upstream as its own branch off
  `upstream/main`, one idea per PR, clean and mergeable. Purely fork-shaped work
  (CB, SWL, this station's needs) stays here.
- Two divergences are deliberate and settled with evidence. **Do not "fix"
  either by taking upstream's side**: Olivia's polarity (this fork votes
  *positive*, which is what reads a real off-air recording; upstream's negative
  vote makes every character decode as its bit-6 twin) and the spoken alerts
  (this fork speaks whether or not the alert window is in front).
- Occasionally **check upstream's issues** for anything that touches this fork.
  Do not triage or reply to their queue, and do not re-ping a PR of ours that has
  gone quiet — silence is not a prompt.

## House rules

These are the ones that have cost time or shipped a wrong answer. They are not
style preferences.

- **Say it short.** Release notes: one line per change under `### Fixed` /
  `### Added` / `### Changed` / `### Needs feedback`, then stop — the notes sit
  above the download assets, and `release.yml` hard-fails a release whose notes
  run past 60 lines. The README front page is an elevator ride. A reply to a
  user is **prose**, not a document: no headings, no bulleted essays, no bold on
  every other phrase. Polite, personal, direct. The *why* belongs in the commit
  message, where a reader who wants it will look.
- **Ask a reporter one thing, not three.** A report is usually enough to work
  from; ask the question that decides the fix and leave the rest until it is
  answered.
- **Be kind, polite and to the point with contributors — especially the ones
  supplying data.** A person who records a test tape, runs a measurement or sends
  a file is giving us their own time and bench, and is the reason a mode like
  STANAG 4285 can be finished at all (discussion #165 is the model: one clear
  thank-you, what the file settled, what it did not, and at most one request).
  No defensiveness, no lecture, no wall of detail.
- **A measurement that disagrees with a report is the first suspect.** Before
  writing a fix, ask in this order and stop at the first "no": is it *visible*
  (the reporter's words taken literally)? does the instrument measure *the thing
  reported*? does anything already in the tree have a **number**, and does it
  disagree? is something else **fighting** the fix? When a correct fix changes
  nothing, a second cause is in play — find it with the one-click experiment,
  not a third implementation.
- **A passing test is evidence about the test.** Every version of this has
  shipped green: nine tests on a rendered QSL card with three typed fields
  missing; a sweep whose tolerance existed only to clear the fault it was
  supposed to catch. When a fault survives a test, **the test and the tolerance
  are suspects before the code**. A tolerance added to accommodate a known fault
  is the fault, written down.
- **A fix carries a test verified to fail on the unfixed code.** Remove the fix,
  watch it fail, put it back. For every test, every time.
- **For anything drawn rather than computed, render it and look at it.** Numbers
  prove where the ink is; only looking proves it is there.
- **Never claim something is fixed or shipped when it is not.** Say what is in a
  build and what is not. "Needs feedback" is a request, not a disclaimer; there
  is no "Not fixed" heading — everything in a build has passed its tests.
- **A control must never silently do nothing.** Name *which* thing is off, say
  why a number is not the one expected, and put the fix next to the symptom.
  Assume a beginner and never leave them guessing: the point is to let them
  explore without misreading the screen.
- **No paid service, no API key, no account. Ever.** Data comes from a dataset we
  may ship (public domain or free licence), a committed script, or nothing. The
  program must **work offline**: receive, transmit, decoders, maps and the log
  are local, and everything that reaches out (spots, TLEs, space weather, the
  schedule, callbook, log uploads, the update check) is an extra, off until
  asked, that degrades quietly.
- **`git checkout --` on a file being edited destroys work** — undo a scratch
  patch with the patch. And **verify an edit landed**: a script anchored on a
  string from another file silently does nothing.
- **`rg -n`, never `rg -rn`** (`-r` is replace and rewrites what it prints).
- **Format what you touch.** No repo-wide `cargo fmt`: 26 `fork-modified` files
  are not rustfmt-clean because upstream keeps editing them unformatted, and a
  hunk we fix now is a hunk that can conflict at the next pick. Fork-only files
  are swept on arrival. Measure in place — run `rustfmt` with the path still
  under the repo root, or `--config-path`, because `rustfmt.toml`'s
  `use_small_heuristics = "Max"` is otherwise not found and a scratch copy reads
  as dirty.
- **Commit messages**: a short imperative subject, then the why; say plainly what
  was *not* tested when it could not be.
- **Do not touch `vendor/`** except to update a submodule, and **native-only
  crates** (drm, nrsc5, faad2, the USB drivers, hpsdr) must never become
  dependencies of a wasm-targeted crate.
- **Dependabot's `tract-onnx` / `tract-nnef` advisories are dismissed as not
  used** — only the model embedded in the binary is ever parsed, never an
  operator- or network-supplied one. If a `cargo-audit`/`cargo-deny` config is
  ever added, those ids go in its ignore list with that note.

## Working on your own — the standing order

The operator does not want to sit in front of the screen holding the session's
hand. **When the end goal is clear and the boundaries are set, the default is to
keep going, not to ask.** He asked for this on 2026-10-24, in as many words: he
is tired of saying "go on".

**A task is clear when all four can be written down.** Take them from this file,
`TODO.md`, the handover or the tree; if one of them genuinely cannot be
established, ask that one question and then work — do not ask for the rest.

- the **end goal**, with a definition of done that can be checked: a passing
  test, a real capture decoding, the gate run, a screenshot that shows the ink.
  "Improve X" is not a definition of done.
- the **boundaries**: the crates and files, and which rules above apply — F4NRM's
  list, the wire, no UI, no release.
- **how you will know it failed**, not only how you will know it passed.
- **what needs the bench**: anything off the air or on a radio is a stop. It is
  not a reason to guess, and it is not a reason to stop the session either.

**Then work the queue end to end.** Take the next item in `TODO.md`, finish it,
record it, take the next one. A plan that already sits inside the boundaries
does not need approval — say in one line what you are doing and do it.

**These stop the item; nothing else does.** Write down why, and move to the next
item — a blocked item is not a blocked session.

- a change that touches **F4NRM's list**, one of **his panels**, or **the wire**:
  branch, tests, **PR**, then carry on with something else. Never hold a whole
  session for a PR, and never push any of the three straight to `main`.
- a release or a tag. Never; the operator and F4NRM cut those.
- deleting branches, history, or work you did not create.
- a measurement that needs a radio, an off-air recording, or another person.
- a decision that is genuinely his: which of two products to build, a licence
  question, a public reply in his name, money.
- a check you cannot run here (the wasm target, a platform build). Say so in the
  journal and never claim the result.

**Keep the journal as you go, not at the end.**
[`docs/internal/AUTONOMY-JOURNAL.md`](docs/internal/AUTONOMY-JOURNAL.md), newest
first, one entry per work item, written when the item ends — sessions are closed
without warning. Each entry carries the item, what was done, **the commands and
the numbers that say whether it worked**, the commits, what is still open, and
the plain-language summary at the top. Never rewrite an old entry to look
better; correct it with a new line.

**The queue moves when the work moves** — not batched at the end of a session.

- **A task arrives** — put it in `TODO.md` under *Now* (one line: what, why, and
  where it came from) and in the board's *In Progress*, **before** starting it.
- **A task finishes** — move it in `TODO.md` to *Recently done* under today's
  date with one line, and to the board's *Achieved*; take it out of *In
  Progress*. If a re-measurable number changed (crate version,
  `PROTO_VERSION`, open issues and PRs), fix it in *Where the work stands* and
  in `TODO.md`'s state line too.
- **A task blocks** — put it under *Waiting on other people* with the single
  question, and carry on.
- The board is scripted (`gh project item-create` + `item-edit`). If the
  `project` scope is not granted, update `PROJECT-BOARD.md` itself and say in the
  journal that the GitHub board is behind.

**A finished task gets a plain-language summary** — the first thing in the reply,
and the top of its journal entry. Three or four sentences for a man who has been
away and has not read the diff: what changed for him in the program, what he can
now do or see, what is still not done, and what you are doing next. No crate
names, no file paths, no "refactored the demodulator" — that belongs in the entry
and the commit message. **If you cannot demonstrate it, it is not finished**: say
what is missing rather than calling it done.

**Then say where you stopped and why, in one line** — so he can restart you with
a word instead of a conversation.

## The wire

`PROTO_VERSION` and `VERSION_BYTE` live in `crates/sdroxide-proto/src/lib.rs`,
which is the **only** register that is kept current — one entry per bump, each
naming both numbers (`239 → 240`, `VERSION_BYTE 0x3B → 0x3C`). Current on
`main`: **241 / `0x3D`**; two numbers are claimed in open PRs — 242 by #174
(8PSK) and 243 by the voice-hunting PR.

- **Append last, always.** A variant inserted mid-enum shifts every discriminant
  below it; a field inserted mid-struct shifts every field below it on a struct
  that rides the wire whole. `Action` variants are on the wire too (through the
  bindings) — treat one as a wire change.
- **Take the next number yourself and check it against the register**, then add
  the register line. Two branches doing that correctly still collide, and the
  later one moves.
- ⛔ **A wire change goes to a PR, never straight to `main`.** Two people work
  this repository — the maintainer's agent here and F4NRM's sessions — and
  `PROTO_VERSION` (with `VERSION_BYTE` beside it) is one counter between them.
  Claiming a number by pushing it to `main` is a claim nobody else can see until
  it is too late; claiming it in a **PR** puts the number in a diff the other
  side reads before it picks its own. So: branch, bump, register line, tests,
  open the PR, and let it be merged — the same for anything that adds a
  `Command`, a `ServerMsg`, a `RadioEvent`, a `Mode` or an `Action` variant, a
  `DigiConfig`/`RadioConfig`/`NetworkConfig` field, or a `Mode` discriminant.
  Everything wire-neutral still goes straight to `main` as before.
- **Where two PRs do collide, the later one moves** and says so in the register,
  as it did on 2026-10-10 (199 → 200 → 201).
- **`VERSION_BYTE` rides `PROTO_VERSION` and nothing reminds you.** Bump it, or
  the client passes every test and is refused at the socket.
- A new wire variant needs a postcard round-trip test.
- **`UiSettings` is not postcard-safe** (`spot_colors`/`bandplan_colors`
  deserialize to a `Vec` where the derived serializer writes an array). The
  presentation-only wire type is `ClientScreen`. `InputSettings` *is* safe, and
  that is checked.
- `DigiConfig` and `RadioConfig` ride whole: **never write one from a panel's
  stale copy** — one field at a time, or a command that carries the field.

## Build, test, and the gate

- `cargo check --workspace --all-targets` — the gate, and it must be **silent**.
  It is the only check that sees a `dead_code` stub standing in for a test.
- After a change, test **the crates you touched**, not the workspace:
  `cargo test --release -p <crate>`. Reserve `--workspace` for a release day —
  it spends most of its time in two flakes and links ~72 engine test binaries.
- **`sdroxide-radio`'s test set is the expensive one — name the binary**:
  `cargo test -p sdroxide-radio --test cw_panadapter`, not the package. Linking
  is the cost, not compiling; `--all-targets` is seconds.
- **The browser build is a separate check**: `cargo check --release --target
  wasm32-unknown-unknown -p sdroxide-ui` whenever wasm-reachable code changes.
  The full workspace check cannot see it.
- `cargo test --workspace` builds `examples/`, so **a scratch probe parked in a
  crate's `examples/` breaks the gate** and is invisible to `git status` as a
  change. A committed probe that is meant to be there is the right shape.
- Known flakes under a parallel run, all passing alone: `skim_window` (bounded
  IQ channel; under load the spots arrive and the callsigns do not — read a
  failure as "the machine was busy"), `sdroxide-pluto`'s `iiod_loopback`, and
  `sdroxide`'s `icomnet_source`.
- A test that touches the machine it runs on is not a test: split the decision
  from the side effect and test the decision.
- **A harness that takes every default is testing the defaults.** `EngineConfig`
  has two materially different configurations (`audio: Some` / `None`); when the
  subject is a station, build the station.
- `SDROXIDE_SETTINGS_TRACE=1` draws a numbered scale down the settings page and
  labels every widget with its own y, so an overlap is a number rather than an
  argument.

## Releases

- ⛔ **No release happens by itself, and an agent does not cut one.** No workflow
  in `.github/workflows/` carries a `schedule:` trigger; `release.yml` runs on a
  tag push, `nightly.yml` is `workflow_dispatch` only. **F4NRM cuts the
  pre-releases**, and a full release is cut when he says everything is good —
  so ask, and never tag on your own initiative.
- A release day is the full gate: `cargo check --workspace --all-targets`
  silent and `cargo test --release --workspace` green before the tag.
- **Bump `Cargo.toml` and let cargo refresh `Cargo.lock`, then check the lock
  against the tag's own tree with `cargo check --locked -p <crate>`.** The
  workflow builds `--locked`, so a stale lock fails every platform — that
  happened once, on `v2.0.9_brown`, and the wasm job had already gone green.
- Write the changelog entry in `CHANGELOG.md` (rename `## [Unreleased]`, add a
  fresh one above), then tag `vX.Y.Z_brown` and push — `release.yml` runs on the
  tag. **Do not also dispatch it**: two identical full releases race on the
  uploads.
- **Never re-cut a tag.** A bad tag is recovered with a **fourth-point tag**
  (`v2.0.9.1_brown`) on the same crate version; cancel the failed run first,
  because `create release` is tolerant of failed build jobs and will otherwise
  publish a partial page.
- Install locally with `cargo build --release` and copy the binary to
  `~/.cargo/bin/sdroxide` (after `pkill -x sdroxide`).
- A release page with **no assets** is the shape to watch for after a cancelled
  run — three lines of `gh` to list releases with their asset counts.

## Pull requests to this fork

A PR must be **clean and mergeable as it stands**, because that is what saves the
maintainer's build time: up to date with `main` (a merge commit is fine, no
rebase required), `cargo check --workspace --all-targets` silent, the touched
crates' tests passing, `rustfmt --check` clean on everything touched, and **one
complete thing per PR** — as many commits as you like, tidied before opening. If
a check cannot be run, do not open it: say what blocked it.

**Anything that touches the wire is a PR by rule, even when it is finished** —
see "The wire": a `PROTO_VERSION`/`VERSION_BYTE` bump is one counter shared with
F4NRM's sessions, and a PR is how a number is claimed without colliding.

- **F4NRM's PRs are approved and merged without waiting** when they are sensible
  and merge cleanly (`gh pr merge <n> --merge --admin`): sensible means it does
  what its body says and carries no red flag its own body has not declared.
  Otherwise leave it and say why on the PR.
- A sandboxed build of the radio engine needs **both** `github.com/xiph/opus`
  and `media.xiph.org` reachable; `sdroxide-rade` fetches opus and its models at
  build time. Nothing else in the tree fetches.

## The bench

- **SDRplay RSP1** through the `sdrplay_apiService` system unit (it is not called
  `sdrplay_api`). Gain 20–59 dB, 0.01–2000 MHz. **`SoapySDRUtil --find` is not
  to be trusted here** — it has reported "No devices found" while both radios
  opened in the program. `tools/rsp1-capture` builds in place and writes
  interleaved CF32 that `--file` reads back; it reads the rate back from the
  radio, because it once wrote the right number of samples at the wrong rate.
- **CRT SS9900v** (11 m CB) as radio 1: `Backend::UsbAudio`, no CAT, keyed by
  **VOX** off the audio we send, with a CH55x paddle in its key jack. It is a
  transmitter under our control, which makes the RSP1 + CRT **loopback** the
  technique to reach for when a decode fix cannot wait for the band: one keys,
  the other receives, no ionosphere in the path. That has confirmed a CB CQ fix
  both offline and with a real station.
- **Transmitting is authorised** on the licence-free frequency — **keep it
  short**, never sit on the air, and ask a hand for the dial on a rig with no CAT.
- Three traps that have each cost an evening: the rig's own USB card has
  **`Mic` (capture) and `PCM` (playback) as separate levels** and `PCM` at 2 %
  looks exactly like an app fault (`amixer -c <card> sget 'PCM'`); the squelch
  defaults to wide open, so the Auto-record gate must be given a real threshold
  before it can prove anything; and a **wireless mouse radiates into the
  SS9900v**, moving the S-meter with no app involved.
- There is **no Pluto on this network** and no HackRF on the bench.

## Focus until the operator says otherwise (2026-10-24)

- **No UI work for now.** F4NRM is working through the detachable windows and the
  layouts; two people editing the same screens is how both get worse. **Leave the
  screens to him.** Exactly two exceptions: a **new mode** needs its own panel and
  controls, or something is **very off** (a crash, unreadable ink, a control that
  does the wrong thing).
- **The programme is the modes that did not finish in 2.1.0.** **STANAG 4285
  first** — the receiver locks real captures and Annex E is built, but the coded
  payload has never been read off the air — then the rest of the half-done list
  in [`MODE-PROGRESS.md`](docs/internal/MODE-PROGRESS.md): the 8PSK vestigial
  filter, the four wide `C`/`RC` PSK banks, the speeds that lose a word, and the
  MT63/Contestia/DominoEX variants. `MODE-PHASE-4-PROMPT.md`'s five-column table
  is the deliverable the operator asked for.
- **F4NRM's list still holds**, including the variants: Olivia, RTTY, FT8,
  NAVTEX, WEFAX, SSTV, DAB, APRS, WFM, DSC, RDS are his and are not part of this
  programme, however half-done a journal line makes one look. A fault there is
  still his to be told about and ours to fix **as a PR** — never as an
  unrequested tidy-up, and never straight to `main`. The **RSP1** is not on that
  list: it is the bench's receiver.

## Where the work stands

State at 2026-10-24, and the thing to re-measure rather than trust:

- `main` at crate **2.1.0**, `PROTO_VERSION` **239**; latest release
  **v2.1.0.2_brown** (latest) and **v2.1.0.2.1_brown** (pre-release). The
  2.1.0 release was the big mode release: THOR, MFSK, Contestia, MT63, DominoEX,
  the coded PSK banks, DTMF and STANAG 4285, with Wavelog, the Aoki schedule,
  SSTV FTP upload, the FDM-DUO's power steps and per-memory colours.
- **The live queue is the mode work**:
  [`MODE-PHASE-4-PROMPT.md`](docs/internal/MODE-PHASE-4-PROMPT.md) is the brief —
  finish the modes, get TX working for everything fldigi can transmit, and
  **vendor a mode when the port and the bench have both been exhausted**. The
  measurements are in [`MODE-PROGRESS.md`](docs/internal/MODE-PROGRESS.md), and
  the mode table at the end of the brief is the deliverable.
- **Open issues** (5): #132 and #133 the Brick2 frequency and LSB-as-USB reports
  ([`BRICK2-HANDOVER.md`](docs/internal/BRICK2-HANDOVER.md) — the code half is
  merged, the radio half needs a Brick2), #153 Pluto socket stalls (asks for the
  diagnostic report), #162 NAVTEX needing to tune 516.6 kHz instead of 518
  (the dial-is-channel−1.7 kHz rule), #105 AppImage (planned for a stable
  release). **No open PRs.**
- **Live handovers**: ALE (built, **never decoded a real burst** — the manual
  §3.29 asks for a recording, and that is the only thing missing), STANAG 4285
  (needs a coded off-air capture), Olivia (receive proven; our transmit unproven
  by another station), CW keyer (built and benched), SWL schedule sources
  (scoped, not built), FST4W (do not ship as wired).
- **Confirmed on the air by users**, so not to be re-opened as unproven: WEFAX,
  DAB, SSTV, DSC, and **HD Radio** (reported on r/ShortwavePlus, 2026-10-24).
- ⛔ **The SDRuno look is parked, and is not to be raised.** The operator's own
  pet project, on branch `fork/sdruno-clone` (off `fork/sdruno-look`), **not to be
  merged into `main` and not to appear in a proposal, a reply, a release note or
  the TODO.** He will call for it when he has session room. The record stays in
  [`SDRUNO-UI-HANDOVER.md`](docs/internal/SDRUNO-UI-HANDOVER.md).
- **Decided, do not re-litigate**: **DMR** is out — the air interface is
  ordinary DSP but the AMBE+2 variant it needs is covered by US Patent 8,359,197
  until 2028-05-20, and this fork is public with a named maintainer; the answer
  to askers is the patent, and the detail is in
  [`SESSIONS.md`](docs/internal/SESSIONS.md) under "DMR". **The ATS Mini's
  RadioScript firmware and DiamondRadio** are closed and unlicensed — research
  only, nothing built, nothing copied; `Backend::AtsMini` speaks the stock MIT
  firmware, which is the licence-clean path. **DAB is F4NRM's** and confirmed on
  the air: do not pick up a DAB item. **Phone UI** is delegated as well.

## History

[`docs/internal/SESSIONS.md`](docs/internal/SESSIONS.md) is this file as it grew
between 2026-09-25 and 2026-10-24 — the sessions, the measurements, the traps and
the wrong turns, newest first. Read it when a rule here needs its evidence, or
before touching something it says has a history. It is a record: numbers in it
describe the day they were taken.
