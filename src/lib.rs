#![doc = include_str!("../README.md")]

/// Terminal rendering engine, compiled from the internal render crate's sources.
#[path = "render/api.rs"]
pub mod render;

/// Sample-based canvas with Braille and Half-block terminal encoding.
#[path = "canvas/api.rs"]
pub mod canvas;

/// Terminal application framework, compiled from the internal TUI crate's sources.
#[path = "tui/api.rs"]
pub mod tui;
