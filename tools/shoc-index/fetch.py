"""Build a local index of the Shoc decoder catalogue's **parameters**.

The catalogue at <https://cartoonman.github.io/Shoc/> documents about seventy
decoders, one page each, and every page carries the same structure: a table of
the mode's modulation, tone count, shift, symbol rate, error correction and
alphabet, then a tuning note and a spectrogram. That table is what a session
needs in order to scope a decoder, and it is facts — so this script reads the
pages and writes them out as one table.

What it does **not** do, deliberately:

- no prose and no images are copied. The wording of a page and its spectrogram
  are the site's; we keep the numbers and the URL, and the picture is looked at
  on the site.
- nothing is fetched at build or run time. This is a working note generator, run
  by hand, and its output is committed as `docs/internal/SHOC-MODES.md`.

    python3 tools/shoc-index/fetch.py > /tmp/opencode/shoc.md

Pages that fail are listed at the end rather than silently skipped, because a
catalogue with a hole in it reads as though the mode has no parameters.
"""
import re
import sys
import time

import requests
from bs4 import BeautifulSoup

INDEX = "https://cartoonman.github.io/Shoc/"
UA = {"User-Agent": "sdroxide-brown documentation index (one-off, by hand)"}

# The index page groups its links; keep the group so the table can be sorted the
# way the catalogue is.
GROUPS = ("Standard Decoders HF", "Standard Decoders VHF/UHF", "PREMIUM DECODERS")


def links(html):
    soup = BeautifulSoup(html, "html.parser")
    out = []
    group = ""
    for node in soup.find_all(["h2", "h3", "a"]):
        if node.name in ("h2", "h3"):
            group = node.get_text(" ", strip=True)
            continue
        href = node.get("href") or ""
        if not href.endswith(".htm") or href.startswith(("gloss", "http")):
            continue
        name = node.get_text(" ", strip=True)
        if name and (group, name, href) not in out:
            out.append((group, name, href))
    return out


def parameters(html):
    """The `Item | Value` rows of the mode-details table, as a flat dict."""
    soup = BeautifulSoup(html, "html.parser")
    # Every row is `[section heading or empty, item, value]`: the left-hand
    # column carries "Standard" / "Demodulator Settings" / "Features" on the row
    # that starts a block, and the same items (symbol rate, shift) come round
    # again under the next heading — so the section is part of the key.
    out = {}
    heading = ""
    for tr in soup.find_all("tr"):
        cells = [c.get_text(" ", strip=True) for c in tr.find_all(["td", "th"])]
        if any(c in ("Item", "Value") for c in cells):
            continue
        if len(cells) == 3 and cells[1] and cells[2]:
            head, key, val = cells
            if head:
                heading = head
            out[f"{heading}: {key}" if heading else key] = val
        elif len(cells) == 2 and cells[0] and cells[1]:
            out[cells[0]] = cells[1]
    text = soup.get_text("\n", strip=True)
    m = re.search(r"Tuning\s*\n(.+)", text)
    if m:
        out["Tuning"] = m.group(1).strip()
    return out


def main():
    r = requests.get(INDEX, headers=UA, timeout=30)
    r.raise_for_status()
    found = links(r.text)
    print(f"# Shoc catalogue index ({len(found)} pages)", file=sys.stderr)

    failed = []
    print("| mode | group | modulation | tones | shift (Hz) | baud | error correction | alphabet |")
    print("|---|---|---|---|---|---|---|---|")
    for group, name, href in found:
        try:
            page = requests.get(INDEX + href, headers=UA, timeout=30)
            page.raise_for_status()
            p = parameters(page.text)
        except Exception as e:  # noqa: BLE001 - a catalogue run wants the list
            failed.append((name, href, str(e)))
            continue
        def get(*keys):
            for k in keys:
                for pk, pv in p.items():
                    if pk.endswith(k):
                        return pv
            return ""
        print(
            "| [{name}]({url}) | {group} | {mod} | {tones} | {shift} | {baud} | {fec} | {alphabet} |".format(
                name=name,
                url=INDEX + href,
                group=group,
                mod=get("Modulation"),
                tones=get("Number of tones"),
                shift=get("Shift (Hz)"),
                baud=get("Symbol rate (Bd)"),
                fec=get("Error correction"),
                alphabet=get("Alphabet"),
            )
        )
        time.sleep(0.4)  # one page at a time; this is somebody's small server

    if failed:
        print(f"\n<!-- {len(failed)} pages could not be read:", file=sys.stderr)
        for name, href, err in failed:
            print(f"  {name} ({href}): {err}", file=sys.stderr)


if __name__ == "__main__":
    main()
