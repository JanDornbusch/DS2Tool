# DssPlay as second oracle (loopback capture, 06.10.)

Motivation: no runnable Grundig converter found (the player does not
save). Instead: observe vendor-player behavior at gaps/markers.
Measurement accuracy: STRUCTURE (what plays when: skip/silence/
content/order), no LSBs (loopback + resampling 44.1 kHz).

## Setup (Windows)

- Play the files in `example/DssPlay.exe` in order, one recording per
  file (alternatively one session with ~3 s pause between):
  1. `testdata/Test3/DICT0173.DSS` (control — must match, else the chain is invalid!)
  2. `testdata/Test1/DICT0163.DS2` (QP gap-2)
  3. `testdata/Test3/DICT0174.DSS` (SP insert)
  4. `probes/P2a.DSS` (synthetic marker — does the player accept it?)
- Capture: stereo mix / WASAPI loopback (Audacity), 44100 Hz / 16 bit /
  stereo, fixed system volume, all enhancements/AGC OFF.
- Deliver: WAVs + play order + player error messages (even
  refusals are data!).

## Reading guide (registered beforehand)

- Control ≠ close to NCH → discard chain, set up again.
- 0163 gap-2 like NCH (loud speech 20–24 s) → gap-2 is a real demux
  question, NCH rehabilitated.
- 0163 gap-2 silent/different → NCH-artifact suspicion strengthened
  (gap rendered as silence or similar), re-evaluate reference model.
- P2a refusal → synthetic files outside player tolerance
  (adjust probe design).
