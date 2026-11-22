//! Interactive fixtures driven by scripts/check-sixel-query.py on a controlling PTY.
use rsille::tui::prelude::*;
use std::time::Duration;

fn graphics(app: App<bool, ()>) -> Runtime<bool, ()> {
    app.mode(Mode::Fullscreen)
        .with_quit_key(KeyCode::Char('q'))
        .on_key(KeyCode::Char('i'), || ())
        .update(|visible, (), _| *visible = true)
        .view(|visible| {
            let view = col().child(label("Press i for graphics"));
            if *visible {
                view.child(
                    raster_canvas(RasterCanvas::new(2, 2, Rgb(3, 5, 7)).unwrap()).fixed(2, 1),
                )
            } else {
                view
            }
        })
        .build()
}

#[test]
#[ignore = "requires the raster PTY driver"]
fn managed_sync() -> WidgetResult<()> {
    graphics(App::new(false).raster_backend(RasterBackend::Auto)).run()?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires the raster PTY driver"]
async fn managed_async() -> WidgetResult<()> {
    graphics(App::new(false).raster_backend(RasterBackend::Auto))
        .run_async()
        .await?;
    Ok(())
}

#[test]
#[ignore = "requires the raster PTY driver"]
fn successive_sessions() -> Result<(), Box<dyn std::error::Error>> {
    let session = RasterBackend::Kitty.negotiate_fullscreen(None, Duration::from_millis(500))?;
    App::new(())
        .mode(Mode::Fullscreen)
        .with_quit_key(KeyCode::Char('q'))
        .update(|_, (), _| {})
        .view(|_| label("First text session"))
        .run()?;
    let error = graphics(App::new(false).raster_backend(RasterBackend::Kitty))
        .run()
        .expect_err("managed probing after the first input reader");
    assert!(error
        .to_string()
        .contains("terminal probing must precede managed input"));
    graphics(App::new(false).raster_session(session)).run()?;
    Ok(())
}

#[test]
#[ignore = "requires the raster PTY driver"]
fn inline_auto_sixel_requires_confirmation() {
    let error = RasterBackend::Auto
        .negotiate_inline(None, Duration::from_millis(500))
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("host confirmation"));
}

#[test]
#[ignore = "requires the raster PTY driver"]
fn inline_confirmed_sixel() -> std::io::Result<()> {
    let session = RasterBackend::Sixel.negotiate_inline(None, Duration::from_millis(500))?;
    assert_eq!(
        session,
        InlineRasterSession::Sixel(CellPixels {
            width: 10,
            height: 20
        })
    );
    Ok(())
}
