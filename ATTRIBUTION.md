# Attribution

- Codec core + tables + algorithms: **dss-codec** by Kieran Hirpara,
  MIT-licensed (`https://github.com/hirparak/dss-codec`), at rev
  `c38d319`, vendored under `crates/upstream/` (unchanged, including
  `LICENSE-MIT-upstream`). Reference Python (`ds2decode.py` etc.) was only
  used for analysis from a separate checkout (not in this repo).
- Ticket analysis: FFmpeg Trac #6091 (community reverse engineering).
- Our own work in this repo: Grundig container layer, demux rules,
  test matrix + findings docs. No tables duplicated.

## No-snooping decision (2026-10-06)

**No reversing of proprietary binaries** (Grundig/Olympus/NCH DLLs):
- Benefit: low — the codec is already fully reversed (ticket +
  reference, verified bit-exact on our matrix); the rest (gap-2) is
  a demux/segmentation question, answerable from our own controlled
  recordings.
- Risk: high — knowledge from disassembly would contaminate a later
  FFmpeg upstream contribution (clean-room principle).
- Instead: behavioral oracles (NCH outputs as reference WAVs) +
  our own test matrix. Black box only.
