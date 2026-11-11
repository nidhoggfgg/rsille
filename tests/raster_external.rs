//! An external producer needs only public APIs, not a terminal or image library.
use rsille::render::{
    area::Area,
    buffer::Buffer,
    chunk::Chunk,
    raster::{
        RasterAlign, RasterCanvas, RasterCell, RasterError, RasterOptions, RasterScale, Rgb,
        MAX_RASTER_PIXELS,
    },
    raster_output::CellPixels,
    style::Stylized,
};
use rsille::tui::{
    animation::AnimationStore,
    layout::Constraints,
    prelude::raster_canvas,
    style::Theme,
    widget::{RenderCtx, Widget, WidgetStore},
};
use std::{cell::RefCell, collections::HashMap};

const BG: Rgb = Rgb(255, 255, 255);

#[test]
fn imported_buffers_keep_identity_and_cow_contract() {
    let pixels = vec![Rgb(1, 2, 3), Rgb(4, 5, 6)];
    let pointer = pixels.as_ptr();
    let mut canvas = RasterCanvas::from_pixels(2, 1, BG, pixels).unwrap();
    assert_eq!(canvas.pixels().as_ptr(), pointer);
    assert_eq!(canvas.background(), BG);
    assert_eq!(canvas.version(), 0);
    let snapshot = canvas.clone();
    canvas.set_pixel(0, 0, Rgb(1, 2, 3));
    assert_eq!(canvas, snapshot);
    assert_eq!(canvas.pixels().as_ptr(), pointer);
    canvas.set_pixel(0, 0, BG);
    assert_eq!(canvas.id(), snapshot.id());
    assert_ne!(canvas.version(), snapshot.version());
    assert_ne!(canvas.pixels().as_ptr(), pointer);
    let bytes = [1, 2, 3, 4, 5, 6];
    let imported = RasterCanvas::from_rgb8(2, 1, BG, &bytes).unwrap();
    assert_eq!(imported.pixels(), snapshot.pixels());
    assert_ne!(imported.id(), snapshot.id());
    let mut branch = snapshot.clone();
    branch.set_pixel(0, 0, BG);
    assert_ne!(branch.version(), canvas.version());
    assert_eq!(snapshot.pixels()[0], Rgb(1, 2, 3));
}

#[test]
fn import_rejects_invalid_dimensions_and_lengths() {
    for (w, h, expected) in [
        (0, 1, RasterError::EmptyDimensions),
        (1, 0, RasterError::EmptyDimensions),
        (u32::MAX, u32::MAX, RasterError::TooManyPixels),
        (MAX_RASTER_PIXELS as u32 + 1, 1, RasterError::TooManyPixels),
        (2, 2, RasterError::InvalidPixelLength),
    ] {
        assert_eq!(
            RasterCanvas::from_pixels(w, h, BG, vec![]).unwrap_err(),
            expected
        );
        assert_eq!(
            RasterCanvas::from_rgb8(w, h, BG, &[]).unwrap_err(),
            expected
        );
    }
    for len in [2, 4, 6] {
        assert_eq!(
            RasterCanvas::from_rgb8(1, 1, BG, &vec![0; len]).unwrap_err(),
            RasterError::InvalidPixelLength
        );
    }
    assert_eq!(
        RasterCanvas::from_pixels(1, 1, BG, vec![BG; 2]).unwrap_err(),
        RasterError::InvalidPixelLength
    );
}

#[test]
fn cell_size_conversion_is_checked_and_rounds_up() {
    let metrics = CellPixels {
        width: 8,
        height: 16,
    };
    for (w, h, expected) in [
        (16, 32, (2, 2)),
        (17, 33, (3, 3)),
        (0, 1, (0, 1)),
        (0, 0, (0, 0)),
    ] {
        assert_eq!(metrics.size_for_pixels(w, h), Some(expected.into()));
    }
    assert!(metrics.size_for_pixels(u32::MAX, 1).is_none());
    assert!(metrics.size_for_pixels(1, u32::MAX).is_none());
    for metrics in [
        CellPixels {
            width: 0,
            height: 1,
        },
        CellPixels {
            width: 1,
            height: 0,
        },
    ] {
        assert!(metrics.size_for_pixels(0, 0).is_none());
    }
    let metrics = CellPixels {
        width: u16::MAX,
        height: u16::MAX,
    };
    let max = u32::from(u16::MAX).pow(2);
    assert_eq!(
        metrics.size_for_pixels(max, max),
        Some((u16::MAX, u16::MAX).into())
    );
    assert!(metrics.size_for_pixels(max + 1, max).is_none());
}

#[test]
fn original_alignment_crops_and_pads_each_axis_without_resampling() {
    let source =
        RasterCanvas::from_pixels(5, 3, BG, (0..15).map(|i| Rgb(i % 5, i / 5, 0)).collect())
            .unwrap();
    for (align, crop, pad) in [
        (RasterAlign::Start, 0, 0),
        (RasterAlign::Center, 1, 1),
        (RasterAlign::End, 3, 3),
    ] {
        let cell = RasterCell {
            canvas: source.clone(),
            target: (1, 1).into(),
            x: 0,
            y: 0,
            options: RasterOptions {
                scale: RasterScale::Original,
                align_x: align,
                align_y: align,
                background: BG,
                ..Default::default()
            },
        };
        // Width crops three pixels; height pads three pixels.
        for y in 0..6 {
            for x in 0..2 {
                let expected = if (pad..pad + 3).contains(&y) {
                    Rgb((x + crop) as u8, (y - pad) as u8, 0)
                } else {
                    BG
                };
                assert_eq!(cell.sample(x, y, 2, 6), expected);
            }
        }
        assert_eq!(cell.sample(2, 0, 2, 6), BG);
        assert_eq!(cell.sample(0, 0, 0, 6), BG);
    }
}

#[test]
fn external_rgb8_to_widget_and_chunk_preserves_owners_and_placeholders() {
    let bytes: Vec<u8> = (0..72).collect();
    let source = RasterCanvas::from_rgb8(6, 4, BG, &bytes).unwrap();
    // Explicit metrics: no probing, global state, or image-library dependency.
    let size = CellPixels {
        width: 2,
        height: 2,
    }
    .size_for_pixels(6, 4)
    .unwrap();
    let area = Area::new((0, 0).into(), size);
    {
        let options = RasterOptions {
            scale: RasterScale::Original,
            align_x: RasterAlign::Start,
            align_y: RasterAlign::Start,

            ..Default::default()
        };
        let widget = raster_canvas::<()>(source.clone())
            .options(options)
            .constraints(Constraints::fixed(size.width, size.height))
            .fixed(size.width, size.height);
        let store = WidgetStore::new();
        let animations = AnimationStore::new();
        let theme = Theme::dark();
        let geometry = RefCell::new(HashMap::new());
        let ctx = RenderCtx::new(&store, &animations, &theme, None, &geometry);
        widget.layout(area, &ctx.measure_ctx());
        let mut frame = Buffer::new(size);
        widget.render(&mut Chunk::new(&mut frame, area).unwrap(), &ctx);
        let mut direct = Buffer::new(size);
        Chunk::new(&mut direct, area)
            .unwrap()
            .raster(&source, options)
            .unwrap();
        for (x, y, owner) in frame.raster_cells() {
            assert_eq!(Some(owner), direct.raster_at((x, y).into()));
            assert_eq!(frame.get((x, y).into()), Some(&owner.placeholder()));
            assert_eq!(owner.canvas.pixels().as_ptr(), source.pixels().as_ptr());
            assert_eq!(
                owner.sample(0, 0, 2, 2),
                source.pixels()[usize::from(y) * 12 + usize::from(x) * 2]
            );
        }
        assert_eq!(frame.raster_cells().count(), 6);
        // Text occlusion removes only the covered owner, never refits neighbors.
        let neighbor = frame.raster_at((1, 0).into()).unwrap().clone();
        frame.set((0, 0).into(), Stylized::plain('X')).unwrap();
        assert!(frame.raster_at((0, 0).into()).is_none());
        assert_eq!(frame.raster_at((1, 0).into()), Some(&neighbor));
    }
}
