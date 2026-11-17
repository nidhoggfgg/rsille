use super::*;
use std::{cell::RefCell, rc::Rc};

use rsille::render::{
    raster::{RasterCanvas, RasterCell},
    style::Stylized,
};

#[derive(Default)]
struct Transport {
    bytes: Vec<u8>,
    fail_after: Option<usize>,
    sends: usize,
}

#[derive(Clone, Default)]
struct Sink(Rc<RefCell<Transport>>);

impl FrameSink for Sink {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut state = self.0.borrow_mut();
        state.sends += 1;
        if let Some(count) = state.fail_after.take() {
            state
                .bytes
                .extend_from_slice(&bytes[..count.min(bytes.len())]);
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected transport failure",
            ));
        }
        state.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

impl Sink {
    fn take(&self) -> String {
        String::from_utf8(std::mem::take(&mut self.0.borrow_mut().bytes)).unwrap()
    }
}

fn text_buffer(width: u16, height: u16) -> Buffer {
    let mut buffer = Buffer::new((width, height).into());
    buffer.set((0, 0).into(), Stylized::plain('界')).unwrap();
    buffer
}

fn raster_buffer(height: u16) -> Buffer {
    let mut buffer = Buffer::new((2, height).into());
    buffer
        .set_raster(
            (0, 0).into(),
            RasterCell {
                canvas: RasterCanvas::new(2, 2, Default::default()).unwrap(),
                target: (1, 1).into(),
                x: 0,
                y: 0,
                options: Default::default(),
            },
        )
        .unwrap();
    buffer
}

#[test]
fn writer_buffers_split_utf8_and_flushes_the_real_sink() {
    let sink = Sink::default();
    let mut writer = FrameWriter::new(sink.clone());
    for byte in "界😀".as_bytes() {
        writer.write_all(&[*byte]).unwrap();
    }
    assert_eq!(sink.0.borrow().sends, 0);
    writer.flush().unwrap();
    assert_eq!(sink.take(), "界😀");
    assert_eq!(sink.0.borrow().sends, 1);
    writer.flush().unwrap();
    assert_eq!(sink.0.borrow().sends, 1);
    writer.write_all(b"not flushed on drop").unwrap();
    drop(writer);
    assert_eq!(sink.take(), "");
}

#[test]
fn failed_payload_is_not_replayed_by_recovery_flush() {
    let sink = Sink::default();
    let mut writer = FrameWriter::new(sink.clone());
    writer.write_all(b"frame-payload").unwrap();
    sink.0.borrow_mut().fail_after = Some(3);
    assert_eq!(
        writer.flush().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(sink.take(), "fra");
    writer.write_all(b"recovery").unwrap();
    writer.flush().unwrap();
    assert_eq!(sink.take(), "recovery");
}

#[test]
fn initial_newline_is_flushed_once_and_failure_is_reported() {
    let sink = Sink::default();
    let mut presentation = Presentation::new(sink.clone(), true, None).unwrap();
    assert_eq!(sink.take(), "\r\n");
    let buffer = text_buffer(4, 1);
    presentation.present(&buffer, (80, 24)).unwrap();
    assert!(!sink.take().contains("\r\n"));
    presentation.present(&buffer, (80, 24)).unwrap();
    assert!(!sink.take().contains("\r\n"));

    sink.0.borrow_mut().fail_after = Some(1);
    assert!(Presentation::new(sink.clone(), true, None).is_err());
}

#[test]
fn text_diff_is_persistent_in_both_modes() {
    for inline in [false, true] {
        let sink = Sink::default();
        let mut presentation = Presentation::new(sink.clone(), inline, None).unwrap();
        let buffer = text_buffer(8, 3);
        presentation.present(&buffer, (80, 24)).unwrap();
        let first = sink.take();
        assert!(first.contains('界'));
        assert_eq!(first.contains("\x1b[2J"), !inline);
        presentation.present(&buffer, (80, 24)).unwrap();
        let second = sink.take();
        assert!(!second.contains('界'));
        assert!(!second.contains("\x1b[2J"));
        assert!(!second.contains("\r\n"));
        assert!(second.len() < first.len());
    }
}

#[test]
fn raster_diff_and_removal_share_the_same_renderer() {
    for inline in [false, true] {
        let sink = Sink::default();
        let mut presentation = Presentation::new(
            sink.clone(),
            inline,
            Some(CellPixels {
                width: 8,
                height: 18,
            }),
        )
        .unwrap();
        let buffer = raster_buffer(8);
        presentation.present(&buffer, (80, 24)).unwrap();
        assert!(sink.take().contains("\x1bP"));
        let stats = presentation.renderer.raster_stats();
        presentation.present(&buffer, (80, 24)).unwrap();
        assert!(!sink.take().contains("\x1bP"));
        let repeated = presentation.renderer.raster_stats();
        assert_eq!(repeated.encodes, stats.encodes);
        assert_eq!(repeated.tiles_sent, stats.tiles_sent);
        assert_eq!(repeated.bytes_sent, stats.bytes_sent);
        assert!(repeated.cache_entries > 0);
        presentation.present(&text_buffer(2, 8), (80, 24)).unwrap();
        let removed = sink.take();
        assert!(!removed.contains("\x1bP"));
        assert!(removed.contains(if inline { "\x1b[2K" } else { "\x1b[2J" }));
        assert!(removed.contains('界'));
        if inline {
            assert!(!removed.contains("\x1b[2J"));
        }
    }
}

#[test]
fn inline_cleanup_and_commit_include_sixel_guards() {
    for height in [1u16, 2, 18, 20] {
        let padding = ((6 - height % 6) % 6).div_ceil(height);
        for preserve in [false, true] {
            let sink = Sink::default();
            let mut presentation =
                Presentation::new(sink.clone(), true, Some(CellPixels { width: 8, height }))
                    .unwrap();
            presentation.present(&raster_buffer(2), (80, 24)).unwrap();
            let drawn = sink.take();
            assert!(drawn.contains("\x1bP"));
            assert!(!drawn.contains("\x1b[2J"));
            if preserve {
                presentation.preserve_inline().unwrap();
                let committed = sink.take();
                assert_eq!(committed.matches("\r\n").count(), usize::from(padding + 1));
                assert!(!committed.contains("\x1b[2K"));
                presentation.clear_inline(24).unwrap();
                assert!(!sink.take().contains("\x1b[2K"));
            } else {
                presentation.clear_inline(24).unwrap();
                let cleared = sink.take();
                assert_eq!(cleared.matches("\x1b[2K").count(), usize::from(2 + padding));
                assert!(!cleared.contains("\x1b[2J"));
                presentation.clear_inline(24).unwrap();
                assert!(!sink.take().contains("\x1b[2K"));
            }
        }
    }
}

#[test]
fn resize_clips_snapshot_and_bounds_cleanup() {
    let sink = Sink::default();
    let mut presentation = Presentation::new(sink.clone(), true, None).unwrap();
    presentation
        .present(&text_buffer(10, 12), (80, 24))
        .unwrap();
    sink.take();
    presentation.present(&text_buffer(10, 8), (4, 2)).unwrap();
    assert_eq!(presentation.renderer.size(), (4, 2).into());
    let resized = sink.take();
    assert!(!resized.contains("\x1b[2J"));
    presentation.clear_inline(1).unwrap();
    assert_eq!(sink.take().matches("\x1b[2K").count(), 1);
}

#[test]
fn clipped_last_raster_cell_does_not_trigger_global_clear() {
    let sink = Sink::default();
    let mut presentation = Presentation::new(
        sink.clone(),
        true,
        Some(CellPixels {
            width: 8,
            height: 2,
        }),
    )
    .unwrap();
    let mut buffer = Buffer::new((2, 4).into());
    buffer
        .set_raster(
            (0, 3).into(),
            raster_buffer(1).raster_at((0, 0).into()).unwrap().clone(),
        )
        .unwrap();
    presentation.present(&buffer, (2, 4)).unwrap();
    let output = sink.take();
    assert!(!output.contains("\x1b[2J"));
    assert!(!output.contains("\x1bP"));
}

#[test]
fn invalid_raster_frames_are_rejected_before_transport() {
    for pixels in [
        None,
        Some(CellPixels {
            width: 8,
            height: 18,
        }),
    ] {
        let sink = Sink::default();
        let mut presentation = Presentation::new(sink.clone(), true, pixels).unwrap();
        presentation.present(&text_buffer(2, 2), (80, 24)).unwrap();
        sink.take();
        let mut invalid = raster_buffer(2);
        if pixels.is_some() {
            invalid.mark_unsupported_raster_composition();
        }
        assert!(presentation.present(&invalid, (80, 24)).is_err());
        assert_eq!(sink.take(), "");
        presentation.clear_inline(24).unwrap();
        assert_eq!(sink.take().matches("\x1b[2K").count(), 2);
    }
}

#[test]
fn failed_terminal_flush_does_not_blindly_erase_inline_rows() {
    for raster in [false, true] {
        let sink = Sink::default();
        let pixels = raster.then_some(CellPixels {
            width: 8,
            height: 20,
        });
        let mut presentation = Presentation::new(sink.clone(), true, pixels).unwrap();
        presentation.present(&text_buffer(2, 2), (80, 24)).unwrap();
        sink.take();
        sink.0.borrow_mut().fail_after = Some(5);
        let next = if raster {
            raster_buffer(3)
        } else {
            text_buffer(2, 3)
        };
        assert!(presentation.present(&next, (80, 24)).is_err());
        sink.take();
        // Recovery can emit protocol termination, but not guessed row erasure.
        let _ = presentation.clear_inline(24);
        let cleanup = sink.take();
        assert!(!cleanup.contains("\x1b[2K"));
        assert!(!cleanup.contains("\x1b[2J"));
    }
}

#[test]
fn failed_commit_does_not_erase_or_replay_output() {
    let sink = Sink::default();
    let mut presentation = Presentation::new(sink.clone(), true, None).unwrap();
    presentation.present(&text_buffer(2, 2), (80, 24)).unwrap();
    sink.take();
    sink.0.borrow_mut().fail_after = Some(1);
    assert!(presentation.preserve_inline().is_err());
    sink.take();
    let _ = presentation.clear_inline(24);
    let cleanup = sink.take();
    assert!(!cleanup.contains('界'));
    assert!(!cleanup.contains("\x1b[2K"));
    assert!(!cleanup.contains("\x1b[2J"));
}

#[test]
fn fullscreen_finishing_does_not_emit_inline_commands() {
    let sink = Sink::default();
    let mut presentation = Presentation::new(sink.clone(), false, None).unwrap();
    assert_eq!(sink.take(), "");
    presentation.present(&text_buffer(2, 2), (80, 24)).unwrap();
    sink.take();
    presentation.preserve_inline().unwrap();
    presentation.clear_inline(24).unwrap();
    assert_eq!(sink.take(), "");
}

#[test]
fn height_is_bounded_and_nonzero() {
    assert_eq!(inline_height(8, 24), 8);
    assert_eq!(inline_height(30, 24), 23);
    assert_eq!(inline_height(0, 24), 1);
    assert_eq!(inline_height(8, 1), 1);
}
