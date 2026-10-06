# Probes (synthetic DSS files for questioning NCH)

Base: `testdata/Test3/DICT0173.DSS` (clean, reference decode = 1.0).
All probes identical except block 21 (after 2 injected zero blocks
behind block 20): only `byte0`/`byte1` of the resume head vary, `fc`
unchanged (= same frame count/duration expected — differences in output
are thus pure head semantics).

| Probe | Head block 21 | Question |
|---|---|---|
| P1a | `[15, 3, 12]` | control (natural resume shape) |
| P1b | `[15, 61, 12]` | 0177 anomaly byte (61): does NCH treat it differently? |
| P1c | `[15, 192, 12]` | marker class: own treatment? |
| P1d | `[143, 3, 12]` | swap-init set at resume: phase restart? |

(`fc` deliberately = original everywhere (12), else the duration balance breaks.)

## NCH settings (exactly like this, please)

- Format: **WAV, uncompressed PCM**
- Sample rate: **same as source (11025 Hz)** — do not force resampling!
- Channels: **mono**, bit depth: **16 bit**
- **No** normalization/effects/fades, trim or append **nothing**
- Keep file names (`P1a.wav` …), back into this folder (or as a
  package — I'll collect them and compare mutually + vs. reference)

## Probes Q1 (QP gap rule, finding open)

Base 0162 (clean, 1.0) + 2 zero blocks after block 50, only resume head
varied (`fc` preserved). NCH settings: WAV/**16000**/mono/16 bit,
dither off, no resampling.

| Probe | Head block 51 | Question |
|---|---|---|
| Q1a | `[15, 3, 9]` | pause-resume (fresh section): transparent like P1b/c? |
| Q1b | `[15, 27, 9]` | insert-resume (countdown jump 9→27, 0163 geometry): blip or garbage? |

Evaluation: NCH probes mutually + vs. clean-0162 (where does what
deviate?) + vs. reference (eats pad/phase break?). Goal: measure the
QP gap rule directly instead of guessing from 0163.

Tone files Test3: 440.99 Hz / 881.97 Hz at 11025 Hz playback ⇒
**SP = 11025 Hz effective** ("12 kHz" in the menu is nominal). QP = 16000 Hz
(undisputed). No rate doubts the matrix anymore.

## Probes P2 (tail after marker/anomalous head, finding open)

Base 0173, gap after block 20, then tail = original blocks 21+:

| Probe | After the gap | Question |
|---|---|---|
| P2a | 2×0 + real 0174-block-46 (`[15,192,4]`, authentic bytes) | marker + normal tail like 0174: does NCH solve the tail? |
| P2b | 3×0 + head `[15,61,fc]` | 0177 geometry (punch-out): tail solvable? |
| P2c | 6×0 + head `[15,164,fc]` | 0175-S3 geometry: tail solvable? |

NCH settings like P1 (WAV/11025/mono/16 bit, dither off, no resampling).
Evaluation: NCH tails mutually + vs. reference section split
(new per probe). Goal: isolate whether marker/anomalous heads make the
tail solvable for NCH (then a rule is derivable) or not.
