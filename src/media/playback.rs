use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result, anyhow};
use cpal::{
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use super::Sound;

#[derive(Default)]
struct Deck {
    sounds: [Option<Arc<Sound>>; 2],
    heard: usize,
    position: f64,
    playing: bool,
    repeat: Option<(f64, f64)>,
    end: f64,
}

impl Deck {
    fn next(&mut self, step: f64, channel_count: usize, frame: &mut [f32]) {
        frame.fill(0.);
        if !self.playing {
            return;
        }
        if let Some((from, to)) = self.repeat
            && self.position >= to
        {
            self.position = from;
        }
        if self.position >= self.end {
            self.playing = false;
            self.position = self.end;
            return;
        }
        if let Some(sound) = &self.sounds[self.heard] {
            for (channel, value) in frame.iter_mut().enumerate().take(channel_count) {
                *value = sound.at(self.position, channel);
            }
        }
        self.position += step;
    }
}

pub(crate) struct Player {
    deck: Arc<Mutex<Deck>>,
    stream: Option<Stream>,
}

impl Player {
    pub(crate) fn new(before: Option<Arc<Sound>>, after: Option<Arc<Sound>>) -> Self {
        Self::lasting(before, after, 0.)
    }

    pub(crate) fn lasting(before: Option<Arc<Sound>>, after: Option<Arc<Sound>>, length: f64) -> Self {
        let end = [&before, &after]
            .into_iter()
            .flatten()
            .map(|sound| sound.duration())
            .fold(length, f64::max);
        let heard = usize::from(after.is_some());
        Self {
            deck: Arc::new(Mutex::new(Deck {
                sounds: [before, after],
                heard,
                end,
                ..Deck::default()
            })),
            stream: None,
        }
    }

    pub(crate) fn toggle(&mut self) -> Result<()> {
        if self.is_playing() {
            self.with_deck(|deck| deck.playing = false);
            if let Some(stream) = &self.stream {
                let _ = stream.pause();
            }
            return Ok(());
        }
        if self.stream.is_none() {
            self.stream = Some(open_stream(self.deck.clone())?);
        }
        self.with_deck(|deck| {
            if deck.position >= deck.end {
                deck.position = deck.repeat.map_or(0., |(from, _)| from);
            }
            deck.playing = true;
        });
        if let Some(stream) = &self.stream {
            stream.play().context("Couldn't start the sound output")?;
        }
        Ok(())
    }

    pub(crate) fn seek(&self, seconds: f64) {
        self.with_deck(|deck| deck.position = seconds.clamp(0., deck.end));
    }

    pub(crate) fn hear(&self, version: usize) {
        self.with_deck(|deck| {
            if deck.sounds[version].is_some() {
                deck.heard = version;
            }
        });
    }

    pub(crate) fn heard(&self) -> usize {
        self.with_deck(|deck| deck.heard)
    }

    pub(crate) fn repeat(&self, range: Option<(f64, f64)>) {
        self.with_deck(|deck| deck.repeat = range);
    }

    pub(crate) fn pause(&self) {
        self.with_deck(|deck| deck.playing = false);
        if let Some(stream) = &self.stream {
            let _ = stream.pause();
        }
    }

    pub(crate) fn position(&self) -> f64 {
        self.with_deck(|deck| deck.position)
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.with_deck(|deck| deck.playing)
    }

    fn with_deck<R>(&self, change: impl FnOnce(&mut Deck) -> R) -> R {
        let mut deck = self.deck.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        change(&mut deck)
    }
}

impl Sound {
    pub(crate) fn at(&self, seconds: f64, channel: usize) -> f32 {
        let position = seconds * f64::from(self.rate);
        let frame = position.floor() as usize;
        if frame + 1 >= self.frames() {
            return 0.;
        }
        let source = if channel < self.channels {
            channel
        } else if self.channels == 1 {
            0
        } else {
            return 0.;
        };
        let fraction = (position - frame as f64) as f32;
        let here = self.value(frame, source);
        here + (self.value(frame + 1, source) - here) * fraction
    }
}

fn open_stream(deck: Arc<Mutex<Deck>>) -> Result<Stream> {
    let device = cpal::default_host()
        .default_output_device()
        .context("There is no sound output to play to")?;
    let supported = device
        .default_output_config()
        .map_err(|error| anyhow!("Couldn't read the sound output's settings: {error}"))?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    match format {
        SampleFormat::F32 => build::<f32>(&device, config, deck),
        SampleFormat::I16 => build::<i16>(&device, config, deck),
        SampleFormat::I32 => build::<i32>(&device, config, deck),
        SampleFormat::U16 => build::<u16>(&device, config, deck),
        other => Err(anyhow!("SterneGit can't play to a {other} sound output")),
    }
}

fn build<T: SizedSample + FromSample<f32>>(device: &Device, config: StreamConfig, deck: Arc<Mutex<Deck>>) -> Result<Stream> {
    let step = 1. / f64::from(config.sample_rate);
    let channels = usize::from(config.channels).max(1);
    let mut frame = vec![0f32; channels];
    device
        .build_output_stream(
            config,
            move |out: &mut [T], _| {
                let Ok(mut deck) = deck.try_lock() else {
                    out.iter_mut().for_each(|sample| *sample = T::from_sample(0f32));
                    return;
                };
                for chunk in out.chunks_mut(channels) {
                    deck.next(step, channels, &mut frame);
                    for (sample, value) in chunk.iter_mut().zip(&frame) {
                        *sample = T::from_sample(*value);
                    }
                }
            },
            |_| {},
            None,
        )
        .map_err(|error| anyhow!("Couldn't open the sound output: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Deck, Player};
    use crate::media::Sound;

    fn ramp(rate: u32, frames: i16) -> Arc<Sound> {
        Arc::new(Sound::from_pcm(rate, 1, (0..frames).map(|frame| frame * 100).collect()))
    }

    #[test]
    fn switching_versions_keeps_the_place() {
        let before = ramp(10, 20);
        let after = Arc::new(Sound::from_pcm(10, 1, vec![-1000; 20]));
        let player = Player::new(Some(before), Some(after));
        assert_eq!(player.heard(), 1);
        player.seek(0.5);
        player.hear(0);
        assert_eq!(player.heard(), 0);
        assert_eq!(player.position(), 0.5);
    }

    #[test]
    fn the_deck_plays_the_heard_version_and_stops_at_the_end() {
        let mut deck = Deck {
            sounds: [Some(ramp(10, 10)), None],
            heard: 0,
            position: 0.2,
            playing: true,
            repeat: None,
            end: 1.,
        };
        let mut frame = [0f32; 2];
        deck.next(0.1, 2, &mut frame);
        assert_eq!(frame, [200. / 32768.; 2]);
        deck.position = 1.;
        deck.next(0.1, 2, &mut frame);
        assert!(!deck.playing);
        assert_eq!(frame, [0.; 2]);
    }

    #[test]
    fn a_repeated_stretch_starts_over() {
        let mut deck = Deck {
            sounds: [Some(ramp(10, 10)), None],
            heard: 0,
            position: 0.5,
            playing: true,
            repeat: Some((0.2, 0.5)),
            end: 1.,
        };
        let mut frame = [0f32; 1];
        deck.next(0.1, 1, &mut frame);
        assert!((deck.position - 0.3).abs() < 1e-9);
    }
}
