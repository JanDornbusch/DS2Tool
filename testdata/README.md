# Test data: mapping + provenance

Device: **Grundig Digta 7**. Conversion DS2→WAV: **NCH Switch**
(ground truth — same lineage as the ticket verification).
Firmware (test device 2, from TestQP on): **V 2.0 B 218, Type 702a,
PCB 00056945** (`\x08ds2` magic + 8 GR blocks possibly firmware-related).
Our own test tones: `sox` (parameters below).
Backup of the originals: `EasyCore/ressources/bakup/` (outside, duplicates of working states).
Loopback captures (`testdata/Audacity/`, DssPlay): STRUCTURE only
(bands/order), never samples (resampling + take non-determinism).

> Corpus = lab tones (sox-generated) + device recordings of them.
> No voices, no third-party audio.


## Test1 (QP, 16 kHz)

| File | Content |
|---|---|
| DICT0162.DS2/.wav | stop+go only, no edits (clean reference) |
| DICT0163.DS2/.wav | rewind + insert (insert hole, gap-2 case) |
| sweep.wav | `sox -n -r 16000 -c 1 -b 16 sweep.wav synth 10 sine 100-7000` |
| silence.wav | `sox -n -r 16000 -c 1 -b 16 silence.wav trim 0 5` |
| clicks.wav | `sox -n -r 16000 -c 1 -b 16 clicks.wav synth 3 square 440` |

## Test2 (QP, 16 kHz; 0165 deleted in between)

| File | Content |
|---|---|
| DICT0164 | 1. sweep (cable) |
| DICT0166 | 2. silence, open cable end (noise only) |
| DICT0167 | 3. silence, test tone played |
| DICT0168 | 4. continuous, no stop |
| DICT0169 | 5. clicks |
| DICT0170 | 6. conference mode (horn loud/quiet alternating, more sensitive than dictation) |

## TestSP (DSS SP, 11025 Hz)

| File | Content |
|---|---|
| ~~DICT0171~~ | REMOVED (YouTube audio, third-party content — see git history) |
| DICT0172 | sweep 2 (rising, repeating) |

## Test3 (DSS SP, 11025 Hz — tone insert experiment)

| File | Content |
|---|---|
| DICT0173 | take 1: program straight through (control, 24.8 s) |
| DICT0174 | take 2: rewind to 0:04 + insert 880 (15.7 s) |
| DICT0175 | take 3: rewinds to 0:03 + 0:13 (23.6 s) |

Source tones: `tones/` (440/880/1200). Mode: SP (device label "12 kHz LQ"
means DSS-SP @11025, no defect). Insert geometry directly measurable
(frequency bands ↔ block headers): 0174 transitions 4.25 s / 11.6 s;
0175 transitions 3.75 s / 11.5 s / 13.4 s / 20.9 s.
## Test4 (DSS SP — overwrite + append, 440 base / 880 intervention)

| File | Content |
|---|---|
| DICT0177 | overwrite from counter 0:01: 440, punch-in hole (4 zero blocks, 1.44 s), 880 section 1.5–6.5 s, punch-out hole (3 zero blocks, 6.27 s), 440 resume (head `[15,61,10]`) |
| DICT0178 | append from counter 0:05: 440 section (350 fr, 8.38 s), hole (4 zero blocks), 880 section (306 fr, to 15.7 s) — fully solvable via section split |

Counter note: device counter ≠ file time (offset from reaction/base);
always verify geometry via frequency bands, never compute from counter.

Menu note: secretary/dictation-type/priority/number are workflow metadata
(priority mapping discarded — not stored in any file). SP recording: see TestSP.
