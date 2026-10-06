# Grundig Digta container: measured format (as of 2026-10-06)

Basis: Grundig Digta 7 (`.DS2` QP, `.DSS` SP), FFmpeg ticket #6091 as
hypothesis, verified against `testdata/` (see matrix below). The ticket covers
codec core/tables; **the container differs** (own wrapper, documented here).

## 1. File layout

```
0x000–0x5FF  file header, 1536 bytes (metadata, black-and-white texts, IDs)
0x600–0xDFF  4 × 512-byte section blocks `GR___<type>_000000...`
0xE00–…      audio blocks, 512 bytes, 6-byte head + 506 payload
```

Magic (first 4 bytes): `\x07ds2` (QP/SP) or `\x07dss` (DSS SP) —
the ticket names `\x03ds2`; on these devices it's the `0x07` variant.
Harmless (version, no structure change).

Section blocks (identical in all files except the GUID in `9607`):

| Offset | Content | Reading |
|---|---|---|
| 0x600 | `GR___9607_000000` + GUID (`3B3FA14C-…`) + FF | recording ID |
| 0x800 | `GR___0504_000000` + `01 01` + FF | static (unknown) |
| 0xA00 | `GR___0607_000000` + `06 00` + FF | static (`06` ≈ format, like audio byte 4) |
| 0xC00 | `GR___1008_000000` + FF | static (unknown) |

No edit lists inside (static except GUID) — insert structure lives
only in the audio block heads.

## 2. Audio block heads

`[b0, b1, fc, 0xFF, fmt, 0xFF]`, 506 payload bytes behind it.

- `b0`: `0x0F` (QP; SP without swap) or `0x8F` (SP with swap init, bit 7).
- `b1`: countdown per section (see below).
- `fc` (byte 2): frames *starting* in this block. **Sum over all
  blocks = total frames; duration = sum × samples/frame ÷ rate matches
  every file sample-exactly** (demux rule, see §4).
- `fmt` (byte 4): `6` = DS2 QP (16000 Hz, 448-bit frames, 256 samples/frame),
  `0` = DSS SP (11025 Hz, 264 samples/frame) or DS2 SP (12000 Hz, 288 samples).
  Variant `8` from the ticket comments not observed here.

## 3. Sections + countdown (QP)

- Section ≈ 28 blocks: head `[*, 3, 10]` (10 frames) + 27 data blocks with
  `b1` = 30, 29, …, 4 (9 frames each) = 253 frames ≈ 4.05 s @16 kHz.
  A new section starts 512-aligned (stop+go, pause/resume).
- `b1` rule (ticket, confirmed): payload offset of the first frame bit
  `off = b1*2 − 6` bytes (overlap packing, frames span blocks).
  Countdown −1/block ⇒ offsets −2/block ⇒ continuous bitstream.
- Full blocks: 9 frames = 504 of 506 payload bytes (2 left over).

## 4. Demux rule (proven)

Read heads from audio start, **discard payload of `fc == 0` blocks**,
concatenate the rest, read exactly `sum(fc)` frames of frame-bits sequentially.
Zero padding / GR wrapper is never consumed. Arithmetic (0162: 991 fr →
15.86 s; 0163: 1894 fr → 30.30 s) matches WAV durations exactly; same for all
Test2/TestSP files (SP: 264 samples/frame @11025).

Special cases:
- **Resume block** (after stop): fewer frames (e.g. 6) + zero padding to
  block end (0162: 164 B; 6×56 + 164 + 6 head = 506 ✓).
- **Pause gap**: `fc = 0` block (0163 `0xBE00`), payload skipped.
- **Insert hole**: fully zeroed block including head (0163 `0x12C00`).
- **File end**: partial block + zero tail (0163: 8 frames + 32 B).

## 5. Post-insert: flat packing (offset map 06.10.)

After an insert hole (zero block + `b1` jump upward, here 6 → 27) the
countdown does **not** apply: the offset map over blocks 144–167 shows best
offset **0** everywhere (R1 offsets 48…2 lose). Each block carries its 9 frames
in payload `[0:504]`, 2 slack bytes per block are skipped
(24 × 2 B = 48 B = exactly the arithmetic budget up to section head
`0x15E00`). Per-block fresh decoding matches file order
(0.75–0.99; examples: block 151 → frame 1344 at 0.994; 147–149 →
1308/1317/1326 and 160–162 → 1425/1434/1443 each consecutive).
R1-sequential (slack as bits) drifts +2 B/block.

## 6. Gap-2 state/slack detail (CLOSED see §20, player confirms code)

Bytes and framing stand (see §5); decoded sequentially it stays
0.6–0.99 instead of 1.0; carry explodes (wrong coefficients ⇒ slack bytes
partly land as bits); exact from frame ~1501. Narrowed to
state initialization and/or slack distribution in exactly one
24-block window. Next candidates: carry chain from corrected
seg1 end, slack positions per block (start vs. end), fresh
section heads as reset points. Hanging test in the suite
(`decode_0163_laenge` checks length only for now).

### Gap-2 experiment log (goal: corr 1.0 vs. NCH)

| Approach | Result |
|---|---|
| R1 (skip stub, from 72912, reset) | garbage 20–24 s, exact from 24 s |
| R1 + carry (seg1 end) | no difference |
| Flat from 72864 (stub in, tail skip), reset/carry | 0.044 (wrong) |
| Flat-slack (skip 2 B/block end), reset | 0.60–0.99, 213/216 frames slightly off |
| Flat-slack + carry | explosion (peaks up to 430k) |
| Single-block starts ±112 B sweeped (9-fr window) | no hit > 0.27 |
| Gain hypothesis (LS fit) | 1.46 only an effect of explosion peaks, no global factor |
| Lag sweep ±300 samples | no shift (lag 0 best) |
| QP7-records suspicion | discarded (147+ are valid QP frames) |
| Scaling single blocks / mixed offsets | no improvement |

Reference result logged: corr 0.545383, 56809/484864 bad
(Rust and Python implementations bit-identical — shared
demux model, no port difference).

### Ruled out (with reason)

Mode mix (all blocks type 6), sample shift, gain, edit lists in
GR blocks (static except GUID), duplicates (insert audio unique,
no match > 0.3 elsewhere), reader phase, de-emphasis/pitch state as
sole cause (finite memories ≪ 4 s error span), re-sorting
(countdown monotone 27→4), codebook mistake (all |k| < 1, max 0.9986).

### Splice markers (byte 1 anomalous, finding 06.10., Test3/4)

Blocks `[0x0F, b1 ∉ 3…30, fc 4–13, 0xFF, fmt, 0xFF]` sit exactly at
splice/punch joints: `[15,192,4]` (0174-46), `[15,165,5]` (0175-44),
`[15,164,5]` (0175-90), `[15,61,10]` (0177-29, punch-out after overwrite).
Bit-7 rule (≥ 128) too narrow — 61 belongs here (outside countdown 3…30).
Reading open (info marker for player edit functions vs.
demux directive); content follows. Overwrite otherwise leaves NO
markers (0177 punch-in: normal `[15,3,13]` head).

### Harness bug + calibration (finding 06.10.)

All manual SP packet scripts used `range(0,38,2)` instead of correct
`range(0,40,2)` (byte 38 wrong per swap packet) — all custom-slicing
results (tail, C, carry, straight, wordswap, skip, cont-V1 tests)
were thereby invalidated. Still valid: section splits via
`read_dss_file` (correct code) + all QP tests (no packet loop).
After fix: calibration middle [17–46] = 0.9997 (trustworthy).

### Per-block anchored model (finding 06.10., partial)

Swap init from own byte0-bit7 + cont offset (`2·b1+2·swap−6`) per block:
blocks 47/48/51 match exactly on it (36/22/22, +0.91–0.98).
Rest (49/50/52/54/56 inverted −0.68…−0.95 or scattered, 53/55/57/58
weak) needs another rule (state carry? sign?). Full tail
per-block anchored: 0.68. Tail offset map (fixed): no block fits
foreign — content lies in file order.

### Swap-reset bug in reference SP (finding 06.10.)

Reset positions (`pos in swap_reset_positions`) are checked via exact
dict lookup, but the running position jumps over them in 42/40 steps
(0174: 7586 → 7626 over key 7590) — reset never fires.
Consequence: pre-gap pad tail (e.g. 128 B) is consumed as packets,
phase error permanent (no recovery — unlike QP countdown,
which catches itself at section heads). Fix direction: section split at
gaps + fresh swap init per section (A/middle thus 1.0/0.9997),
resume-head byte0-bit7 as init.

### Cont-as-skip is wrong (finding 06.10.)

Per-block anchoring (`off = 2·b1 + 2·swap − 6`, skip bytes `[0:off)` per block,
continuous alternation) was tested and discarded:
breaks even clean files (0173: 1.0 → 0.13). So byte 1 is no
skip offset — rather a UI/seek metadata or overlap descriptor whose
bytes are still consumed (sequential reading stays correct
for intact sections).

### Skip-forward + reset (finding 06.10., ALL SP files solved, in the tool)

Model (implemented: `decode_sp_sections` in `crates/grundig`): per block
anchor = `base + max(0, 2·b1 + 2·swap − 6)`; if anchor > run position:
jump + swap/state/decoder fresh (resume-head byte0-bit7 as init),
else continuous. Validated tool-vs-NCH: 0172: 1.0 · 0173: 1.0 · 0174: 0.999630 · 0175: 0.999662 · 0177: 0.997020 ·
0178: 0.999897 — lengths exact everywhere (frame budget), remaining errors only
transients at jump points. Regression via suite (`sp_decode_*`).
QP analogue gains nothing over the reference (cut/reset logic already exists
there; the gap-2 error lies INSIDE the segment (flat/slack),
see below).

### QP segment-state rule (pause vs. insert)

Pause/resume (gap 1, 0163): no reset — carry exact except 907 samples
transient in the first block (encoder state keeps running). Insert hole
(gap 2): reset behavior clarified (§20 — NCH artifact, no demux defect;
previously: R1 reset and carry both wrong,
fresh-per-block closest. Hypothesis: partial blocks
(`[6,6]` tail) and heads pack flat from payload+0, full countdown blocks
via overlap; flat after insert hole until next head.

## 7. SP path (TestSP)

`\x07dss`, GR wrapper identical, audio at `0xE00`, heads
`[0x0F/0x8F, b1, 12|13, 0xFF, 0, 0xFF]` (swap bit like ticket SP),
11025 Hz. After wrapper strip, standard SP decoding: corr 0.990336
(second-0 transient only). Digta SP recording mode thus covered.

## 8. Matrix (reference decoder vs. NCH WAVs)

| File | Mode | Content | Corr | maxdiff |
|---|---|---|---|---|
| 0162 | QP | stop+go | 1.000000 | 3 |
| 0164–0170 | QP | sweep/silence/clicks/conference/speech | ≥0.9989 | ≤3 |
| 0172 | SP | sweep2 | 0.990336 | start transient | (0171 YouTube audio removed)
| 0163 | QP | rewind+insert | 0.545383 vs NCH (NCH artifact) | CLOSED see §20 (player: ref-S1 c=0.81–0.96) |
| 0173 | SP | program straight through (control) | 1.000000 | start transient only |
| 0174 | SP | rewind 0:04 + insert 880 | vs NCH 0.199 (NCH pad) | player rule see §19 (player skips 0.5 s pad) |
| 0175 | SP | rewinds 0:03 + 0:13 | vs NCH 0.103 (tones, metric artifact) | structure Rust=NCH=player see §21 |
| 0177 | SP | overwrite 0:01 (440 + 880 punch 1.5–6.5 s) | S0✓ S1✓ S2✗ | resume heads [15,3,13] + [15,61,10] |
| 0178 | SP | append 0:05 (440 + 880 from ~8.4 s) | 0.9999+ | fully solvable (section split) |

WAV-side note: NCH sometimes appends trailing chunks after `data`
(0162: 216 B) — the chunk length always counts, never the file size
(test helper `wav_data` respects that).

## 9. FFmpeg outlook

Goal stays patch (demuxer `libavformat/dss.c` for `\x07` variants +
new codec IDs + f64 path; existing `dss_sp` is no basis).
Order: first this tool (full matrix incl. gap-2 solution),
then upstream form. No proprietary disassemblies in the repo
(clean room, see `ATTRIBUTION.md`).

## 10. Probes P1 (resume-head semantics, finding 06.10.)

Base 0173 + 2 zero blocks after block 20, only block-21 head varied
(`fc` preserved; probes in `probes/`, NCH WAVs in `testdata/NCH/`).

- NCH distinguishes heads: P1b (`61`) ≈ P1c (`192`) at 0.98
  (transparent: bands like clean), P1a (`3`) and P1d (`143,3`) each own
  (~0.83; P1a with 880 blip 6.5–7 s).
- Reference ignores heads (P1a–d output identical) and eats gap pad
  as packets (phase break; reset keys never fire, see §6).
- Outside gap region (±1–2 s) everything sample-exact; gaps cost no
  time (`fc = 0` ⇒ 0 frames).
- Model: NCH trust path (natural heads; stumbles over synthetically
  inconsistent gaps → local blip) vs. safe mode (anomalous head →
  skip pad region, resync). Cont-as-skip stays dead
  (breaks clean files).
- NCH settings: WAV/11025/mono/16 bit, dither off, no resampling
  (see `probes/README.md`). Rate finding: SP = 11025 Hz effective
  (440.99/881.97 Hz measured), "12 kHz" nominal.

## 11. Oracle hygiene: budgeting NCH artifacts (06.10.)

NCH is not infallible — transients at joints (sec-0 starts,
splice blips, 907-sample transient) can be NCH-side (fade/WSOLA/
smoothing; `AVCWSOLA` class in the player binary evidences time stretching as
a mechanism). Rule: **transient mismatches** (≤ ~1 s, exact after)
count as NCH-side until a second oracle contradicts;
**sustained content mismatches** (tails, gap-2: seconds of wrong content
at correct bands) stay real demux questions (no
smoothing artifact explains 4 s of wrong content).
Second oracle wanted: Grundig converter (player doesn't save!) or
Olympus DSS Player Pro (DS2-capable). Until then: 3-pillar comparison
(protocol expectation, NCH, reference).

## 12. Spike protocol (player code → behavior → own implementation)

Clean room: read decompile only for **behavioral rules** (words/math,
no code copy into the repo, no slices in product code). Flow per spike:
(1) identify target function in decompile, (2) record rule in docs,
(3) throwaway spike implements the rule from the docs (not from
the decompile), (4) verify against test vectors, (5) product code
is built independently from docs + vectors. Spikes do NOT land in the repo
(or under `spikes/` with provenance note + deletion date).
Priority: S1 state init per block/section in the playback path; S2
byte1≥128 handling; S3 swap reset at gaps.

## 13. Player captures (loopback oracle, finding 06.10.)

Chain validated via bands (resampling-proof): 0173 control exact
(440/880/1200 at the same places), 0174 insert exact (440/880/440 at
4.25/11.5 s). Sample comparisons are INVALID (double resampling).
0163 capture discarded (hash, no envelope match —
protocol slip or player glitch at exactly this take; recapture done afterwards
(clock 1.0101, both captures identical). Player is truth (§§20–21).)

Addendum: recapture + P2a capture show **take non-determinism**
(bands stable, samples ≠; L/R dual-mono clean, no clipping).
Loopback validity scope thereby final: STRUCTURE (bands, order,
silence-vs-content), never sample exactness. NCH determinism proven
(DICT0162_1 vs _2, two conversions: 0 samples difference; vs. old file
maxdiff 1) — bit-exact statements thus hardened.

## 14. Firmware findings (gh0700.hex, strings, 06.10.)

Update package Digta 7 (10 MB, multilingual help HTML + code). Usable
facts (behavior, no code copy):

- Recording modes (menu 1031): **insert, overwrite, append as presets**;
  auto-switching ("After end of recording, overwrite mode is activated
  again") ⇒ mixed structures in one file are NORMAL (no
  user error needed). Explains 0174/0175/0177 geometries.
- **Index marks** settable during recording AND playback ("Set an index
  during recording") — candidate for marker blocks (`byte1≥128`): either
  user indexes or auto marks at edits (open; ask users!).
- **ORG_BLOCK/ORG_FRAME/ORG_RELFRAME vs. POS_BLOCK/POS_FRAME/RELFRAME,
  CURFRAME/FRAMEIDX**: device tracks original vs. play position
  (edit mapping, at runtime). No offset tables found in files
  ⇒ the section structure itself IS the map (sequential
  playback in file order; jump-mark theory stays dead).
- Section tags confirmed in firmware (9607+GUID, 0504, 0607, 1008) plus
  **fifth type `GR___1203_000000`** (not observed in our files).
- Codecs/platform: Blackfin DSP hints (`adi_*`), ASR hooks
  (`DIGTA7_ASR`), `DssMover`, `testdict.dss`. Irrelevant for demux.
- CPU: **ARM Thumb/Thumb-2, LE, 32 bit** (4820 prologues, 0× A32).
  Ghidra: `ARM:LE:32`, Cortex variant (fallback v5t), compiler default.
  Load the whole file (forensic, container accepted as-is). Navigate by map:
  code at 0x30000–0x1C0000 (446 Thumb-2 functions), REV cluster
  0x30DE4–0x35B2C (byte-swap code!), GR templates at
  0x4A5xxx/0x521xxx/0x973xxx; rest text/modules/padding. File is NOT
  Intel-HEX (single random colon — loader errors are correct).
  Base address open (pool votes scatter). Code regions approx.
  0x30000–0x1C0000, 0x4A0000–0x4D0000, 0x700000–0x770000, 0x970000–end
  (zero zones = module borders); base address unknown → load from 0,
  calibrate via string xrefs (GR tags!). Blackfin parts (eDigta.ldr)
  ignored (no Ghidra module; irrelevant for container questions).

## 15. DICT0016 (QP minimal insert, finding 06.10.)

440 base + 880 insert (counter 0:04–0:12), new test device (FW V2.0 B218,
`\x08ds2`, audio at `0x1000`!). S0 (304 fr) 1.0 exact · S1 (450 fr,
880 take) 0.9999 · S2 (651 fr, marker head `[15,166,4]`): first ~250 fr
deviating (NCH partly CLIPPING up to 32766!), exact from ~fr 1004.
Reading: stop/handling transient (slider stop!) + NCH-side
post-processing (limiter/AGC/de-clicker) vs. reference raw decode —
no demux defect (all slicing/state combos systematically exhausted; content
lies in file order). Do not rebuild NCH transients bit-exactly
(player DSP, not codec).

## 16. Recording path: cable noise + mic mute + NCH limiter (06.10.)

- Cable/plugs/time offset: all cable takes (Test2/3/4) carry
  analog noise (hiss, level wobble). Bit-exact statements on those only with
  tolerance (±1–8 LSB, transients excluded); silence takes are
  noisy by nature (0166: RMS 13).
- Jack mutes mic ⇒ handling-noise theory DEAD (slider clicks
  etc. never reach the recording). Disturbed onset regions come
  device-side (encoder/AGC/DC at (re)start), not acoustic.
- NCH limits (brickwall evidence: 4.6–5.6% samples at 3276x in
  onset/gap regions) — NCH transients are post-processing, not codec.
  Separation stands: transients NCH-side, sustained content
  demux question.

## 17. Joint regions: characterization (finding 06.10.)

NCH joints (section transitions after gaps) contain ~100–250 frames
of unique, non-silent content (no duplicate elsewhere, no
gain artifact, no silence, no transient peaks alone):
bit-exact after. All slicing/state/init combos exhausted
(SP + QP) with no hit. Reading clarified (§21 — NCH-side, player confirms
reference). Formerly open readings: device-specific
joint coding (firmware record path, S1 spike) or unknown
packing rule for resume onsets. NCH stays oracle for content,
reference for everything outside joints.

## 18. Gap table: gaps are identical, stubs hang off the head (06.10.)

All 14 gaps (11 files): payloads ALWAYS fully 0 (506/506) —
gaps identical to each other, no content. Only exception type:
`fc = 0` WITH head (once: 0163 block 88 `[15,27,0]` + 48 data + 458×0,
pause gap). Resume stub (zeros after resume head): depends ONLY on head,
never on predecessor: `[15,3,x]` → 0 · `[15,27,x]` → 48 ·
`[15,61/164/165/166/183/192,x]` → 64. Pre-gap block almost always partial
(fc 2–10; exception 0174 gap 1 full = pause instead of stop).
Anti-glue hypothesis (silence pads against word merging) supported:
joints carry silence cushions (hole + stub), content intact on both sides.

## 19. Player rule: skip the gap, NCH pads (validated 06.10.)

DICT0174, gap 2 (blk 43–45, file fr 493 = 11.45 s, marker head `[15,192,4]`,
stub 64): NCH renders 0.5 s silence (11.5–12.0 s), DssPlay plays through
(band at 11.5 s straight 440, only play hole = trailing capture stop
15.25–15.75 s). Math: NCH content 15.71 − 0.5 = 15.21 s;
play content 15.98 − 0.5 = 15.48 s; 15.48/15.21 = 1.0178 = clock factor
(control 0173: 1.0171, Δ 0.0003). Player skips exactly the NCH pad.
Gap 1 (blk 15–16, 4.23 s, normal head `[15,3,13]`, stub 0): both seamless
(440→880 at 4.25 s). FFmpeg rule: follow the player (concatenate frames
gaplessly, drop stub zeros); NCH pad = transcriber behavior, not
codec. Basis: 1 validated case (0174); 0163 capture discarded (hash mismatch).

## 20. Gap-2 revisited: player confirms code, NCH was artifact (06.10.)

0163 gap-2 (blk 143, file fr 1281 = 20.50 s, resume `[15,27,9]`, stub 48)
with player as truth (both captures, clock 1.0101 identical):
ref-S1 fully found in player — [0:50] c=0.900, [0:100] c=0.811,
[100:220] c=0.936, [220:300] c=0.962, gaplessly contiguous.
Rust full decode (30.30 s) = vendor S1 bit-exact (corr 1.0000),
Rust-S1[100:220] in player c=0.936. NCH mismatch (~220 fr) was
NCH-side (parallel-system activation vs. test version). Gap-2 thus
closed. (Addendum: 0016/0017 onsets + 0175 also closed, see §21.)

## 21. Three player captures close QP onsets + 0175 (06.10.)

0017-S2 onset [0:100] in player Pearson 0.975 ([100:200] 0.982,
[200:300] 0.994); 0016-S2 onset [0:100] 0.921 ([100:220] 0.999).
Both QP joint mismatches were NCH-side — with player as truth
the reference plays (and Rust full decode, = vendor bit-exact 1.0000)
the joint content exactly. 0175 (SP pure tones): bands Rust = NCH =
player (440/880/440/1200/440 in sync); sample Pearson methodically
blind (pure tones + clock drift), no contradiction — old
"S2/S3 open (0.103)" was metric artifact. No length problem: NCH-0175 =
260568 samples = Rust exact; the 987-fr hand sum missed partial blocks
(own counting, no code bug).
