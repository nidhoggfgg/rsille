//! Cell-based pointer gestures. Positions use a fixed layout-space baseline,
//! independent of the handle's movement and of render frequency.
use crate::tui::event::KeyModifiers;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellPoint {
    pub x: i32,
    pub y: i32,
}
impl CellPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
pub type CellDelta = CellPoint;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DragAxis {
    Horizontal,
    Vertical,
    #[default]
    Both,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragCancelReason {
    Escape,
    FocusLost,
    CoordinateSpaceChanged,
    Disabled,
    Hidden,
    ModalChanged,
    CaptureTransferred,
    Explicit,
    ControlledValueChanged,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragUpdate {
    pub origin: CellPoint,
    pub position: CellPoint,
    pub total_delta: CellDelta,
    pub delta: CellDelta,
    pub modifiers: KeyModifiers,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEvent {
    Begin(DragUpdate),
    Update(DragUpdate),
    End(DragUpdate),
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
    pub fn is_pending(&self) -> bool {
        self.origin.is_some()
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
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
    pub fn cancel(&mut self, reason: DragCancelReason) -> Option<DragEvent> {
        let result = self.active.then_some(DragEvent::Cancel(reason));
        *self = Self::default();
        result
    }
}

/// Outer window rectangle, including border and title, relative to its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}
impl Default for WindowRect {
    fn default() -> Self {
        Self::new(0, 0, 40, 14)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl WindowRect {
    pub const fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn translated(self, delta: CellDelta) -> Self {
        Self {
            x: self.x.saturating_add(delta.x),
            y: self.y.saturating_add(delta.y),
            ..self
        }
    }
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
