//! ds2tool: decode/inspect DSS/DS2 (Grundig Digta and others).
//!
//! Safety: default output lands in the working directory, never next to the
//! input (no accidental overwriting of reference files).

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ds2tool", about = "Decode and inspect DSS/DS2")]
struct Cli {
    /// Input file (.ds2/.dss)
    input: PathBuf,
    /// Output WAV (default: ./<stem>.wav)
    #[arg(short = 'O', long)]
    output_file: Option<PathBuf>,
    /// Inspect only (container, codec, frames, duration), no decoding
    #[arg(long)]
    info: bool,
}

fn default_output(input: &std::path::Path) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    PathBuf::from(format!("{stem}.wav"))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let data = std::fs::read(&cli.input)?;
    let layout = grundig_ds2::detect(&data).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Container: audio_start=0x{:x} format_type={} codec={:?} ({} Hz)",
        layout.audio_start,
        layout.format_type,
        layout.codec,
        layout.codec.native_rate()
    );
    if cli.info {
        return Ok(());
    }
    let buf = grundig_ds2::decode(&data).map_err(|e| anyhow::anyhow!("{e}"))?;
    let out = cli.output_file.unwrap_or_else(|| default_output(&cli.input));
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: buf.native_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&out, spec)?;
    for s in &buf.samples {
        w.write_sample(s.clamp(-32768.0, 32767.0) as i16)?;
    }
    w.finalize()?;
    println!(
        "-> {} ({} samples, {} Hz)",
        out.display(),
        buf.samples.len(),
        buf.native_rate
    );
    Ok(())
}
