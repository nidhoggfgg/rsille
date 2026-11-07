//! Cell-based pointer gestures. Positions use a fixed layout-space baseline,
//! independent of the handle's movement and of render frequency.
//!
//! [`DragState`] recognizes a drag from pointer positions; it does not capture
//! input or move widgets itself. Keep the coordinate baseline fixed for a whole
//! sequence. [`WindowRect`] applies its deltas to window geometry, with explicit
//! resizing, host constraints, and edge snapping.
//!
//! # Examples
//!
//! ```
//! use rsille::tui::{event::KeyModifiers, gesture::{CellPoint, DragAxis, DragEvent, DragState}};
//!
//! let mut drag = DragState::default();
//! let modifiers = KeyModifiers::empty();
//! assert!(drag.begin(CellPoint::new(2, 3), modifiers, 2).is_empty());
//! let events = drag.motion(CellPoint::new(5, 3), modifiers, 2, DragAxis::Horizontal);
//! assert!(matches!(events.as_slice(), [DragEvent::Begin(_), DragEvent::Update(_)]));
//! assert!(matches!(drag.end(CellPoint::new(5, 3), modifiers, DragAxis::Horizontal),
//!     Some(DragEvent::End(_))));
//! assert!(!drag.is_pending());
//! ```
use crate::tui::event::KeyModifiers;

/// A signed position in terminal-cell layout coordinates.
///
/// The x axis grows rightward and the y axis grows downward.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellPoint {
    /// Horizontal coordinate or delta in terminal columns.
    pub x: i32,
    /// Vertical coordinate or delta in terminal rows.
    pub y: i32,
}
impl CellPoint {
    /// Creates a signed terminal-cell position or displacement.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
/// Signed displacement in terminal columns and rows.
pub type CellDelta = CellPoint;
/// Axes along which drag deltas and activation thresholds are measured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DragAxis {
    /// Reports horizontal deltas and ignores vertical travel for activation.
    Horizontal,
    /// Reports vertical deltas and ignores horizontal travel for activation.
    Vertical,
    /// Reports both axes and uses their maximum absolute travel for activation (default).
    #[default]
    Both,
}
/// Reason a begun drag sequence was cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragCancelReason {
    /// The user pressed Escape.
    Escape,
    /// The terminal lost focus.
    FocusLost,
    /// The layout-space baseline changed during the gesture.
    CoordinateSpaceChanged,
    /// The gesture owner became disabled.
    Disabled,
    /// The gesture owner became hidden.
    Hidden,
    /// A modal boundary changed input ownership.
    ModalChanged,
    /// Pointer capture moved to another widget.
    CaptureTransferred,
    /// Application or widget code explicitly cancelled the gesture.
    Explicit,
    /// An application-controlled value changed during the gesture.
    ControlledValueChanged,
}
/// Pointer positions and axis-projected deltas for one drag event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragUpdate {
    /// Initial pointer position in the fixed layout coordinate space.
    pub origin: CellPoint,
    /// Current pointer position in the same coordinate space.
    pub position: CellPoint,
    /// Axis-projected displacement from the origin, using saturating subtraction.
    pub total_delta: CellDelta,
    /// Axis-projected displacement since the previous recorded position.
    pub delta: CellDelta,
    /// Keyboard modifiers held for this pointer event.
    pub modifiers: KeyModifiers,
}
/// Lifecycle event emitted by a drag recognizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEvent {
    /// The activation threshold was reached.
    Begin(DragUpdate),
    /// An active drag moved to a different position.
    Update(DragUpdate),
    /// An active drag ended at the supplied position.
    End(DragUpdate),
    /// An active drag was cancelled for the supplied reason.
    Cancel(DragCancelReason),
}

/// Pure recognizer: pending cancellation is silent; a begun sequence has exactly
/// one terminal event. Repeated positions do not produce updates.
#[derive(Debug, Clone, Default)]
pub struct DragState {
    origin: Option<CellPoint>,
    last: CellPoint,
    active: bool,
}
impl DragState {
    /// Returns whether a pointer-down sequence exists, including an already active drag.
    pub fn is_pending(&self) -> bool {
        self.origin.is_some()
    }
    /// Returns whether the sequence has crossed its activation threshold.
    pub fn is_active(&self) -> bool {
        self.active
    }
    /// Starts or replaces a pointer sequence at `position`.
    ///
    /// A zero threshold immediately emits `Begin`; otherwise returns an empty
    /// vector until motion reaches the threshold. Call `end` or `cancel` before
    /// reusing an active recognizer to preserve its terminal-event contract.
    pub fn begin(
        &mut self,
        position: CellPoint,
        modifiers: KeyModifiers,
        threshold: u16,
    ) -> Vec<DragEvent> {
        self.origin = Some(position);
        self.last = position;
        self.active = threshold == 0;
        if self.active {
            vec![DragEvent::Begin(self.update(
                position,
                modifiers,
                DragAxis::Both,
            ))]
        } else {
            vec![]
        }
    }
    fn update(&self, position: CellPoint, modifiers: KeyModifiers, axis: DragAxis) -> DragUpdate {
        let origin = self.origin.unwrap_or(position);
        let project = |x, y| match axis {
            DragAxis::Horizontal => CellPoint::new(x, 0),
            DragAxis::Vertical => CellPoint::new(0, y),
            DragAxis::Both => CellPoint::new(x, y),
        };
        DragUpdate {
            origin,
            position,
            total_delta: project(
                position.x.saturating_sub(origin.x),
                position.y.saturating_sub(origin.y),
            ),
            delta: project(
                position.x.saturating_sub(self.last.x),
                position.y.saturating_sub(self.last.y),
            ),
            modifiers,
        }
    }
    /// Records pointer movement and emits drag events when the threshold is reached.
    ///
    /// Thresholds use the maximum absolute projected delta in terminal cells. The
    /// first qualifying motion emits both `Begin` and `Update`. Missing sequences
    /// and repeated positions emit nothing.
    pub fn motion(
        &mut self,
        position: CellPoint,
        modifiers: KeyModifiers,
        threshold: u16,
        axis: DragAxis,
    ) -> Vec<DragEvent> {
        if self.origin.is_none() || self.last == position {
            return vec![];
        }
        let update = self.update(position, modifiers, axis);
        let mut events = vec![];
        if !self.active
            && update
                .total_delta
                .x
                .unsigned_abs()
                .max(update.total_delta.y.unsigned_abs())
                >= u32::from(threshold)
        {
            self.active = true;
            events.push(DragEvent::Begin(update));
        }
        if self.active {
            events.push(DragEvent::Update(update));
        }
        self.last = position;
        events
    }
    /// Ends and resets the sequence, returning `End` only if dragging had begun.
    ///
    /// This final position does not itself activate a pending drag.
    pub fn end(
        &mut self,
        position: CellPoint,
        modifiers: KeyModifiers,
        axis: DragAxis,
    ) -> Option<DragEvent> {
        let result = self
            .active
            .then(|| DragEvent::End(self.update(position, modifiers, axis)));
        *self = Self::default();
        result
    }
    /// Resets the sequence and returns a cancellation event only if dragging had begun.
    pub fn cancel(&mut self, reason: DragCancelReason) -> Option<DragEvent> {
        let result = self.active.then_some(DragEvent::Cancel(reason));
        *self = Self::default();
        result
    }
}

/// Outer window rectangle, including border and title, relative to its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRect {
    /// Left edge in terminal columns relative to the host; may be negative.
    pub x: i32,
    /// Top edge in terminal rows relative to the host; may be negative.
    pub y: i32,
    /// Outer width in terminal columns, including chrome.
    pub width: u16,
    /// Outer height in terminal rows, including chrome.
    pub height: u16,
}
impl Default for WindowRect {
    fn default() -> Self {
        Self::new(0, 0, 40, 14)
    }
}
/// Edge or corner moved when resizing a window rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    /// Moves the left edge.
    Left,
    /// Moves the right edge.
    Right,
    /// Moves the top edge.
    Top,
    /// Moves the bottom edge.
    Bottom,
    /// Moves the top-left corner.
    TopLeft,
    /// Moves the top-right corner.
    TopRight,
    /// Moves the bottom-left corner.
    BottomLeft,
    /// Moves the bottom-right corner.
    BottomRight,
}
impl WindowRect {
    /// Creates an outer rectangle without constraining it to a host or minimum size.
    pub const fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    /// Moves the rectangle by signed cell deltas, saturating coordinate arithmetic.
    pub fn translated(self, delta: CellDelta) -> Self {
        Self {
            x: self.x.saturating_add(delta.x),
            y: self.y.saturating_add(delta.y),
            ..self
        }
    }
    /// Moves the selected edge or corner by signed deltas.
    ///
    /// Resulting dimensions are clamped to `1..=u16::MAX`; host and configured
    /// size constraints are applied separately by [`Self::constrained`].
    pub fn resized(self, delta: CellDelta, edge: ResizeEdge) -> Self {
        let left = matches!(
            edge,
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
        );
        let right = matches!(
            edge,
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight
        );
        let top = matches!(
            edge,
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
        );
        let bottom = matches!(
            edge,
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
        );
        let dx = if left {
            delta.x.min(i32::from(self.width).saturating_sub(1))
        } else {
            0
        };
        let dy = if top {
            delta.y.min(i32::from(self.height).saturating_sub(1))
        } else {
            0
        };
        Self::new(
            self.x.saturating_add(dx),
            self.y.saturating_add(dy),
            i32::from(self.width)
                .saturating_sub(dx)
                .saturating_add(if right { delta.x } else { 0 })
                .clamp(1, u16::MAX.into()) as u16,
            i32::from(self.height)
                .saturating_sub(dy)
                .saturating_add(if bottom { delta.y } else { 0 })
                .clamp(1, u16::MAX.into()) as u16,
        )
    }
    /// Viewport bounds take priority over minimum size, including empty hosts.
    pub fn constrained(self, viewport: (u16, u16), min: (u16, u16), max: (u16, u16)) -> Self {
        let width = self.width.clamp(min.0.min(max.0), max.0).min(viewport.0);
        let height = self.height.clamp(min.1.min(max.1), max.1).min(viewport.1);
        Self::new(
            self.x.clamp(0, i32::from(viewport.0 - width)),
            self.y.clamp(0, i32::from(viewport.1 - height)),
            width,
            height,
        )
    }
    /// Snaps nearby edges to the host when their distance is at most `distance` cells.
    ///
    /// Left and top edges take precedence when both opposing edges are near.
    /// This changes position only; use [`Self::constrained`] for size and bounds limits.
    pub fn snapped(mut self, viewport: (u16, u16), distance: u16) -> Self {
        let right = i32::from(viewport.0.saturating_sub(self.width));
        let bottom = i32::from(viewport.1.saturating_sub(self.height));
        if self.x.unsigned_abs() <= u32::from(distance) {
            self.x = 0;
        } else if right.abs_diff(self.x) <= u32::from(distance) {
            self.x = right;
        }
        if self.y.unsigned_abs() <= u32::from(distance) {
            self.y = 0;
        } else if bottom.abs_diff(self.y) <= u32::from(distance) {
            self.y = bottom;
        }
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threshold_and_terminal_events() {
        let mut state = DragState::default();
        let m = KeyModifiers::empty();
        assert!(state.begin(CellPoint::new(8, 8), m, 2).is_empty());
        assert!(state
            .motion(CellPoint::new(7, 7), m, 2, DragAxis::Both)
            .is_empty());
        let events = state.motion(CellPoint::new(5, 6), m, 2, DragAxis::Both);
        assert!(
            matches!(events.as_slice(), [DragEvent::Begin(_), DragEvent::Update(u)] if u.total_delta == CellPoint::new(-3,-2) && u.delta == CellPoint::new(-2,-1))
        );
        assert!(state.cancel(DragCancelReason::Escape).is_some());
        assert!(state.end(CellPoint::new(0, 0), m, DragAxis::Both).is_none());
    }
    #[test]
    fn tiny_viewport_and_signed_resize() {
        let rect = WindowRect::new(10, 10, 40, 20);
        assert_eq!(
            rect.constrained((2, 1), (28, 11), (90, 32)),
            WindowRect::new(0, 0, 2, 1)
        );
        assert_eq!(
            rect.resized(CellPoint::new(-5, -3), ResizeEdge::TopLeft),
            WindowRect::new(5, 7, 45, 23)
        );
    }
}
