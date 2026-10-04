# CRX 11 m DX cluster — research report

**Status: research only. No application code written.** This is the deliverable
the brief asked for before any implementation. Every endpoint below was
**verified by an actual connection from this machine**, not inferred; anything
not verified is marked as such.

Date: 2026-10-04. Researcher's box: Linux, outbound TCP only, no CRX account.

---

## 1. Summary

The CRX 11 m network is **real and publicly reachable over DXSpider/DXTelnet**,
and — the key finding — **an ordinary 11 m operator can connect to the CB node
with only a callsign; no CRX API key, no password, no portal account is
required.** The authenticated CRX API the brief warned about is a *different*
surface, used by the web portals and the logbook, not by the DXTelnet path.

One endpoint is confirmed working from here:

```
cb.crx.cloud : 7500        (DXSpider V1.55 build 0.67, node 14CRX999, France)
```

An implementation is therefore viable as a **plain DXSpider read-only client**:
connect, send the callsign, read spot lines. No API, no scraping, no auth.

Two facts that shape the design:

- **The ham node rejects 11 m calls.** `ham.crx.cloud:7300` answered
  `Sorry 19DC373 is an invalid callsign` — it validates against the licensed
  callsign database. The 11 m node is the CB one (`:7500`), and the two are
  separate services.
- **`11DX.net` has no reachable cluster port from here.** All of
  7300/7373/7500/8000 returned *Network is unreachable* (it appears to be a
  web-only node, and its own site is behind a GoDaddy IP). It is described as
  "part of the CRX node network", so spots from it should arrive **through**
  the CRX DXSpider node rather than by connecting to 11DX.net directly.

---

## 2. CRX architecture (verified + documented)

From CRX's own project page (`project.crx.cloud/11mcluster`, by Bastien
14CRX004) and the portal (`www.crx.cloud`):

- The 11 m cluster began as a modified DXSpider (Rick 30RC222 / Ton 30IR030,
  the old `www.11mcluster.net`) to allow **11 m callsigns**, and CRX continued
  it.
- The network is a mix of **web nodes** and **DXCluster (DXTelnet) nodes**
  ("dxtelnet"). Web-only nodes are synchronised by a PHP **task scheduler** that
  polls a CRX webservice every 30 s–1 min; spots are injected into the network
  by **`crx-dxspider`**, which is connected to the DXCluster network. DXTelnet
  spots are likewise pushed back out to the web nodes.
- `www.crx.cloud` lists the telnet services directly:
  - **DXSPIDER HAM** — `telnet://ham.crx.cloud:7300`
  - **DXSPIDER CB/PMR** — `telnet://cb.crx.cloud:7500` — *"linked to CRX
    NETWORK (dedicated to amateur radio network spots)"*, usable from Ham Radio
    Deluxe / Logger32 / any telnet client.
- The mobile portal (`m.crx.cloud`) states "**Use same LOGIN / PASSWORD as
  portal version**" — i.e. the *web* surface authenticates; the DXTelnet node
  itself does not require it to read spots (verified below).

**CRX API:** separate, authenticated, key-based — used for the portals,
logbook upload and 11DX.net interop. **Not needed and not used** for the
read-only spot feed. (We did not obtain or test an API key; the brief said not
to assume it is accessible, and it is not needed.)

---

## 3. Verified connection transcripts

### 3.1 CB node — accepted, callsign-only, DXSpider

```
$ nc cb.crx.cloud 7500            # or the python probe below
login:
19DC373
Hello 19DC373, this is 14CRX999 in France
running DXSpider V1.55 build 0.67
Cluster: 4 nodes, 13 local / 13 total users  Max users 14  Uptime 0 16:32
Please enter your name, set/name <your name>
Please enter your QTH, set/qth <your qth>
Please enter your location with set/location or set/qra
Please enter your Home Node, set/homenode <your home DX Cluster>
19DC373 de 14CRX999  4-Oct-2026 0922Z dxspider >
```

- The node asks for a callsign (`login: `), and **any plausible 11 m call is
  accepted** — `19DC373` was taken with no password. (The HAM node rejects it,
  see 3.2.)
- The banner is the **standard DXSpider greeting**; the node identifies itself
  as **DXSpider V1.55 build 0.67**.
- The prompt is the DXSpider prompt: `<call> de 14CRX999  <date>  <time>Z dxspider >`.

### 3.2 HAM node — rejects an 11 m call

```
login:
19DC373
Sorry 19DC373 is an invalid callsign
```

So `ham.crx.cloud:7300` validates the callsign against the licensed database;
it is **not** usable for an 11 m client, and it is not the target anyway.

### 3.3 Spot wire format — real 11 m spots, `sh/dx 25`

Read-only command `sh/dx 25` returned the last 25 spots:

```
 27520.0  172ZZ02      4-Oct-2026 0922Z  with 172at124               <47DA103>
 27545.0  26AT027      4-Oct-2026 0922Z                              <60AT101>
 27267.0  19AT461      4-Oct-2026 0919Z                              <19AT168>
 27520.0  172ZZ00      4-Oct-2026 0919Z  Mr. Jack tnx                <47DK137>
 27640.0  1AT/I460     4-Oct-2026 0917Z  Thx Loreto & Pino 73´s IMA <13AT114>
 27540.0  172ZZ002     4-Oct-2026 0916Z  -Jacques-> 30°             <56DX113>
 27610.0  271DA/0      4-Oct-2026 0849Z  CQ CQ Daren                 <25EK111>
 ...
```

Columns: **`freq_kHz`  `spotter`  `DD-Mon-YYYY HHMMZ`  `comment`  `<spotted_call>`**.
This is the standard DXSpider `sh/dx` layout. Live spots (unsolicited, as the
client is connected) arrive on the same connection in the same shape — that is
what a listener reads. Real 11 m frequencies in the sample are 27.266–27.670
MHz, all within the 11 m band.

### 3.4 Interaction commands that produce output

- `set/name <name>` → `Your name is now "<name>"` (confirmed).
- `sh/dx [n]` → the spot list above (confirmed).
- A bogus command → `Invalid command`.

### 3.5 What did **not** work

- `11dx.net` on 7300/7373/7500/8000 — *Network is unreachable* (see §1).
- `cb.crx.cloud:7500` did **not** emit a banner on connect alone; the
  `login: ` prompt appears immediately, but the *cluster* banner only follows
  a callsign. So a client must send a callsign to enter the session.
- Passing a wrong "password" simply got `Invalid command` — there is **no
  password phase** on the CB node for a fresh callsign (it went straight to the
  DXSpider prompt). `set/logininfo` (a password) is an optional sysop-set
  feature, documented in DXSpider, not required to read spots.

---

## 4. Login / identification sequence (as implemented by DXSpider)

This matches the official DXSpider documentation
(`wiki.dxcluster.org/wiki/Logins_and_logouts`,
`dxspider.org/usermanual_en.html` §2.3):

1. TCP connect.
2. Server sends `login: ` (the "login phase"; may be preceded by an optional
   `issue` banner file, empty here).
3. Client sends its callsign (one line).
4. Server prints the greeting (`Hello <call> … DXSpider V1.55 …`), any
   `connect`/`motd` text, the "please enter your name/QTH/…" nudges, then the
   `dxspider >` prompt.
5. Spots then arrive **asynchronously** as line-oriented text; the client sends
   nothing more to stay connected (DXSpider sends periodic keepalive/prompt
   lines). `set/name`, `set/qth` etc. are optional courtesies.

No socket-level keepalive is required by the protocol itself; a TCP keepalive
or a periodic harmless command is enough to detect a dead link.

---

## 5. The spot message format (the shape to parse)

DXSpider emits spots on the wire as single lines. There are **two families**:

- **`sh/dx` output** (as above), padded into columns.
- **Live/spotted lines** (pushed): the same fields, conventionally rendered
  with the spotter in the middle, e.g.
  `DX de <spotter>:  <freq_kHz>  <spot>  <comment>  <HHMMZ>`
  (the AR-Cluster/DXSpider "DX de" form).

Because the brief requires parsing **real examples**, the parser tests should
use the `sh/dx` columns captured in §3.3 verbatim, plus the "DX de" form once a
live spot is captured (see §8 unknown). A regex per field, tolerant of absent
comment, is the `dxclparser` approach and is the right one here.

Field notes from the real data:

- **frequency**: kHz with one decimal (`27520.0`) → Hz `freq_kHz * 1000`.
- **spotter** and **spotted call**: 11 m calls look like `172ZZ02`, `19AT461`,
  `14AT/CF14059`, `271DA/0` — i.e. **our own `is_cb_callsign` grammar** (and
  slash forms) applies; a generic ham-callsign parser will reject them.
- **time**: `4-Oct-2026 0922Z` — day has no leading zero; month is an English
  three-letter abbreviation; **UTC**.
- **comment**: free text, often empty, can contain `´`/`°` (Latin-1 bytes),
  so decode as **latin-1/windows-1252 tolerant**, not strict UTF-8.

This last point matters: the sample contained `73´s` and `30°` that arrived as
`0x92`-style bytes. The parser must not assume UTF-8.

---

## 6. Existing software / libraries

| Source | What | Licence / state |
|---|---|---|
| `dxcllistener` 1.0.3 (crates.io) | tokio listener for DXSpider / AR-Cluster / CC Cluster / RBN; hands each line to a channel; **cannot send commands** | MPL-2.0, 11 k downloads, last updated 2023-10 |
| `dxclparser` 1.0.1 (crates.io) | regex parser (DX/RBN/WCY/WWV/WX/ToAll/ToLocal) | MPL-2.0, 3 k downloads, last updated 2022-09 |
| `DD5HT/dxtracker` / `dxtool` (GitHub) | Rust telnet CLI + library, filter/alerts | see repo |
| Ham Radio Deluxe / Logger32 | the tools CRX names as compatible clients | closed |

**Recommendation: do not take a dependency on either crate.** Both are
MPL-2.0 (fine in a GPL-3 project, but a dependency for ~150 lines of logic),
neither knows the 11 m callsign grammar (which this fork already has in
`sdroxide_types::is_cb_callsign`), and `dxcllistener`'s inability to send a
command is a poor fit if we later want `set/name`. **Write a small in-tree
parser using them as references**, with tests over the real lines in §3.3.
This mirrors the fork's existing approach (`sdroxide-net` hand-rolls its feeds).

---

## 7. Recommended implementation

Matching the brief's seam (`ClusterConnection -> parser -> Spot events -> bus`):

- **Crate/home:** a new module in `sdroxide-net` (already the home of the
  upload and spot-feed clients), e.g. `sdroxide-net/src/dxcluster/`, with a
  `ClusterConnection` trait so the parser and event types are testable with no
  socket. Keep it native-only (`tokio` is already a dependency there).
- **Config** (`net.json` / env): `host` (default `cb.crx.cloud`), `port`
  (default `7500`), `callsign`, optional `name`/`qth` sent once, and an
  on/off switch. **No credentials in the file** — none are needed; if the
  operator wants their real CRX callsign, they type it.
- **Connection:** tokio `TcpStream`, connect timeout, send `"<call>\r\n"` (or
  `\n`; DXSpider accepts both) once, then a read loop; reconnect with backoff
  on drop; a ping/keepalive only for liveness detection.
- **Parser:** pure function `parse_line(&str) -> Option<ClusterEvent>`,
  handling `ClusterEvent::{DxSpot, Announce, Wcy/Wwv, Banner, Prompt, Other}`.
  Reuse `is_cb_callsign` for the call fields. Latin-1 tolerant.
- **Events:** `DxSpot { freq_hz, spotter, spotted, comment, at: DateTime<Utc>,
  mode: None }` — mode is not in the DXSpider line (it is inferred from
  frequency; leave `None` for now, matching the brief's optional field).
- **Read-only:** never send spots or commands except the login line (and,
  optionally, `set/name`). No transmit path.
- **UI (later, not in the first cut):** feed the existing SPOTS window or a new
  list; the brief says start with a **diagnostic client** that connects and
  prints/parses, so the first deliverable is the connection + parser + tests.
- **Testing:** unit tests over the exact `sh/dx` lines captured in §3.3
  (including the `´`/`°` bytes), plus a `#[ignore]` live-connect test gated on
  an env var, like the other off-air/offline fixtures.

---

## 8. What remains unknown / unverified

1. **The live "DX de" push format on this node** — not yet captured (the band
   was quiet during the 20 s watch). The `sh/dx` format is known; a live spot
   should be captured before finalising the parser's second branch. **This is
   the one experiment to run first.**
2. **`set/name` persistence** — whether the node remembers a name across
   sessions (probably not, DXSpider default). Not important for reading spots.
3. **Whether CRX intends the CB node to stay callsign-open.** The portal says
   "LOGIN/PASSWORD as portal version" for the *web*; the telnet node we reached
   did **not** ask for one. It could change; the client should handle an
   unexpected additional prompt gracefully. **Do not hard-code the assumption
   that no password will ever be asked** — parse the prompts and, if one
   appears, surface "this node asked for a password" rather than failing
   silently.
4. **The other named services** (LOG11DX / log11dx.com, RCQSL, DX27, IRDX) —
   not investigated for a *cluster* endpoint; they appear to be **logbook /
   awards / web** services (11DX.net was explicitly unreachable). Any spot
   feed from them should be assumed to arrive via the CRX node, not as separate
   clusters, until shown otherwise.
5. **CRX API itself** — not probed. It is authenticated and not needed.
6. **IPv6** — `11dx.net` failed as unreachable; whether CRX nodes have v6 is
   untested and irrelevant if we use `cb.crx.cloud`.

---

## 9. Sources

- `https://www.crx.cloud/` — the two DXSpider services and their ports.
- `http://project.crx.cloud/11mcluster` — the network's architecture
  (web + DXTelnet nodes, `crx-dxspider`, task-scheduler sync, Bastien 14CRX004).
- `https://dxcb.crx.cloud/` — the CB/PMR web portal (DXTelnet-compatible,
  Logger32-compatible).
- `https://11dx.net/` and the 13AT031 write-up — 11DX.net described as part of
  the CRX node network; no reachable cluster port found.
- DXSpider docs: `https://wiki.dxcluster.org/wiki/Logins_and_logouts`,
  `https://www.dxspider.org/usermanual_en.html` §2.3,
  `https://www.dxcluster.org/client.html` (login phase, `issue`/`connect`/`motd`).
- crates.io: `dxcllistener` 1.0.3, `dxclparser` 1.0.1 (MPL-2.0).
- **Direct live captures from this machine** (§3), 2026-10-04 — the primary
  evidence, reproducible with the probe below.

### Reproduce the core finding

```python
import socket
s = socket.create_connection(("cb.crx.cloud", 7500), timeout=15)
s.sendall(b"19DC373\r\n")
import time; time.sleep(1)
print(s.recv(8192).decode("latin-1", "replace"))   # greeting + prompt
s.sendall(b"sh/dx 25\r\n"); time.sleep(1)
print(s.recv(16384).decode("latin-1", "replace"))  # real 11 m spots
```
