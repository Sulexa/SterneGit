use std::io::Cursor;

use anyhow::{Context as _, Result, bail};
use symphonia::core::{
    codecs::audio::{AudioDecoderOptions, well_known as codec},
    errors::Error as DecodeError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

const BLOCK_SECONDS: f64 = 0.4;
const HOP_SECONDS: f64 = 0.1;
const ABSOLUTE_GATE: f64 = -70.;
const RELATIVE_GATE: f64 = -10.;
const LOUDNESS_OFFSET: f64 = -0.691;

pub(crate) struct Sound {
    pub(crate) rate: u32,
    pub(crate) channels: usize,
    pub(crate) samples: Vec<i16>,
    pub(crate) codec: &'static str,
    pub(crate) bits: Option<u32>,
    pub(crate) bytes: usize,
    pub(crate) loudness: Option<f64>,
    pub(crate) peak: f64,
}

impl Sound {
    pub(crate) fn decode(bytes: Vec<u8>, extension: Option<&str>) -> Result<Self> {
        let size = bytes.len();
        let mut hint = Hint::new();
        if let Some(extension) = extension {
            hint.with_extension(extension);
        }
        let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
        let mut format = symphonia::default::get_probe()
            .probe(&hint, stream, FormatOptions::default(), MetadataOptions::default())
            .context("FastGit can't read this kind of sound")?;
        let track = format
            .default_track(TrackType::Audio)
            .context("This file has no sound in it")?;
        let track_id = track.id;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .context("This file has no sound in it")?
            .clone();
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .context("FastGit can't decode this kind of sound")?;
        let mut samples: Vec<i16> = Vec::new();
        let mut packet_samples: Vec<i16> = Vec::new();
        let mut rate = params.sample_rate.unwrap_or(0);
        let mut channels = params.channels.as_ref().map_or(0, |channels| channels.count());
        loop {
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => break,
                Err(DecodeError::IoError(_)) => break,
                Err(error) => return Err(error).context("Couldn't read this sound"),
            };
            if packet.track_id != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    rate = decoded.spec().rate();
                    channels = decoded.spec().channels().count();
                    decoded.copy_to_vec_interleaved(&mut packet_samples);
                    samples.extend_from_slice(&packet_samples);
                }
                Err(DecodeError::DecodeError(_)) => {}
                Err(error) => return Err(error).context("Couldn't decode this sound"),
            }
        }
        if rate == 0 || channels == 0 || samples.is_empty() {
            bail!("This file has no sound in it");
        }
        let mut sound = Self {
            rate,
            channels,
            samples,
            codec: codec_name(params.codec),
            bits: params.bits_per_sample,
            bytes: size,
            loudness: None,
            peak: 0.,
        };
        sound.loudness = sound.integrated_loudness();
        sound.peak = sound.sample_peak();
        Ok(sound)
    }

    pub(crate) fn from_pcm(rate: u32, channels: usize, samples: Vec<i16>) -> Self {
        let mut sound = Self {
            rate,
            channels,
            samples,
            codec: "PCM",
            bits: Some(16),
            bytes: 0,
            loudness: None,
            peak: 0.,
        };
        sound.loudness = sound.integrated_loudness();
        sound.peak = sound.sample_peak();
        sound
    }

    pub(crate) fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1)
    }

    pub(crate) fn duration(&self) -> f64 {
        self.frames() as f64 / f64::from(self.rate.max(1))
    }

    pub(crate) fn value(&self, frame: usize, channel: usize) -> f32 {
        let at = frame * self.channels + channel.min(self.channels - 1);
        self.samples.get(at).map_or(0., |sample| f32::from(*sample) / 32768.)
    }

    pub(crate) fn mono(&self, frame: usize) -> f32 {
        (0..self.channels).map(|channel| self.value(frame, channel)).sum::<f32>() / self.channels as f32
    }

    pub(crate) fn frame_at(&self, seconds: f64) -> usize {
        (seconds.max(0.) * f64::from(self.rate)) as usize
    }

    pub(crate) fn bit_rate(&self) -> Option<u32> {
        let duration = self.duration();
        (duration > 0. && self.bytes > 0).then(|| (self.bytes as f64 * 8. / duration) as u32)
    }

    pub(crate) fn channel_name(&self) -> String {
        match self.channels {
            1 => "Mono".to_owned(),
            2 => "Stereo".to_owned(),
            count => format!("{count} channels"),
        }
    }

    pub(crate) fn codec_name(&self) -> String {
        match (self.codec, self.bits) {
            ("PCM", Some(bits)) => format!("PCM, {bits} bits"),
            (codec, _) => codec.to_owned(),
        }
    }

    pub(crate) fn levels(&self, from: f64, to: f64, columns: usize) -> Vec<Option<(f32, f32)>> {
        let span = (to - from).max(f64::EPSILON);
        let frames = self.frames();
        (0..columns)
            .map(|column| {
                let start = from + span * column as f64 / columns as f64;
                let end = from + span * (column + 1) as f64 / columns as f64;
                let first = self.frame_at(start);
                if first >= frames {
                    return None;
                }
                let last = self.frame_at(end).clamp(first + 1, frames);
                let step = ((last - first) / 256).max(1);
                let mut low = 0f32;
                let mut high = 0f32;
                for frame in (first..last).step_by(step) {
                    let value = self.mono(frame);
                    low = low.min(value);
                    high = high.max(value);
                }
                Some((low, high))
            })
            .collect()
    }

    fn sample_peak(&self) -> f64 {
        let peak = self
            .samples
            .iter()
            .map(|sample| i32::from(*sample).unsigned_abs())
            .max()
            .unwrap_or(0);
        decibels(f64::from(peak) / 32768.)
    }

    fn integrated_loudness(&self) -> Option<f64> {
        let rate = f64::from(self.rate);
        let block = (BLOCK_SECONDS * rate) as usize;
        let hop = (HOP_SECONDS * rate) as usize;
        let frames = self.frames();
        if block == 0 || hop == 0 || frames < block {
            return None;
        }
        let mut squares = vec![0f64; frames];
        for channel in 0..self.channels.min(2) {
            let mut filter = KWeighting::new(rate);
            for (frame, square) in squares.iter_mut().enumerate() {
                let value = filter.next(f64::from(self.value(frame, channel)));
                *square += value * value;
            }
        }
        let mut sums = vec![0f64; frames + 1];
        for (frame, square) in squares.iter().enumerate() {
            sums[frame + 1] = sums[frame] + square;
        }
        let blocks: Vec<f64> = (0..=(frames - block) / hop)
            .map(|ix| (sums[ix * hop + block] - sums[ix * hop]) / block as f64)
            .collect();
        let loudness = |power: f64| LOUDNESS_OFFSET + 10. * power.log10();
        let above = |threshold: f64| -> Vec<f64> {
            blocks
                .iter()
                .copied()
                .filter(|power| *power > 0. && loudness(*power) > threshold)
                .collect()
        };
        let audible = above(ABSOLUTE_GATE);
        if audible.is_empty() {
            return None;
        }
        let relative = loudness(audible.iter().sum::<f64>() / audible.len() as f64) + RELATIVE_GATE;
        let gated = above(ABSOLUTE_GATE.max(relative));
        (!gated.is_empty()).then(|| loudness(gated.iter().sum::<f64>() / gated.len() as f64))
    }
}

pub(crate) fn decibels(amplitude: f64) -> f64 {
    if amplitude <= 0. {
        f64::NEG_INFINITY
    } else {
        20. * amplitude.log10()
    }
}

fn codec_name(id: symphonia::core::codecs::audio::AudioCodecId) -> &'static str {
    match id {
        codec::CODEC_ID_VORBIS => "Vorbis",
        codec::CODEC_ID_MP3 => "MP3",
        codec::CODEC_ID_AAC => "AAC",
        codec::CODEC_ID_FLAC => "FLAC",
        codec::CODEC_ID_ALAC => "ALAC",
        _ if id < codec::CODEC_ID_VORBIS => "PCM",
        _ => "Other",
    }
}

struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    state: [f64; 2],
}

impl Biquad {
    fn next(&mut self, input: f64) -> f64 {
        let output = self.b[0] * input + self.state[0];
        self.state[0] = self.b[1] * input - self.a[0] * output + self.state[1];
        self.state[1] = self.b[2] * input - self.a[1] * output;
        output
    }
}

struct KWeighting {
    shelf: Biquad,
    high_pass: Biquad,
}

impl KWeighting {
    fn new(rate: f64) -> Self {
        let pi = std::f64::consts::PI;
        let (f0, gain, q) = (1681.974450955533, 3.999843853973347, 0.7071752369554196);
        let k = (pi * f0 / rate).tan();
        let vh = 10f64.powf(gain / 20.);
        let vb = vh.powf(0.4996667741545416);
        let a0 = 1. + k / q + k * k;
        let shelf = Biquad {
            b: [
                (vh + vb * k / q + k * k) / a0,
                2. * (k * k - vh) / a0,
                (vh - vb * k / q + k * k) / a0,
            ],
            a: [2. * (k * k - 1.) / a0, (1. - k / q + k * k) / a0],
            state: [0.; 2],
        };
        let (f0, q) = (38.13547087602444, 0.5003270373238773);
        let k = (pi * f0 / rate).tan();
        let a0 = 1. + k / q + k * k;
        let high_pass = Biquad {
            b: [1., -2., 1.],
            a: [2. * (k * k - 1.) / a0, (1. - k / q + k * k) / a0],
            state: [0.; 2],
        };
        Self { shelf, high_pass }
    }

    fn next(&mut self, input: f64) -> f64 {
        self.high_pass.next(self.shelf.next(input))
    }
}

#[cfg(test)]
mod tests {
    use super::Sound;

    fn tone(rate: u32, seconds: f64, amplitude: f64) -> Vec<i16> {
        (0..(f64::from(rate) * seconds) as usize)
            .flat_map(|frame| {
                let value = (amplitude * (2. * std::f64::consts::PI * 997. * frame as f64 / f64::from(rate)).sin() * 32767.) as i16;
                [value, value]
            })
            .collect()
    }

    #[test]
    fn a_full_scale_stereo_tone_peaks_near_zero_and_measures_about_zero_lufs() {
        let sound = Sound::from_pcm(48_000, 2, tone(48_000, 2., 1.));
        assert!(sound.peak > -0.1 && sound.peak <= 0., "{}", sound.peak);
        let loudness = sound.loudness.unwrap();
        assert!(loudness.abs() < 0.2, "{loudness}");
    }

    #[test]
    fn halving_the_amplitude_takes_six_decibels_off() {
        let full = Sound::from_pcm(48_000, 2, tone(48_000, 2., 1.)).loudness.unwrap();
        let half = Sound::from_pcm(48_000, 2, tone(48_000, 2., 0.5)).loudness.unwrap();
        assert!((full - half - 6.02).abs() < 0.1, "{full} {half}");
    }

    #[test]
    fn silence_has_no_loudness_and_levels_end_with_the_sound() {
        let silence = Sound::from_pcm(8_000, 1, vec![0; 8_000]);
        assert_eq!(silence.loudness, None);
        assert_eq!(silence.duration(), 1.);
        let levels = silence.levels(0., 2., 4);
        assert!(levels[0].is_some() && levels[1].is_some());
        assert!(levels[2].is_none() && levels[3].is_none());
    }
}
