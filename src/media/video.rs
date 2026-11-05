use std::{
    io::Read as _,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        OnceLock,
        mpsc::{Receiver, SyncSender, sync_channel},
    },
};

use anyhow::{Context as _, Result, bail};

use super::{Picture, Region, Sound};
use crate::git::without_console_window;

const PROBED_ENTRIES: &str = "format=duration,bit_rate:stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels";
const SOUND_RATE: u32 = 44_100;
const THUMB_WIDTH: u32 = 64;
const THUMB_HEIGHT: u32 = 36;
const THUMBS_PER_SECOND: f64 = 2.;
const VISIBLE_DIFFERENCE: f64 = 6.;
const FRAMES_AHEAD: usize = 3;
const BRIDGED_GAP: usize = 1;

pub(crate) fn ffmpeg_installed() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| {
        tool("ffprobe")
            .arg("-version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

fn tool(name: &str) -> Command {
    let mut command = Command::new(name);
    command.stdin(Stdio::null()).stderr(Stdio::null());
    without_console_window(&mut command);
    command
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct VideoFacts {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frame_rate: Option<f64>,
    pub(crate) duration: f64,
    pub(crate) video_codec: String,
    pub(crate) audio: Option<String>,
    pub(crate) bit_rate: Option<u64>,
    pub(crate) bytes: u64,
}

impl VideoFacts {
    pub(crate) fn probe(path: &Path) -> Result<Self> {
        let output = tool("ffprobe")
            .args(["-v", "error", "-show_entries", PROBED_ENTRIES, "-of", "compact=p=0"])
            .arg(path)
            .output()
            .context("Couldn't run ffprobe")?;
        if !output.status.success() {
            bail!("ffprobe couldn't read this video");
        }
        let mut facts = Self::parse(&String::from_utf8_lossy(&output.stdout));
        facts.bytes = std::fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
        if facts.width == 0 {
            bail!("This file has no picture in it");
        }
        Ok(facts)
    }

    fn parse(text: &str) -> Self {
        let mut facts = Self::default();
        for line in text.lines() {
            let field = |name: &str| {
                line.split('|')
                    .find_map(|pair| pair.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')))
                    .filter(|value| !value.is_empty() && *value != "N/A")
            };
            match field("codec_type") {
                Some("video") if facts.width == 0 => {
                    facts.width = field("width").and_then(|value| value.parse().ok()).unwrap_or(0);
                    facts.height = field("height").and_then(|value| value.parse().ok()).unwrap_or(0);
                    facts.frame_rate = field("r_frame_rate").and_then(ratio);
                    facts.video_codec = field("codec_name").map(codec_label).unwrap_or_default();
                }
                Some("audio") if facts.audio.is_none() => {
                    let codec = field("codec_name").map(codec_label).unwrap_or_default();
                    let rate = field("sample_rate")
                        .and_then(|value| value.parse::<f64>().ok())
                        .map(|hertz| format!(", {} kHz", hertz / 1000.))
                        .unwrap_or_default();
                    facts.audio = Some(format!("{codec}{rate}"));
                }
                Some(_) => {}
                None => {
                    if let Some(duration) = field("duration").and_then(|value| value.parse().ok()) {
                        facts.duration = duration;
                    }
                    facts.bit_rate = field("bit_rate").and_then(|value| value.parse().ok());
                }
            }
        }
        facts
    }

    pub(crate) fn frame_size(&self, widest: u32) -> (u32, u32) {
        let width = self.width.min(widest).max(2) & !1;
        let height = ((u64::from(self.height) * u64::from(width) / u64::from(self.width.max(1))) as u32).max(2) & !1;
        (width, height)
    }
}

fn ratio(text: &str) -> Option<f64> {
    let (top, bottom) = text.split_once('/')?;
    let (top, bottom): (f64, f64) = (top.parse().ok()?, bottom.parse().ok()?);
    (bottom > 0. && top > 0.).then(|| top / bottom)
}

fn codec_label(name: &str) -> String {
    match name {
        "h264" => "H.264".to_owned(),
        "hevc" => "H.265".to_owned(),
        "vp8" => "VP8".to_owned(),
        "vp9" => "VP9".to_owned(),
        "av1" => "AV1".to_owned(),
        "mpeg4" => "MPEG-4".to_owned(),
        "prores" => "ProRes".to_owned(),
        "aac" => "AAC".to_owned(),
        "mp3" => "MP3".to_owned(),
        "opus" => "Opus".to_owned(),
        "vorbis" => "Vorbis".to_owned(),
        "flac" => "FLAC".to_owned(),
        other => other.to_uppercase(),
    }
}

fn frames_command(path: &Path, from: f64, size: (u32, u32), filter: &str) -> Command {
    let mut command = tool("ffmpeg");
    command
        .args(["-v", "error", "-ss"])
        .arg(format!("{from:.3}"))
        .arg("-i")
        .arg(path)
        .args(["-an", "-vf"])
        .arg(format!("scale={}:{}{filter}", size.0, size.1))
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdout(Stdio::piped());
    command
}

pub(crate) fn frame_at(path: &Path, seconds: f64, size: (u32, u32)) -> Result<Picture> {
    let output = frames_command(path, seconds, size, "")
        .arg("-frames:v")
        .arg("1")
        .output()
        .context("Couldn't run ffmpeg")?;
    let expected = (size.0 * size.1 * 4) as usize;
    if output.stdout.len() < expected {
        bail!("ffmpeg gave no picture at {seconds:.2} s");
    }
    Ok(Picture::from_frame(size.0, size.1, output.stdout[..expected].to_vec()))
}

pub(crate) fn soundtrack(path: &Path) -> Result<Option<Sound>> {
    let output = tool("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-vn", "-ac", "2", "-ar"])
        .arg(SOUND_RATE.to_string())
        .args(["-f", "s16le", "-"])
        .stdout(Stdio::piped())
        .output()
        .context("Couldn't run ffmpeg")?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let samples = output
        .stdout
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Ok(Some(Sound::from_pcm(SOUND_RATE, 2, samples)))
}

pub(crate) fn thumbnails(path: &Path) -> Result<Vec<Vec<u8>>> {
    let output = tool("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-an", "-vf"])
        .arg(format!("fps={THUMBS_PER_SECOND},scale={THUMB_WIDTH}:{THUMB_HEIGHT},format=gray"))
        .args(["-f", "rawvideo", "-"])
        .stdout(Stdio::piped())
        .output()
        .context("Couldn't run ffmpeg")?;
    Ok(output
        .stdout
        .chunks_exact((THUMB_WIDTH * THUMB_HEIGHT) as usize)
        .map(<[u8]>::to_vec)
        .collect())
}

pub(crate) fn visible_changes(before: &[Vec<u8>], after: &[Vec<u8>], lengths: (f64, f64)) -> Vec<Region> {
    let step = 1. / THUMBS_PER_SECOND;
    let shared = before.len().min(after.len());
    let changed: Vec<bool> = (0..shared)
        .map(|ix| {
            let total: u64 = before[ix]
                .iter()
                .zip(&after[ix])
                .map(|(was, is)| u64::from(was.abs_diff(*is)))
                .sum();
            total as f64 / before[ix].len().max(1) as f64 > VISIBLE_DIFFERENCE
        })
        .collect();
    let mut regions: Vec<Region> = Vec::new();
    let mut run: Option<(usize, usize)> = None;
    for (ix, is_changed) in changed.iter().enumerate() {
        match (*is_changed, run) {
            (true, Some((first, last))) if ix - last <= BRIDGED_GAP + 1 => run = Some((first, ix)),
            (true, Some((first, last))) => {
                regions.push(Region { from: first as f64 * step, to: (last + 1) as f64 * step });
                run = Some((ix, ix));
            }
            (true, None) => run = Some((ix, ix)),
            (false, _) => {}
        }
    }
    if let Some((first, last)) = run {
        regions.push(Region { from: first as f64 * step, to: (last + 1) as f64 * step });
    }
    let (shorter, longer) = (lengths.0.min(lengths.1), lengths.0.max(lengths.1));
    if longer - shorter > step / 2. {
        match regions.last_mut() {
            Some(last) if last.to >= shorter - step => last.to = longer,
            _ => regions.push(Region { from: shorter, to: longer }),
        }
    }
    regions
}

pub(crate) struct TimedFrame {
    pub(crate) at: f64,
    pub(crate) picture: Picture,
}

pub(crate) struct FrameStream {
    child: Child,
    frames: Receiver<TimedFrame>,
    pending: Option<TimedFrame>,
}

impl FrameStream {
    pub(crate) fn start(path: &Path, from: f64, size: (u32, u32), rate: f64) -> Result<Self> {
        let mut child = frames_command(path, from, size, &format!(",fps={rate}"))
            .spawn()
            .context("Couldn't run ffmpeg")?;
        let mut stdout = child.stdout.take().context("ffmpeg gave no output")?;
        let (sender, frames): (SyncSender<TimedFrame>, _) = sync_channel(FRAMES_AHEAD);
        std::thread::spawn(move || {
            let length = (size.0 * size.1 * 4) as usize;
            for index in 0.. {
                let mut pixels = vec![0u8; length];
                if stdout.read_exact(&mut pixels).is_err() {
                    break;
                }
                let frame = TimedFrame {
                    at: from + f64::from(index) / rate,
                    picture: Picture::from_frame(size.0, size.1, pixels),
                };
                if sender.send(frame).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            frames,
            pending: None,
        })
    }

    pub(crate) fn latest_until(&mut self, clock: f64) -> Option<TimedFrame> {
        let mut latest = None;
        loop {
            let frame = match self.pending.take() {
                Some(frame) => frame,
                None => match self.frames.try_recv() {
                    Ok(frame) => frame,
                    Err(_) => break,
                },
            };
            if frame.at <= clock {
                latest = Some(frame);
            } else {
                self.pending = Some(frame);
                break;
            }
        }
        latest
    }
}

impl Drop for FrameStream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::{VideoFacts, visible_changes};

    #[test]
    fn ffprobe_lines_become_the_video_details() {
        let facts = VideoFacts::parse(
            "codec_name=h264|codec_type=video|width=1920|height=1080|r_frame_rate=60/1\ncodec_name=aac|codec_type=audio|sample_rate=48000|channels=2|r_frame_rate=0/0\nduration=44.500000|bit_rate=9600000\n",
        );
        assert_eq!((facts.width, facts.height), (1920, 1080));
        assert_eq!(facts.frame_rate, Some(60.));
        assert_eq!(facts.video_codec, "H.264");
        assert_eq!(facts.audio.as_deref(), Some("AAC, 48 kHz"));
        assert_eq!(facts.duration, 44.5);
        assert_eq!(facts.bit_rate, Some(9_600_000));
        assert_eq!(facts.frame_size(960), (960, 540));
    }

    #[test]
    fn frames_that_look_different_make_the_changes_and_so_does_the_extra_end() {
        let gray = |level: u8| vec![level; 64 * 36];
        let before = vec![gray(10); 6];
        let mut after = vec![gray(10); 8];
        after[1] = gray(200);
        after[2] = gray(200);
        let regions = visible_changes(&before, &after, (3., 4.));
        let spans: Vec<(f64, f64)> = regions.iter().map(|region| (region.from, region.to)).collect();
        assert_eq!(spans, [(0.5, 1.5), (3., 4.)]);
    }
}
