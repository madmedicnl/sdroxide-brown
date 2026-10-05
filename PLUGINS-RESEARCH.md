# User plugins — a research note

**Status: research only. Nothing here is built, and nothing here should be
built until the questions below are answered.** Written 2026-10-05 after the
operator asked whether users could write plugins in Python or Lua.

The honest headline: **yes, it is technically possible, and it is the wrong
next thing to do to this program.** The reasons are below, and they are
structural rather than matters of taste.

## What the options actually are

| Route | What it costs us | Verdict |
|---|---|---|
| **Lua** (`mlua`) | Small, embeddable, no system dependency, fast to start. Ships as a library inside the binary. | The technically easy one. Still a huge product decision. |
| **Python** (`pyo3` + CPython) | Needs `libpython` on the machine **or** a bundled interpreter. Static embedding is documented as fragile and not first-class. Every distribution (`.deb`, `.msi`, `.dmg`, AppImage, 8 build targets) grows a Python dependency and a new class of "won't start on this box" bug. | Hard, and it lands on the release pipeline we are already trying to make faster. |
| **Python** (RustPython) | Pure-Rust interpreter, links statically, no system Python. | Solves deployment. Is *not* CPython — no C extensions, and a meaningful part of the standard library is missing. A user's numpy/pandas/requests script would not run. |
| **Pyodide** (Python in WASM) | Works in the browser build, which is the half of this program that runs on a phone. | Interesting for the wasm side only, and a separate implementation from any native one. |
| **A plugin *protocol* instead** — external process, or a data/UDP hook | A documented interface and nothing else. | See below. This is probably the one. |

## Why "yes" is not "do it"

**1. Every real plugin wants engine internals, not UI.** What would a user
actually script? A new decoder, a new CAT dialect, a skimming hook, a logger
that post-processes decodes. All of those sit *below* the UI and touch
`Command`/`RadioEvent` and the DSP. Exposing that to a scripting language means
exposing **the transmit path and the wire protocol** to arbitrary code. This
program's house rules already treat the transmitter and the bindable control
surface as safety-critical — that is the whole reason a paddle cannot arm a
CAT-keyed rig and why control bindings need an explicit acknowledgement.

A plugin API is a remote-control surface for the transmitter. "It is opt-in"
does not help; the plugin is the thing the user installed.

**2. There is no upstream appetite and no fork audience for it.** The maintainer
has consistently steered *away* from additions without a general audience, and
§12 of the standing direction says new work is fork-only. A plugin API is the
opposite of fork-only: it is a permanent maintenance surface that outlives
every feature anyone has asked for so far, and it would need supporting across
every `PROTO_VERSION` bump.

**3. The wasm half cannot have it.** `sdroxide-ui` is shared between the native
build and the browser, and the browser is where this program is actually used on
a phone. A native-only plugin system makes the phone experience strictly worse,
and the fork's own standing rule is that native-only crates must never become
dependencies of a wasm-targeted crate.

**4. It would grow the release we are trying to shrink.** The 15-job matrix
exists because this program has many radio backends. Adding an interpreter means
an interpreter in every `.deb`, `.msi`, `.dmg` and AppImage, on 8 targets.

## The cheap version that gets most of the value

If the goal is "a user can add their own thing without forking", three options
in increasing order of cost — none of which is a scripting language:

1. **A documented decode-log format.** Everything decoded is already in a
   structured form with CSV/ADIF export and `swl_log_to_csv`. A user who wants
   something built on top of the decodes can already read them.
2. **An external hook.** The engine already speaks a WebSocket protocol
   (`--server`). A user program can already subscribe to a running station and
   read decodes, spots and state. This *is* a plugin API — it just costs nothing
   because it is already there.
3. **A WASM plugin target**, gated behind the existing sign-in and explicitly
   denied anything touching transmit. Weeks of work, and still a security
   surface.

## The questions to answer before building anything

- **What is the concrete plugin someone would write that they cannot write
  today?** Without an answer this is a platform project, and the standing
  direction says new work is fork-only because *this fork serves a different
  user group*. Nobody has asked for one.
- **Does it need to work in the browser?** If yes, Lua/RustPython is the only
  sane route and PyO3 is out.
- **May a plugin transmit?** If the answer is yes, this needs the kind of
  security review this program has never had, and that is a decision for the
  operator, not an implementation detail.

## What would make me revisit this

A user asking for a specific thing they cannot do today — "I want my own
decoder for X", "I want to post-process decodes into my own log format" — plus
a clear answer that it must work in the browser. Until then this note is the
whole of it.