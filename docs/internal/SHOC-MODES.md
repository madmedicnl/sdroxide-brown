# The Shoc catalogue — a parameter index for the modes we do not carry

<https://cartoonman.github.io/Shoc/> is the documentation site for shoc.ch's
decoder suite, and it documents **117 modes**, one page each, in the same shape:
modulation, tone count, shift, symbol rate, error correction, alphabet, then a
tuning note and a spectrogram. That table is exactly what scoping a decoder
needs, and it is facts — so the numbers are indexed here, with a link to each
page for the rest.

**What this file is not.** It is not a copy of the catalogue. No prose and no
image is reproduced: the wording of a page and its spectrogram are shoc.ch's,
and the picture is looked at on the site. What is kept is the parameter set, in
our own table, which is what a decoder port needs first.

**How it was made, and how to refresh it.** `python3 tools/shoc-index/fetch.py`
reads the index, then every page, and prints this table; run it by hand when a
new mode is wanted. Two pages on the index are dead links (`gwofdm.htm`,
`pactoriiiii.htm`) and are the only ones missing.

**Read the blanks as "the page does not state it there"**, not as absent: several
modes (the Piccolo and Coquelet families in particular) describe their timing in
prose instead, and a few list their shift only under the demodulator settings.
Every page carries a spectrogram, which is the best single reference for what a
mode looks like on the waterfall.

**Licence and patents.** No licence is stated on the site, so nothing of its text
or images is redistributed here — the parameters are facts and the URL is the
reference. Two entries are worth a check before any code is written: **PACTOR**
is SCS's and patent-encumbered, and this fork already declines the AMBE+2 family
for the same reason (`AGENTS.md`, DMR). The CIS, ARQ, SITOR and STANAG entries
are decades old and published in the utility-listening literature.

**What it points at first** is in
[`MODE-ROADMAP.md`](MODE-ROADMAP.md): CIS-11 (the Russian military FSK mode that
dominates 4–18 MHz utility listening), then the MIL-STD-188-110 serial tones
that share STANAG 4285's DSP, then ARQ-E/E3 for the utilities, and ALE-3G only
after 2G decodes a real station.

## Where our own modes sit in it

The catalogue is a useful cross-check on our own parameter tables, because it was
written independently: **DSC** (2-FSK, 170 Hz, 100 Bd), **HFDL** (PSK2/4/8 at
1800 Bd), **MFSK16** (16 tones, 15.625 Bd, FEC), **Olivia** (MFSK, 200 Hz
spacing, 31.25 Bd), **VDL2**, **ALE-2G**, **STANAG 4285**, **PSK31**, **POCSAG**
and **FLEX** all appear with the numbers we implement.

Of the rest, the modes this fork does **not** carry and the catalogue documents
fully: the ARQ family (`ARQ-E`, `ARQ-E3`, `ARQ-N`, `ARQ-6-90/98`, the `ARQ-M`
series), `SITOR-A/B`, `FEC-A`, `HNG-FEC`, `RUM-FEC`, `POL-ARQ`, `SWED-ARQ`,
`SI-ARQ/FEC`, **CIS-11/12/14/36/50/405/8181**, `TWINPLEX`, `Piccolo MK6/MK12`,
the Coquelets, `Clover II/2000`, `PACTOR I–III`, `G-TOR`, `MIL-STD-188-110B`
16/39-tone, `STANAG 4415/4481/4529/4539/5065`, `LINK-11 CLEW/SLEW`, `ALE-3G`,
`MD-674`, `CV-786`, and the VHF paging/trunking set. That is the list to work
down, in the order [`MODE-ROADMAP.md`](MODE-ROADMAP.md) gives.

## The index

| mode | group | modulation | tones | shift (Hz) | baud | error correction | alphabet |
|---|---|---|---|---|---|---|---|
| [Alcatel 801H](https://cartoonman.github.io/Shoc/alcatel801h.htm) | Standard Decoders HF | FSK | 8 |  |  |  |  |
| [ALIS](https://cartoonman.github.io/Shoc/alis.htm) | Standard Decoders HF |  | 2 | 170 | 228.7 | CRC-16 | ITA-2 |
| [ALIS-2](https://cartoonman.github.io/Shoc/alis2.htm) | Standard Decoders HF | FSK | 8 |  | 240.82 | CRC-16 | ITA-5 |
| [ARQ-6-90](https://cartoonman.github.io/Shoc/arq690.htm) | Standard Decoders HF | FSK | 2 | 400 | 200 | ARQ | CCIR-476 |
| [ARQ-6-98](https://cartoonman.github.io/Shoc/arq698.htm) | Standard Decoders HF | FSK | 2 | 200 | 200 | ARQ | CCIR-476 |
| [ARQ-E](https://cartoonman.github.io/Shoc/arqe.htm) | Standard Decoders HF | FSK | 2 |  | 30 … 650 | ARQ | ITA-2 extended |
| [ARQ-E3](https://cartoonman.github.io/Shoc/arqe3.htm) | Standard Decoders HF | FSK | 2 | 400 | 30 … 650 | ARQ | ITA-3 |
| [ARQ-M2-242](https://cartoonman.github.io/Shoc/arqm2242.htm) | Standard Decoders HF | FSK | 2 | 430 | 96 / 200 |  | ITA-3 |
| [ARQ-M2-342](https://cartoonman.github.io/Shoc/arqm2342.htm) | Standard Decoders HF | FSK | 2 | 400 | 96 / 200 |  | ITA-3 |
| [ARQ-M4-242](https://cartoonman.github.io/Shoc/arqm4242.htm) | Standard Decoders HF | FSK | 2 | 170 | 172 / 192 |  | ITA-3 |
| [ARQ-M4-342](https://cartoonman.github.io/Shoc/arqm4342.htm) | Standard Decoders HF | FSK | 2 | 400 | 172 / 192 |  | ITA-3 |
| [ARQ-N](https://cartoonman.github.io/Shoc/arqn.htm) | Standard Decoders HF | FSK | 2 | 850 | 96 | ARQ | ITA-2P |
| [ASCII 7 Bit](https://cartoonman.github.io/Shoc/ascii7bit.htm) | Standard Decoders HF | FSK | 2 | 173 | 100 |  |  |
| [ASCII 8 Bit](https://cartoonman.github.io/Shoc/ascii8bit.htm) | Standard Decoders HF | FSK | 2 | 492 | 150 |  |  |
| [AUTOSPEC](https://cartoonman.github.io/Shoc/autospec.htm) | Standard Decoders HF | FSK | 2 | 270 | 68.5 |  | ITA-2 |
| [Baudot Async](https://cartoonman.github.io/Shoc/baudotasync.htm) | Standard Decoders HF | FSK | 2 | 450 / 850 | 50 / 75 / 100 |  |  |
| [Baudot Sync](https://cartoonman.github.io/Shoc/baudotsync.htm) | Standard Decoders HF | FSK | 2 | 450 / 850 | 200 |  |  |
| [BULG-ASCII](https://cartoonman.github.io/Shoc/bulgascii.htm) | Standard Decoders HF | FSK | 2 | 500 | 120 |  |  |
| [CHU](https://cartoonman.github.io/Shoc/chu.htm) | Standard Decoders HF | FSK | 2 | 200 | 500 |  |  |
| [CIS-11](https://cartoonman.github.io/Shoc/cis11.htm) | Standard Decoders HF | FSK | 2 | 500 | 100 | Parity | ITA-2 |
| [CIS-12](https://cartoonman.github.io/Shoc/cis12.htm) | Standard Decoders HF | Multi-channel PSK2 / PSK4 |  |  | 120 |  |  |
| [CIS-14](https://cartoonman.github.io/Shoc/cis14.htm) | Standard Decoders HF | FSK |  | 500 | 96 |  | M2 Cyrillic |
| [CIS-36](https://cartoonman.github.io/Shoc/cis36.htm) | Standard Decoders HF | MFSK | 36 |  |  |  |  |
| [CIS-36-50](https://cartoonman.github.io/Shoc/cis3650.htm) | Standard Decoders HF | FSK | 2 | 200 / 250 / 500 | 50 |  |  |
| [CIS 405-3915](https://cartoonman.github.io/Shoc/cis4053915.htm) | Standard Decoders HF | FSK | 2 | 500 | 40.5 |  |  |
| [CIS-8181](https://cartoonman.github.io/Shoc/cis8181.htm) | Standard Decoders HF | FSK | 2 | 500 | 81 |  |  |
| [Clover-II](https://cartoonman.github.io/Shoc/cloverii.htm) | Standard Decoders HF | DBPSK / DQPSK / 8-DPSK / 8P2A / 16P4A |  |  |  |  |  |
| [Clover 2000](https://cartoonman.github.io/Shoc/clover2000.htm) | Standard Decoders HF | DBPSK / DQPSK / 8-DPSK / 8P2A / 16P4A |  |  |  |  |  |
| [Codan 3012](https://cartoonman.github.io/Shoc/codan3012.htm) | Standard Decoders HF | PSK | 2 |  | 80 |  | CCIR-476 |
| [Codan SelCal](https://cartoonman.github.io/Shoc/codanselcal.htm) | Standard Decoders HF | FSK | 2 | 200 | 100 |  |  |
| [Coquelet-8](https://cartoonman.github.io/Shoc/coquelet8.htm) | Standard Decoders HF | FSK | 8 | 26.67 |  |  | ITA-2 /  ATU-80 |
| [Coquelet-13](https://cartoonman.github.io/Shoc/coquelet13.htm) | Standard Decoders HF | MFSK | 13 | 30 |  |  | ITA-2 / ATU-80 |
| [Coquelet-80](https://cartoonman.github.io/Shoc/coquelet80.htm) | Standard Decoders HF | FSK | 8 | 26.67 |  |  | ITA-2 /  ATU-80 |
| [DGPS](https://cartoonman.github.io/Shoc/dgps.htm) | Standard Decoders HF | MSK / QPSK |  |  | 200 |  |  |
| [DSC](https://cartoonman.github.io/Shoc/dsc.htm) | Standard Decoders HF | FSK | 2 | 170 | 100 |  |  |
| [DUP-ARQ](https://cartoonman.github.io/Shoc/duparq.htm) | Standard Decoders HF | FSK | 2 | 170 | 125 |  | ITA-2 |
| [FEC-A](https://cartoonman.github.io/Shoc/feca.htm) | Standard Decoders HF | FSK | 2 | 850 | 145 | Convolutional coding | ITA-2 |
| [FSK 400/500](https://cartoonman.github.io/Shoc/fsk400500.htm) | Standard Decoders HF | MFSK | 4 |  | 100 |  |  |
| [Globe Wireless FSK](https://cartoonman.github.io/Shoc/globewirelessfsk.htm) | Standard Decoders HF | FSK | 2 | 200 | 100 | Parity checksum |  |
| [Globe Wireless PSK](https://cartoonman.github.io/Shoc/globewirelesspsk.htm) | Standard Decoders HF | DQPSK |  |  | 200 |  |  |
| [G-TOR](https://cartoonman.github.io/Shoc/gtor.htm) | Standard Decoders HF | FSK | 2 | 170 | 100 / 200 / 300 |  | ITA-5 |
| [HFDL](https://cartoonman.github.io/Shoc/hfdl.htm) | Standard Decoders HF | PSK2/4/8 |  | 170 | 1800 | ARQ |  |
| [HNG-FEC](https://cartoonman.github.io/Shoc/hngfec.htm) | Standard Decoders HF | FSK | 2 | 500 | 100.5 |  | ITA-2 |
| [MEROD](https://cartoonman.github.io/Shoc/merod.htm) | Standard Decoders HF | FSK | 2 | 810 | 266 | BCH (127,78) | MEROD specific 6 bit |
| [MFSK8](https://cartoonman.github.io/Shoc/mfsk8.htm) | Standard Decoders HF | FSK | 32 |  | 7.8125 | FEC |  |
| [MFSK16](https://cartoonman.github.io/Shoc/mfsk16.htm) | Standard Decoders HF | FSK | 16 |  | 15.625 | FEC |  |
| [Morse](https://cartoonman.github.io/Shoc/morse.htm) | Standard Decoders HF | Carrier keyed on/off |  |  | Depending on data-rate |  |  |
| [Olivia](https://cartoonman.github.io/Shoc/olivia.htm) | Standard Decoders HF | MFSK | 4/8/16/32 | 200 | 31.25 |  | ITA-5 |
| [Packet 300](https://cartoonman.github.io/Shoc/packet300.htm) | Standard Decoders HF | FSK | 2 | 200 | 500 |  |  |
| [PACTOR I](https://cartoonman.github.io/Shoc/pactori.htm) | Standard Decoders HF | FSK | 2 | 200 | 100 / 200 |  |  |
| [PACTOR I FEC](https://cartoonman.github.io/Shoc/pactorifec.htm) | Standard Decoders HF | FSK | 2 | 200 | 100 / 200 |  |  |
| [PACTOR II](https://cartoonman.github.io/Shoc/pactorii.htm) | Standard Decoders HF | DBPSK,DQPSK,8-DPSK,16-DPSK |  |  | 200 |  |  |
| [PACTOR II FEC](https://cartoonman.github.io/Shoc/pactoriifec.htm) | Standard Decoders HF | DQPSK |  |  | 200 |  |  |
| [PACTOR III](https://cartoonman.github.io/Shoc/pactoriii.htm) | Standard Decoders HF | DBPSK,DQPSK |  |  | 100 per channel |  |  |
| [Piccolo MK6](https://cartoonman.github.io/Shoc/piccolomk6.htm) | Standard Decoders HF | FSK | 6 | 20 | 20 |  | ITA-2 |
| [Piccolo MK12](https://cartoonman.github.io/Shoc/piccolomk12.htm) | Standard Decoders HF | FSK | 12 | 20 | 20 |  | ITA-5 |
| [POL-ARQ](https://cartoonman.github.io/Shoc/polarq.htm) | Standard Decoders HF | FSK | 2 | 270 | 100 |  | CCIR-476 |
| [PSK10](https://cartoonman.github.io/Shoc/psk10.htm) | Standard Decoders HF | DBPSK |  |  | 10 |  |  |
| [PSK10-AM](https://cartoonman.github.io/Shoc/psk10am.htm) | Standard Decoders HF | DBPSK |  |  | 10 / 31.25 / 50 |  |  |
| [PSK31](https://cartoonman.github.io/Shoc/psk31.htm) | Standard Decoders HF | DBPSK,QPSK |  |  | 31.25  (62.5 / 125) |  |  |
| [RUM-FEC](https://cartoonman.github.io/Shoc/rumfec.htm) | Standard Decoders HF | FSK | 2 | 400 | 164.5 |  | RUM-FEC |
| [SI-ARQ](https://cartoonman.github.io/Shoc/siarq.htm) | Standard Decoders HF | FSK | 2 | 170 | 96 / 192 | ARQ | ITA-2 |
| [SI-FEC](https://cartoonman.github.io/Shoc/sifec.htm) | Standard Decoders HF | FSK | 2 | 170 | 96 / 192 | FEC | ITA-2 |
| [SITOR-A](https://cartoonman.github.io/Shoc/sitora.htm) | Standard Decoders HF | FSK | 2 | 170 | 100 | ARQ | ITA-2 |
| [SITOR-B](https://cartoonman.github.io/Shoc/sitorb.htm) | Standard Decoders HF | FSK | 2 | 170 / 400 | 100 | FEC | ITA-2 |
| [SP14](https://cartoonman.github.io/Shoc/sp14.htm) | Standard Decoders HF |  | 14 |  | 7.5 |  |  |
| [SPREAD 51](https://cartoonman.github.io/Shoc/spread51.htm) | Standard Decoders HF | FSK | 2 | 400 | 102.7 |  | ITA-2 |
| [SWED-ARQ](https://cartoonman.github.io/Shoc/swedarq.htm) | Standard Decoders HF | FSK | 2 | 400 | 100 |  | CCIR-476 |
| [TWINPLEX](https://cartoonman.github.io/Shoc/twinplex.htm) | Standard Decoders HF | FSK | 4 |  | 50 | ARQ | ITA-3 |
| [Visel](https://cartoonman.github.io/Shoc/visel.htm) | Standard Decoders HF | FSK | 2 | 300 | 81.3 / 123.5 / 125 | FEC | ITA-2 |
| [ACARS VHF](https://cartoonman.github.io/Shoc/acarsvhf.htm) | Standard Decoders VHF/UHF |  |  | 1200 | 2400 |  |  |
| [AIS](https://cartoonman.github.io/Shoc/ais.htm) | Standard Decoders VHF/UHF |  |  |  | 9600 |  |  |
| [CCIR](https://cartoonman.github.io/Shoc/ccir.htm) | Standard Decoders VHF/UHF | Multitone | 16 |  |  |  |  |
| [CCITT](https://cartoonman.github.io/Shoc/ccitt.htm) | Standard Decoders VHF/UHF | Multitone | 15 |  |  |  |  |
| [CTCSS](https://cartoonman.github.io/Shoc/ctcss.htm) | Standard Decoders VHF/UHF | Multi-tone | 38 |  |  |  |  |
| [DMR](https://cartoonman.github.io/Shoc/dmr.htm) | Standard Decoders VHF/UHF | FSK | 4 | 4080 | 4800 |  |  |
| [dPMR](https://cartoonman.github.io/Shoc/dpmr.htm) | Standard Decoders VHF/UHF | Multi tone | 4 | 2200 | 2400 |  |  |
| [DSC](https://cartoonman.github.io/Shoc/dsc1.htm) | Standard Decoders VHF/UHF |  | 2 | 800 | 1200 |  |  |
| [EEA](https://cartoonman.github.io/Shoc/eea.htm) | Standard Decoders VHF/UHF | Multitone | 16 |  |  |  |  |
| [EIA](https://cartoonman.github.io/Shoc/eia.htm) | Standard Decoders VHF/UHF | Multitone | 15 |  |  |  |  |
| [Euro](https://cartoonman.github.io/Shoc/euro.htm) | Standard Decoders VHF/UHF | Multitone | 16 |  |  |  |  |
| [FLEX](https://cartoonman.github.io/Shoc/flex.htm) | Standard Decoders VHF/UHF | FFSK-2 / FFSK-4 |  | 9600 | 1600 / 3200 |  | ITA-5 |
| [FMS-BOS](https://cartoonman.github.io/Shoc/fmsbos.htm) | Standard Decoders VHF/UHF |  |  | 600 | 1200 |  |  |
| [Golay Pager](https://cartoonman.github.io/Shoc/golaypager.htm) | Standard Decoders VHF/UHF | FSK | 2 | 2000 | 300 / 600 |  |  |
| [MOBITEX-1200](https://cartoonman.github.io/Shoc/mobitex1200.htm) | Standard Decoders VHF/UHF | FFSK | 2 | 600 | 1200 |  |  |
| [MOBITEX-8000](https://cartoonman.github.io/Shoc/mobitex8000.htm) | Standard Decoders VHF/UHF | GMSK | 2 | 4000 | 8000 |  |  |
| [MPT1327](https://cartoonman.github.io/Shoc/mpt1327.htm) | Standard Decoders VHF/UHF | FFSK |  |  | 1200 |  |  |
| [NATEL](https://cartoonman.github.io/Shoc/natel.htm) | Standard Decoders VHF/UHF | Multitone | 16 |  |  |  |  |
| [NMT450](https://cartoonman.github.io/Shoc/nmt450.htm) | Standard Decoders VHF/UHF | FFSK |  | 600 | 1200 |  |  |
| [NXDN](https://cartoonman.github.io/Shoc/nxdn.htm) | Standard Decoders VHF/UHF |  | 4 |  |  |  |  |
| [POCSAG](https://cartoonman.github.io/Shoc/pocsag.htm) | Standard Decoders VHF/UHF | FFSK | 2 | 8500 | 512 / 1200 / 2400 |  | ITA-5 |
| [TETRA](https://cartoonman.github.io/Shoc/tetra.htm) | Standard Decoders VHF/UHF |  |  |  | 18000 |  |  |
| [Tetrapol](https://cartoonman.github.io/Shoc/tetrapol.htm) | Standard Decoders VHF/UHF | GMSK |  |  | 8000 |  |  |
| [VDEW](https://cartoonman.github.io/Shoc/vdew.htm) | Standard Decoders VHF/UHF | Multitone | 12 |  |  |  |  |
| [VDL 2](https://cartoonman.github.io/Shoc/vdl2.htm) | Standard Decoders VHF/UHF | DPSK |  |  | 10500 |  |  |
| [VDL 3](https://cartoonman.github.io/Shoc/vdl3.htm) | Standard Decoders VHF/UHF | DPSK |  |  | 10500 |  |  |
| [ZVEI](https://cartoonman.github.io/Shoc/zvei.htm) | Standard Decoders VHF/UHF | Multitone | 16 |  |  |  |  |
| [ALE-2G](https://cartoonman.github.io/Shoc/ale2g.htm) | PREMIUM DECODERS | FSK | 8 |  | 125 |  | ITA5 |
| [ALE-3G](https://cartoonman.github.io/Shoc/ale3g.htm) | PREMIUM DECODERS | PSK |  |  | 2400 |  | ITA5 |
| [ALE-400](https://cartoonman.github.io/Shoc/ale400.htm) | PREMIUM DECODERS | FSK | 8 |  | 50 |  | ITA5 |
| [CHN 4plus4](https://cartoonman.github.io/Shoc/chn4plus4.htm) | PREMIUM DECODERS | QPSK |  |  | 75 |  |  |
| [CV-786](https://cartoonman.github.io/Shoc/cv786.htm) | PREMIUM DECODERS | FSK | 2 | 850 | 50 / 75 / 100 / 110 / 150 Bit/s | ARQ | ITA-2 / ITA-5 |
| [HC-ARQ](https://cartoonman.github.io/Shoc/hcarq.htm) | PREMIUM DECODERS | FSK |  | 200 | 240 |  | ITA-2 |
| [LINK 11 CLEW](https://cartoonman.github.io/Shoc/link11clew.htm) | PREMIUM DECODERS | DQPSK |  |  |  |  |  |
| [LINK 11 SLEW](https://cartoonman.github.io/Shoc/link11slew.htm) | PREMIUM DECODERS | PSK |  |  | 2400 |  |  |
| [MD-674](https://cartoonman.github.io/Shoc/md6741.htm) | PREMIUM DECODERS | FSK | 2 | 85 | 50 … 150 | ARQ | ITA-2 /  ITA-5 |
| [MIL-M-55529A](https://cartoonman.github.io/Shoc/milm55529a.htm) | PREMIUM DECODERS |  |  |  | 50, 75, 100, 110 |  | ITA2, ITA5 |
| [MIL-STD-188-110B 16Tone](https://cartoonman.github.io/Shoc/milstd188110b16tone.htm) | PREMIUM DECODERS | DPSK | 16 |  | 75 |  |  |
| [MIL-STD-188-110B 39Tone](https://cartoonman.github.io/Shoc/milstd188110b39tone.htm) | PREMIUM DECODERS | QDPSK | 1 + 39 |  | 44.44 |  |  |
| [STANAG-4197](https://cartoonman.github.io/Shoc/stanag4197.htm) | PREMIUM DECODERS | QPSK |  | 56.25 | 2400 | FEC |  |
| [STANAG-4285](https://cartoonman.github.io/Shoc/stanag4285.htm) | PREMIUM DECODERS |  |  |  | 2400 |  |  |
| [STANAG-4415](https://cartoonman.github.io/Shoc/stanag4415.htm) | PREMIUM DECODERS | 8PSK |  |  | 2400 | FEC rate 1/2 |  |
| [STANAG-4481](https://cartoonman.github.io/Shoc/stanag4481.htm) | PREMIUM DECODERS |  |  |  |  |  |  |
| [STANAG-4529](https://cartoonman.github.io/Shoc/stanag4529.htm) | PREMIUM DECODERS |  |  |  | 1200 |  |  |
| [STANAG-4539](https://cartoonman.github.io/Shoc/stanag4539.htm) | PREMIUM DECODERS | QPSK, 8PSK, 16QAM, 32QAM, 64QAM |  |  | 2400 | FEC , code rate 3/4 |  |
| [STANAG-4539 HDR](https://cartoonman.github.io/Shoc/stanag4539hdr.htm) | PREMIUM DECODERS | QPSK, 8PSK, 16QAM, 32QAM, 64QAM |  |  | 2400 | FEC , code rate 3/4 |  |
| [STANAG-5065](https://cartoonman.github.io/Shoc/stanag5065.htm) | PREMIUM DECODERS | FSK 2, 3, 4, discr. |  | 85 /170 | 50 / 75 / 100 / 150 |  | ITA2, ITA5 |
