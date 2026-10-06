# Source tones for cable insert experiments

All 16 kHz / mono / 16 bit, sine, level −6 dB (`vol 0.5`, no clipping
over cable+mic). Generated with `sox` (commands below).

| File | Content |
|---|---|
| `seg440_8s.wav` | 8 s 440 Hz |
| `seg880_8s.wav` | 8 s 880 Hz |
| `seg1200_8s.wav` | 8 s 1200 Hz |
| `program_24s.wav` | 440+880+1200 back to back (24 s, one take) |

Generation:
`sox -n -r 16000 -c 1 -b 16 seg<F>_8s.wav synth 8 sine <F> vol 0.5`
(`<F>` = 440/880/1200); program via `sox seg440_8s.wav seg880_8s.wav
seg1200_8s.wav program_24s.wav`.

## Recording protocol (reaction-time safe)

Setup like Test2 (cable, dictation mode QP, same volume, no
level change mid-take). Exact times do NOT matter (±1 s invisible)
— only: segments ≥ 5 s, and **write down the Digta counter at every
button press** (anchor!). Note the mode: insert (splice) vs. overwrite!

- **Take A (control):** play `program_24s.wav` fully, device runs
  through, no buttons. Expectation: 440/880/1200, 8 s each.
- **Take B (1 insert):** play `seg440` + record (~5–8 s) → stop
  both, note counter → rewind device ~3 s (note counter) →
  play `seg880` + record (~5–8 s) → stop, note counter.
- **Take C (2 inserts, optional):** like B, then insert third
  segment `seg1200` the same way.
- Deliver: per take DS2 + NCH-WAV + counter notes + mode (splice/overwrite).

Measurement on my side: spectral centroid per second → insert band
(880/1200) sits exactly on section/block borders or not.

## Morse patterns (reaction-compensation test)

`morse440_12s.wav` / `morse880_12s.wav`: 8× [1 s tone + 0.5 s silence]
each (12 s, Python-generated, −6 dB). Silence holes = unambiguous time
markers (no correlation ambiguity like with continuous tones).
Protocol: record `morse440` fully, rewind to middle (counter!),
insert `morse880` (~5–8 s), stop. Note counter + mode.
Measurement: count/measure holes in NCH output (duplicate? intact?
shifted?) → reaction compensation (overlap re-recording) proven
or dead. Finding on DICT0016 so far: S2-onset content unique (no
duplicate ≥ 96 frames found).
