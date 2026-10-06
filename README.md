# DS2-Codec

Decode DSS/DS2 dictation audio (Grundig Digta and others), aiming for an
**FFmpeg patch**.

- `crates/grundig` — our own work: container detection + normalization
  (Grundig wrapper → standard layout: magics `\x07ds2`/`\x07dss`/`\x08ds2`,
  audio at `0xE00`/`0x1000`, section split + skip-forward for SP),
  then the codec core from `crates/upstream`.
- `crates/ds2tool` — CLI (`decode`, `--info`).
- `crates/upstream` — vendored `dss-codec` (MIT, rev `c38d319`, see
  `ATTRIBUTION.md`), unchanged.
- `testdata/` — lab tones (generated with sox) + device recordings of them +
  reference WAVs + mapping (`testdata/README.md`). No voices,
  no third-party audio.
- `docs/GRUNDIG-FORMAT.md` — the measured format (§§1–21).
- `probes/` — synthetic probes (resume/marker/gap) + protocols,
  voice-free, suitable as FATE templates.

## Build

```bash
cargo build --release   # binary: target/release/ds2tool
cargo test              # 9 tests, all green
```

## Use

```bash
ds2tool file.DS2 -O out.wav   # decode (native rate: 11025/16000 Hz, mono)
ds2tool file.DSS -O out.wav
ds2tool file.DS2 --info       # inspect only: codec, frames, duration
```

Without `-O`, the output lands next to the input file. Gaps are rendered
the player way (skipped, see `docs/GRUNDIG-FORMAT.md` §19).

## Status

Player-validated: the reference decode is what the vendor player plays
(loopback correlation 0.81–0.99 at every resume point); all earlier
NCH mismatches were NCH-side. SP solved via section split + skip-forward
(lengths exact, remaining errors are transients only).

## License

Unlicense (public domain, see `UNLICENSE`) for our own work
(`crates/grundig`, `crates/ds2tool` — also stated in their `Cargo.toml`).
Upstream core under MIT (`crates/upstream`, see there + `ATTRIBUTION.md`).
So this is a split license: Unlicense + MIT — MIT forces nothing on us
(permissive, only the notice requirement, fulfilled); no copyleft involved.
