//! Grundig/Olympus DSS/DS2 container layer.
//!
//! Docs: `docs/GRUNDIG-FORMAT.md` (findings), `testdata/README.md` (matrix).
//!
//! Job of this crate: detect + normalize the container (ticket layout:
//! 0x600 header, audio from 0x600 in 512-byte blocks with 6-byte heads), then
//! the codec core from [`dss_codec`] (vendored, MIT, see `ATTRIBUTION.md`).
//! Codec tables/algorithms are NOT duplicated (single source:
//! `crates/upstream`).

use thiserror::Error;

pub const FILE_HEADER_SIZE: usize = 0x600;
pub const BLOCK_SIZE: usize = 512;
pub const BLOCK_HEADER_SIZE: usize = 6;

/// Codec, from byte 4 of the first frame-carrying block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// DS2 QP, 16000 Hz, 448-bit frames (format type 6).
    Ds2Qp,
    /// DS2 SP, 12000 Hz, 328-bit frames (format type 0/1).
    Ds2Sp,
    /// DSS SP, 11025 Hz, 264-bit frames (format type 0, `.dss`).
    DssSp,
}

impl Codec {
    pub fn native_rate(self) -> u32 {
        match self {
            Codec::Ds2Qp => 16000,
            Codec::Ds2Sp => 12000,
            Codec::DssSp => 11025,
        }
    }

    pub fn samples_per_frame(self) -> usize {
        match self {
            Codec::Ds2Qp => 256,
            Codec::Ds2Sp => 288,
            Codec::DssSp => 264,
        }
    }
}

/// Detected audio layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Offset of the first audio block (512-aligned, e.g. 0xE00 with
    /// Grundig wrapper, 0x600 with ticket layout).
    pub audio_start: usize,
    /// Format type from byte 4 (0/1 = SP, 6 = QP, 7 = QP7 records).
    pub format_type: u8,
    /// Derived codec (7 treated like 6, see docs).
    pub codec: Codec,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("too short for container detection ({0} bytes)")]
    TooShort(usize),
    #[error("not a DSS/DS2 container (magic: {0:02x?})")]
    BadMagic([u8; 4]),
    #[error("no audio raster found")]
    NoAudio,
    #[error("unsupported format type: {0}")]
    UnsupportedFormat(u8),
    #[error(transparent)]
    Codec(#[from] dss_codec::error::DecodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// 6-byte block head plausible? `[b0, b1, fc, 0xFF, fmt, 0xFF]` with fc > 0.
/// b0: 0x0F (QP / SP without swap) or 0x8F (SP with swap init, bit 7).
/// fc = 0 (pause/hole blocks) does NOT count (carry no frames).
fn is_audio_header(h: &[u8]) -> bool {
    h.len() >= BLOCK_HEADER_SIZE
        && (h[0] == 0x0f || h[0] == 0x8f)
        && h[3] == 0xff
        && h[5] == 0xff
        && h[2] > 0
        && matches!(h[4], 0 | 1 | 6 | 7)
}

/// Detect container: check magic, scan audio raster on 512 grid from 0x600
/// (first head + following block valid, else end of file).
/// Returns the layout with audio start (Grundig wrapper is skipped).
pub fn detect(data: &[u8]) -> Result<Layout, Error> {
    if data.len() < FILE_HEADER_SIZE + BLOCK_HEADER_SIZE {
        return Err(Error::TooShort(data.len()));
    }
    let magic: [u8; 4] = data[0..4].try_into().unwrap();
    if &magic[1..4] != b"dss" && &magic[1..4] != b"ds2" {
        return Err(Error::BadMagic(magic));
    }
    let mut audio_start = None;
    let mut off = FILE_HEADER_SIZE;
    while off + BLOCK_HEADER_SIZE <= data.len() {
        if is_audio_header(&data[off..off + BLOCK_HEADER_SIZE]) {
            let next = off + BLOCK_SIZE;
            let next_ok = next + BLOCK_HEADER_SIZE > data.len()
                || is_audio_header(&data[next..next + BLOCK_HEADER_SIZE])
                // Tolerate pause/hole blocks (fc = 0, otherwise same shape):
                || (next + BLOCK_HEADER_SIZE <= data.len()
                    && data[next + 3] == 0xff
                    && data[next + 5] == 0xff);
            if next_ok {
                audio_start = Some(off);
                break;
            }
        }
        off += BLOCK_SIZE;
    }
    let audio_start = audio_start.ok_or(Error::NoAudio)?;
    // Format from first frame-carrying block.
    let format_type = data[audio_start + 4];
    let codec = match format_type {
        6 | 7 => Codec::Ds2Qp,
        0 | 1 if magic[1..4] == *b"ds2" => Codec::Ds2Sp,
        0 | 1 => Codec::DssSp,
        _ => return Err(Error::UnsupportedFormat(format_type)),
    };
    Ok(Layout {
        audio_start,
        format_type,
        codec,
    })
}

/// Normalize to ticket layout: 0x600 header (magic patched to
/// `\x03ds2`/`\x03dss` if needed) + audio blocks from `audio_start` unchanged behind it.
/// Empty blocks (fc = 0) stay included (codec core segments itself).
pub fn normalize(data: &[u8]) -> Result<Vec<u8>, Error> {
    let layout = detect(data)?;
    let mut header = data[0..FILE_HEADER_SIZE].to_vec();
    match &header[0..4] {
        b"\x07ds2" => header[0..4].copy_from_slice(b"\x03ds2"),
        b"\x07dss" => header[0..4].copy_from_slice(b"\x03dss"),
        _ => {}
    }
    let mut out = header;
    out.extend_from_slice(&data[layout.audio_start..]);
    Ok(out)
}

/// File → normalize → codec core. Yields f64 mono + native rate.
///
/// SP (DSS) goes through section split (see `decode_sp_sections`), everything
/// else through the upstream core on normalized bytes.
pub fn decode(data: &[u8]) -> Result<dss_codec::AudioBuffer, Error> {
    let layout = detect(data)?;
    if layout.codec == Codec::DssSp {
        decode_sp_sections(data, &layout)
    } else {
        Ok(dss_codec::decode_to_buffer(&normalize(data)?)?)
    }
}

/// File path → decode.
pub fn decode_file(path: &std::path::Path) -> Result<dss_codec::AudioBuffer, Error> {
    Ok(decode(&std::fs::read(path)?)?)
}

/// SP demux with anchor + skip-forward + reset (validated: TestSP/Test3/Test4,
/// see docs §6/§10).
///
/// Model: each block with `fc > 0` anchors its frames at
/// `Payload + max(0, 2·b1 + 2·swap − 6)` (countdown rule, computed from device
/// data). Anchor BEFORE run position: keep reading
/// (overlap, normal case — clean files are exactly sequential this way).
/// If BEHIND (pad tail, gaps, resume heads, marker blocks):
/// jump + swap/state/decoder fresh (resume-head byte0-bit7 as init).
/// `fc == 0` blocks contribute nothing. Frame count exact from header sums
/// (lengths match NCH); rest padding is never consumed.
pub fn decode_sp_sections(
    data: &[u8],
    layout: &Layout,
) -> Result<dss_codec::AudioBuffer, Error> {
    use dss_codec::codec::dss_sp::DssSpDecoder;

    let mut off = layout.audio_start;
    let mut blocks: Vec<(usize, [u8; 6])> = vec![];
    while off + BLOCK_SIZE <= data.len() {
        let h = &data[off..off + BLOCK_HEADER_SIZE];
        let mut hb = [0u8; 6];
        hb.copy_from_slice(h);
        blocks.push((off, hb));
        off += BLOCK_SIZE;
    }

    let mut samples = vec![];
    let mut pos = 0usize; // Laufposition im verketteten Payload-Raum
    let mut swap = false;
    let mut swap_byte = 0u8;
    let mut dec = DssSpDecoder::new();
    // Concatenated payloads of all fc>0 blocks (stable anchor base,
    // zero blocks occupy no space).
    let mut seq: Vec<u8> = vec![];
    let mut seq_base: Vec<usize> = vec![0; blocks.len()];
    for (i, (boff, h)) in blocks.iter().enumerate() {
        seq_base[i] = seq.len();
        if h[2] > 0 {
            seq.extend_from_slice(&data[boff + BLOCK_HEADER_SIZE..boff + BLOCK_SIZE]);
        }
    }
    for (i, (_boff, h)) in blocks.iter().enumerate() {
        let fc = h[2] as usize;
        if fc == 0 {
            continue;
        }
        let cont = (h[1] as usize) * 2 + 2 * ((h[0] as usize >> 7) & 1);
        let cont = cont.saturating_sub(BLOCK_HEADER_SIZE);
        let anchor = seq_base[i] + cont;
        if anchor > pos {
            pos = anchor;
            swap = (h[0] >> 7) & 1 == 1;
            swap_byte = 0;
            dec = DssSpDecoder::new();
        }
        for _ in 0..fc {
            if pos + 42 > seq.len() {
                break;
            }
            let mut pkt = [0u8; 43];
            if swap {
                if pos + 40 > seq.len() {
                    break;
                }
                pkt[3..43].copy_from_slice(&seq[pos..pos + 40]);
                pos += 40;
                let mut k = 0;
                while k < 40 {
                    pkt[k] = pkt[k + 4];
                    k += 2;
                }
                pkt[40] = 0;
                pkt[1] = swap_byte;
            } else {
                pkt[..42].copy_from_slice(&seq[pos..pos + 42]);
                pos += 42;
                swap_byte = pkt[40];
            }
            pkt[40] = 0;
            swap = !swap;
            samples.extend(dec.decode_frame(&pkt[..42]));
        }
    }
    Ok(dss_codec::AudioBuffer {
        samples: samples.into_iter().map(|v| v as f64).collect(),
        native_rate: Codec::DssSp.native_rate(),
        format: dss_codec::demux::AudioFormat::DssSp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata")
            .join(name)
    }

    fn read(name: &str) -> Vec<u8> {
        std::fs::read(td(name)).unwrap()
    }

    #[test]
    fn qp_audio_startet_bei_0xe00() {
        for f in [
            "Test1/DICT0162.DS2",
            "Test1/DICT0163.DS2",
            "Test2/DICT0164.DS2",
            "Test2/DICT0166.DS2",
            "Test2/DICT0167.DS2",
            "Test2/DICT0168.DS2",
            "Test2/DICT0169.DS2",
            "Test2/DICT0170.DS2",
        ] {
            let layout = detect(&read(f)).unwrap();
            assert_eq!(layout.audio_start, 0xe00, "{f}");
            assert_eq!(layout.codec, Codec::Ds2Qp, "{f}");
        }
    }

    #[test]
    fn sp_audio_startet_bei_0xe00() {
        for f in ["TestSP/DICT0172.DSS"] {
            let layout = detect(&read(f)).unwrap();
            assert_eq!(layout.audio_start, 0xe00, "{f}");
            assert_eq!(layout.codec, Codec::DssSp, "{f}");
        }
    }

    /// data chunk exact (length field; ignore trailing chunks).
    /// Returns (rate, samples). All reference WAVs: 16-bit mono.
    fn wav_data(path: &std::path::Path) -> (f64, Vec<i16>) {
        let wd = std::fs::read(path).unwrap();
        let rate = u32::from_le_bytes(wd[24..28].try_into().unwrap()) as f64;
        let pos = wd.windows(4).position(|w| w == b"data").unwrap();
        let n =
            u32::from_le_bytes(wd[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let raw = &wd[pos + 8..pos + 8 + n];
        let mut v = Vec::with_capacity(n / 2);
        for i in 0..n / 2 {
            v.push(i16::from_le_bytes([raw[2 * i], raw[2 * i + 1]]));
        }
        (rate, v)
    }

    /// Frame sum × samples/frame must hit WAV duration (demux rule).
    #[test]
    fn qp_frame_summe_trifft_dauer() {
        for (f, rate) in [
            ("Test1/DICT0162.DS2", 16000),
            ("Test1/DICT0163.DS2", 16000),
            ("Test2/DICT0164.DS2", 16000),
            ("Test2/DICT0166.DS2", 16000),
            ("Test2/DICT0167.DS2", 16000),
            ("Test2/DICT0168.DS2", 16000),
            ("Test2/DICT0169.DS2", 16000),
            ("Test2/DICT0170.DS2", 16000),
        ] {
            let d = read(f);
            let layout = detect(&d).unwrap();
            let mut frames = 0u64;
            let mut off = layout.audio_start;
            while off + BLOCK_HEADER_SIZE <= d.len() {
                frames += d[off + 2] as u64;
                off += BLOCK_SIZE;
            }
            let expect = frames as f64 * 256.0 / rate as f64;
            let (wrate, samples) = wav_data(&td(&f.replace(".DS2", ".wav")));
            let got = samples.len() as f64 / wrate;
            assert!((expect - got).abs() < 0.01, "{f}: {expect} vs {got}");
        }
    }

    #[test]
    fn sp_frame_summe_trifft_dauer() {
        for f in ["TestSP/DICT0172.DSS"] {
            let d = read(f);
            let layout = detect(&d).unwrap();
            let mut frames = 0u64;
            let mut off = layout.audio_start;
            while off + BLOCK_HEADER_SIZE <= d.len() {
                frames += d[off + 2] as u64;
                off += BLOCK_SIZE;
            }
            let expect = frames as f64 * 264.0 / 11025.0;
            let (wrate, samples) = wav_data(&td(&f.replace(".DSS", ".wav")));
            let got = samples.len() as f64 / wrate;
            assert!((expect - got).abs() < 0.01, "{f}: {expect} vs {got}");
        }
    }

    /// 0162 (stop+go, clean) must decode bit-near (max ±8 LSB).
    #[test]
    fn decode_0162_bitnah() {
        let buf = decode_file(&td("Test1/DICT0162.DS2")).unwrap();
        assert_eq!(buf.native_rate, 16000);
        let (_, reference) = wav_data(&td("Test1/DICT0162.wav"));
        assert_eq!(buf.samples.len(), reference.len());
        let mut maxdiff = 0i32;
        for (s, r) in buf.samples.iter().zip(reference.iter()) {
            let a = (*s).clamp(-32768.0, 32767.0) as i32;
            maxdiff = maxdiff.max((a - *r as i32).abs());
        }
        assert!(maxdiff <= 8, "maxdiff={maxdiff}");
    }

    /// 0163 duration: decoder length must hit frame sum (gap-2 content
    /// closed (see docs §20) — length only, no sample comparison).
    #[test]
    fn decode_0163_laenge() {
        let buf = decode_file(&td("Test1/DICT0163.DS2")).unwrap();
        assert_eq!(buf.samples.len(), 1894 * 256);
    }

    /// Pearson correlation (test helper, no deps).
    fn pearson(a: &[i16], b: &[i16]) -> f64 {
        assert_eq!(a.len(), b.len());
        let n = a.len() as f64;
        let (mut sa, mut sb) = (0.0, 0.0);
        for (x, y) in a.iter().zip(b.iter()) {
            sa += *x as f64;
            sb += *y as f64;
        }
        let (ma, mb) = (sa / n, sb / n);
        let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
        for (x, y) in a.iter().zip(b.iter()) {
            let (da, db) = (*x as f64 - ma, *y as f64 - mb);
            cov += da * db;
            va += da * da;
            vb += db * db;
        }
        cov / (va * vb).sqrt()
    }

    fn wav_i16(path: &std::path::Path) -> Vec<i16> {
        let (_, v) = wav_data(path);
        v
    }

    /// SP: decode length exact = frame sum × 264 (catches over/under-decoding).
    #[test]
    fn sp_decode_laenge_exakt() {
        for f in [
                        "TestSP/DICT0172.DSS",
            "Test3/DICT0173.DSS",
            "Test3/DICT0174.DSS",
            "Test3/DICT0175.DSS",
            "Test4/DICT0177.DSS",
            "Test4/DICT0178.DSS",
        ] {
            let d = read(f);
            let layout = detect(&d).unwrap();
            let mut frames = 0usize;
            let mut off = layout.audio_start;
            while off + BLOCK_HEADER_SIZE <= d.len() {
                frames += d[off + 2] as usize;
                off += BLOCK_SIZE;
            }
            let buf = decode_file(&td(f)).unwrap();
            assert_eq!(buf.native_rate, 11025);
            assert_eq!(buf.samples.len(), frames * 264, "{f}");
        }
    }

    /// SP clean: bit-near (start transient tolerated only).
    #[test]
    fn sp_decode_sauber_bitnah() {
        for (f, bound) in [
                        ("TestSP/DICT0172.DSS", 8),
            ("Test3/DICT0173.DSS", 8),
        ] {
            let buf = decode_file(&td(f)).unwrap();
            let reference = wav_i16(&td(&f.replace(".DSS", ".wav")));
            assert_eq!(buf.samples.len(), reference.len(), "{f}");
            let mut maxdiff = 0i32;
            for (s, r) in buf.samples.iter().zip(reference.iter()) {
                let a = (*s).clamp(-32768.0, 32767.0) as i32;
                maxdiff = maxdiff.max((a - *r as i32).abs());
            }
            assert!(maxdiff <= bound, "{f}: maxdiff={maxdiff}");
        }
    }

    /// SP with gaps/inserts: solve tails (corr ≥ 0.99; transients at
    /// jump points explicitly tolerated, see docs §6/§10).
    #[test]
    fn sp_decode_tails() {
        for f in [
            "Test3/DICT0174.DSS",
            "Test3/DICT0175.DSS",
            "Test4/DICT0177.DSS",
            "Test4/DICT0178.DSS",
        ] {
            let buf = decode_file(&td(f)).unwrap();
            let reference = wav_i16(&td(&f.replace(".DSS", ".wav")));
            assert_eq!(buf.samples.len(), reference.len(), "{f}");
            let mine: Vec<i16> = buf
                .samples
                .iter()
                .map(|s| s.clamp(-32768.0, 32767.0) as i16)
                .collect();
            let c = pearson(&mine, &reference);
            assert!(c >= 0.99, "{f}: corr={c}");
        }
    }
}
