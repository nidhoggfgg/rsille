use std::{io::Cursor, sync::OnceLock};

pub(super) const SIDE: usize = 320;

struct Frame {
    png: &'static [u8],
    rgba: OnceLock<Vec<u8>>,
}

impl Frame {
    const fn new(png: &'static [u8]) -> Self {
        Self {
            png,
            rgba: OnceLock::new(),
        }
    }
}

pub(super) fn pixels(id: usize) -> &'static [u8] {
    let frame = &FRAMES[id];
    frame.rgba.get_or_init(|| {
        let mut reader = png::Decoder::new(Cursor::new(frame.png))
            .read_info()
            .expect("embedded animation PNG");
        let mut rgba = vec![0; SIDE * SIDE * 4];
        reader
            .next_frame(&mut rgba)
            .expect("embedded animation pixels");
        rgba
    })
}

pub(super) struct Clip {
    pub duration: u16,
    // Each entry holds an exclusive end tick and an index into the shared frame pool.
    timeline: &'static [(u16, usize)],
}

impl Clip {
    pub fn frame(&self, age: u16) -> usize {
        let tick = age % self.duration;
        self.timeline[self.timeline.partition_point(|&(end, _)| end <= tick)].1
    }
}

include!(concat!(env!("OUT_DIR"), "/animations.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_frames_decode_and_clips_respect_frame_durations() {
        for i in 0..FRAMES.len() {
            assert_eq!(pixels(i).len(), SIDE * SIDE * 4);
        }
        let clip = Clip {
            duration: 7,
            timeline: &[(2, 3), (6, 4), (7, 1)],
        };
        assert_eq!(
            (0..9).map(|t| clip.frame(t)).collect::<Vec<_>>(),
            [3, 3, 4, 4, 4, 4, 1, 3, 3]
        );
    }
}
