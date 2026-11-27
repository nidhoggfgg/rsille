//! Several overlays in one session.
use crate::{Attach, Context, MouseKind, Overlay, Pointer, ShellEvent};
use std::{io, time::Instant};

/// Overlays drawn in order, later ones on top.
///
/// A press goes to the topmost layer that [`Overlay::hit`]s it, and its drag
/// and release follow. Hover motion and all other notifications reach every
/// layer. Presses no layer claims are reported to all of them as
/// [`Overlay::pointer_passed`].
///
/// Text layers should not overlap: erasing one restores the child's text and
/// can wipe part of another until its next repaint.
#[derive(Default)]
pub struct Stack<'a> {
    layers: Vec<Box<dyn Overlay + 'a>>,
    owner: Option<usize>,
}

impl<'a> Stack<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a layer above the existing ones.
    pub fn push(mut self, layer: impl Overlay + 'a) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    fn top_hit(&self, pointer: &Pointer) -> Option<usize> {
        self.layers.iter().rposition(|layer| layer.hit(pointer))
    }
}

impl Overlay for Stack<'_> {
    fn attach(&mut self, attach: &Attach) -> io::Result<()> {
        self.layers.iter_mut().try_for_each(|l| l.attach(attach))
    }

    fn cleanup(&self, out: &mut Vec<u8>) {
        self.layers.iter().for_each(|l| l.cleanup(out));
    }

    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        self.layers.iter_mut().try_for_each(|l| l.draw(cx, out))
    }

    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        // Top first, so lower layers restore the child's text last.
        self.layers.iter_mut().rev().for_each(|l| l.hide(cx, out));
    }

    fn invalidate(&mut self) {
        self.layers.iter_mut().for_each(|l| l.invalidate());
    }

    fn invalidate_placements(&mut self) {
        self.layers
            .iter_mut()
            .for_each(|l| l.invalidate_placements());
    }

    fn wake_at(&mut self, now: Instant) -> Option<Instant> {
        self.layers.iter_mut().filter_map(|l| l.wake_at(now)).min()
    }

    fn hit(&self, pointer: &Pointer) -> bool {
        self.top_hit(pointer).is_some()
    }

    fn pointer(&mut self, pointer: Pointer) {
        match pointer.kind {
            MouseKind::LeftPress | MouseKind::RightPress => {
                self.owner = self.top_hit(&pointer);
                match self.owner {
                    Some(i) => self.layers[i].pointer(pointer),
                    None => self.pointer_passed(pointer),
                }
            }
            MouseKind::Motion if pointer.is_hover() => {
                self.layers.iter_mut().for_each(|l| l.pointer(pointer));
            }
            _ => {
                if let Some(i) = self.owner {
                    self.layers[i].pointer(pointer);
                }
                if pointer.kind == MouseKind::LeftRelease {
                    self.owner = None;
                }
            }
        }
    }

    fn pointer_passed(&mut self, pointer: Pointer) {
        self.layers
            .iter_mut()
            .for_each(|l| l.pointer_passed(pointer));
    }

    fn pointer_reset(&mut self) {
        self.owner = None;
        self.layers.iter_mut().for_each(|l| l.pointer_reset());
    }

    fn input(&mut self, bytes: &[u8]) {
        self.layers.iter_mut().for_each(|l| l.input(bytes));
    }

    fn output(&mut self, bytes: &[u8], cx: &mut Context<'_>) {
        self.layers.iter_mut().for_each(|l| l.output(bytes, cx));
    }

    fn bell(&mut self) {
        self.layers.iter_mut().for_each(|l| l.bell());
    }

    fn shell(&mut self, event: ShellEvent<'_>) {
        self.layers.iter_mut().for_each(|l| l.shell(event));
    }

    fn resized(&mut self, cols: u16, rows: u16) {
        self.layers.iter_mut().for_each(|l| l.resized(cols, rows));
    }
}
