use std::{
    convert::TryInto,
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Error as AnyHow;
use anyhow::{bail, ensure};
use clap::{arg_enum, AppSettings};
use hound::{SampleFormat, WavSpec, WavWriter};
use num::NumCast;
use rodio::{buffer::SamplesBuffer, decoder::Decoder, Sink, Source};
use structopt::StructOpt;

const HELP: &str = r#"
           ||||||||||
           ||||||||||
           ||||||||||
           ||||||||||
           ||||||||||
           ||||||||||
           ||||||||||
           ||||||||||
  ╔═══════════════════════════╗
  ║                           ║
  VvVvVvVvVvVvVvVvVvVvVvVvVvVvV
                ♪
  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
"#;

#[derive(StructOpt)]
#[structopt(name = "CRUSH", about = HELP, setting = AppSettings::ArgRequiredElseHelp)]
struct Opts {
    /// The input file to CRUSH
    #[structopt(short, long, parse(from_os_str))]
    input: PathBuf,

    /// The output CRUSHED file. Supported formats: WAV
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Play the CRUSHED sound
    #[structopt(short, long)]
    play: bool,

    /// Target bit depth. Default: 16-bit depth.
    #[structopt(short, long)]
    bit_depth: Option<u8>,

    /// Target sample rate. Default: 44100 Hz
    #[structopt(short, long)]
    sample_rate: Option<u32>,

    /// Interpolation method for resampling. Available: Nearest, Linear. Default: Nearest
    #[structopt(long)]
    interpolation: Option<Interpolation>,
}

#[derive(Clone)]
struct Sound {
    channels: Vec<Channel>,
    sample_rate: u32,
}

impl Sound {
    fn to_source(&self) -> SamplesBuffer<i16> {
        let c = self.channels.len();

        let data: Vec<_> = (0..c * self.channels[0].samples.len())
            .map(|i| self.channels[i % c].samples[i / c])
            .collect();

        SamplesBuffer::new(
            self.channels.len().try_into().unwrap(),
            self.sample_rate,
            data,
        )
    }
}

#[derive(Clone)]
struct Channel {
    samples: Vec<i16>,
}

impl Sound {
    fn new<S: Iterator<Item = i16> + Source>(mut source: S) -> Self {
        let channels_count: usize = source.channels().try_into().unwrap();
        let samples: Vec<i16> = source.by_ref().collect();

        Self {
            channels: (0..channels_count)
                .map(|i| Channel {
                    samples: samples
                        .iter()
                        .skip(i)
                        .step_by(channels_count)
                        .copied()
                        .collect(),
                })
                .collect(),
            sample_rate: source.sample_rate(),
        }
    }
}

fn main() -> Result<(), AnyHow> {
    let opts = Opts::from_args();

    let sample_rate = opts.sample_rate.unwrap_or(44100);
    let bit_depth = opts.bit_depth.unwrap_or(16);
    let interpolation = opts.interpolation.unwrap_or(Interpolation::Nearest);

    let mut sound = Sound::new(Decoder::new(File::open(opts.input)?)?);

    ensure!(
        opts.output.is_some() || opts.play,
        "Either --output or --play must be specified"
    );

    ensure!(
        (1..=44100).contains(&sample_rate),
        "Sample rate must be between 1 and 44100 Hz inclusive"
    );

    ensure!(
        (1..=32).contains(&bit_depth),
        "Bit depth must be between 1 and 16 bits inclusive"
    );

    if bit_depth == 16 && sample_rate == 44100 {
        println!("Warning: Neither bit depth nor sample rate are being CRUSHED");
    }

    sound = resample(sound, sample_rate, interpolation);
    sound = requantize(sound, bit_depth);
    sound = resample(sound, 44100, interpolation);

    let play_sound = sound.clone();

    let sink = if opts.play {
        let sink = Sink::new(&rodio::default_output_device().unwrap());

        sink.append(play_sound.to_source().buffered());

        Some(sink)
    } else {
        None
    };

    if let Some(output) = opts.output {
        let extension = output
            .extension()
            .map(OsStr::to_str)
            .unwrap()
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "wav" => save_wav(&sound, &output)?,
            _ => bail!("Unsupported output format {}", extension),
        }
    }

    if let Some(sink) = sink {
        sink.sleep_until_end();
    }

    Ok(())
}

arg_enum! {
    #[derive(Clone, Copy, Debug)]
    enum Interpolation {
        Nearest,
        Linear,
    }
}

fn resample(sound: Sound, sample_rate: u32, interpolation: Interpolation) -> Sound {
    let n = sound.channels[0].samples.len();

    if n == 0 {
        return Sound {
            channels: sound.channels,
            sample_rate,
        };
    }

    let r = sample_rate as f64 / sound.sample_rate as f64;
    let q = 1.0 / r;
    let new_sample_count = (n as f64 * r).round() as usize;

    Sound {
        channels: sound
            .channels
            .iter()
            .map(|channel| Channel {
                samples: (0..new_sample_count)
                    .map(|i| {
                        let f = i as f64 * q;
                        lerp(&channel.samples, f, interpolation).round() as i16
                    })
                    .collect(),
            })
            .collect(),
        sample_rate,
    }
}

fn lerp<T: Copy + std::fmt::Debug + NumCast>(
    values: &[T],
    f: f64,
    interpolation: Interpolation,
) -> f64 {
    assert!(values.len() > 0);
    assert!(f >= 0.0);
    assert!(
        f <= values.len() as f64,
        "Lerp index {} out of range: 0..{}",
        f,
        values.len()
    );

    let x = f as usize;
    let y = (x + 1).min(values.len() - 1);
    let a = f.fract();

    match interpolation {
        Interpolation::Nearest => {
            if a < 0.5 {
                num::cast(values[x]).unwrap()
            } else {
                num::cast(values[y]).unwrap()
            }
        }
        Interpolation::Linear => {
            let xv: f64 = num::cast(values[x]).unwrap();
            let yv: f64 = num::cast(values[y]).unwrap();
            (1.0 - a) * xv + a * yv
        }
    }
}

fn requantize(sound: Sound, bit_depth: u8) -> Sound {
    Sound {
        channels: sound
            .channels
            .iter()
            .map(|channel| Channel {
                samples: channel
                    .samples
                    .iter()
                    .map(|&sample| requantize_sample(sample, bit_depth))
                    .collect(),
            })
            .collect(),
        sample_rate: sound.sample_rate,
    }
}

fn requantize_sample(sample: i16, bit_depth: u8) -> i16 {
    if bit_depth == 16 {
        return sample;
    }

    let hi_mask = !0 << (16 - bit_depth);
    let lo_mask = !hi_mask;
    let fill: i16 = if sample & (1 << (15 - bit_depth)) < 0 {
        0
    } else {
        i16::max_value()
    };

    (sample & hi_mask) | (fill & lo_mask)
}

fn save_wav<P: AsRef<Path>>(sound: &Sound, path: P) -> Result<(), AnyHow> {
    let spec = WavSpec {
        channels: sound.channels.len() as u16,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let n = sound.channels[0].samples.len();
    let mut i16_writer = writer.get_i16_writer(n as u32);

    for sample in sound.to_source() {
        i16_writer.write_sample(sample);
    }

    i16_writer.flush()?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lerp() {
        let arr = [1u16, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(lerp(&arr, 4.8, Interpolation::Nearest), 6.0);
        assert_eq!(lerp(&arr, 4.4, Interpolation::Nearest), 5.0);
        assert_eq!(lerp(&arr, 4.8, Interpolation::Linear), 5.8);
    }

    #[test]
    fn test_requantize() {
        assert_eq!(requantize_sample(-1, 1), i16::min_value());
        assert_eq!(requantize_sample(0, 1), i16::max_value());
        assert_eq!(requantize_sample(10, 8), 255);
        assert_eq!(requantize_sample(256, 8), 511);
    }
}
