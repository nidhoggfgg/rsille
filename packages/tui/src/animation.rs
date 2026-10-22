//! Animation primitives shared by TUI widgets and runtime policy.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use render::area::{Area, Position, Size};

use crate::style::Style;
use crate::widget::{WidgetId, WidgetPath};

const VALUE_EPSILON: f64 = 0.000_001;

/// Animation timing and easing configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSpec {
    pub duration: Duration,
    pub delay: Duration,
    pub easing: Easing,
    pub repeat: Repeat,
    pub direction: Direction,
}

impl AnimationSpec {
    pub fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            duration,
            delay: Duration::ZERO,
            easing,
            repeat: Repeat::Never,
            direction: Direction::Normal,
        }
    }

    pub fn fast() -> Self {
        Self::new(Duration::from_millis(120), Easing::EaseOut)
    }

    pub fn normal() -> Self {
        Self::default()
    }

    pub fn slow() -> Self {
        Self::new(Duration::from_millis(320), Easing::EaseInOut)
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn scaled(self, speed: f64) -> Self {
        if speed <= VALUE_EPSILON {
            return self;
        }

        Self {
            duration: scale_duration(self.duration, 1.0 / speed),
            delay: scale_duration(self.delay, 1.0 / speed),
            ..self
        }
    }

    fn has_motion(self) -> bool {
        !self.duration.is_zero() || !self.delay.is_zero()
    }

    fn progress_at(self, elapsed: Duration) -> (f64, bool) {
        if elapsed < self.delay {
            return (self.easing.sample(0.0), false);
        }

        if self.duration.is_zero() {
            return (1.0, true);
        }

        let active_elapsed = elapsed.saturating_sub(self.delay);
        let raw = active_elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let cycle = raw.floor() as u64;

        let complete = match self.repeat {
            Repeat::Never => raw >= 1.0,
            Repeat::Count(count) => raw >= count.max(1) as f64,
            Repeat::Forever => false,
        };

        if complete {
            return (1.0, true);
        }

        let mut t = raw.fract();
        if matches!(self.repeat, Repeat::Never) {
            t = raw.clamp(0.0, 1.0);
        }

        let directed = match self.direction {
            Direction::Normal => t,
            Direction::Reverse => 1.0 - t,
            Direction::Alternate => {
                if cycle % 2 == 0 {
                    t
                } else {
                    1.0 - t
                }
            }
        };

        (self.easing.sample(directed), false)
    }
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self::new(Duration::from_millis(180), Easing::EaseOut)
    }
}

/// Built-in easing curves for component and layout animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f64, f64, f64, f64),
    Steps(u16),
}

impl Easing {
    pub fn sample(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - ((-2.0 * t + 2.0).powi(2) / 2.0)
                }
            }
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier_y_for_x(t, x1, y1, x2, y2),
            Self::Steps(steps) => {
                let steps = steps.max(1) as f64;
                (t * steps).ceil() / steps
            }
        }
    }
}

/// Repeat behavior for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Never,
    Count(u16),
    Forever,
}

/// Playback direction for repeating tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Normal,
    Reverse,
    Alternate,
}

/// A concrete transition effect used by presence and animation wrappers.
///
/// Terminal UIs do not have pixel transforms, so spatial effects are expressed
/// as cell-area transitions and clipping policies.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub spec: AnimationSpec,
    pub effect: TransitionEffect,
}

impl Transition {
    pub fn new(effect: TransitionEffect, spec: AnimationSpec) -> Self {
        Self { effect, spec }
    }

    pub fn fade_in() -> Self {
        Self::new(TransitionEffect::Fade, AnimationSpec::fast())
    }

    pub fn fade_out() -> Self {
        Self::new(TransitionEffect::Fade, AnimationSpec::fast())
    }

    pub fn collapse() -> Self {
        Self::new(TransitionEffect::Collapse, AnimationSpec::fast())
    }

    pub fn expand() -> Self {
        Self::new(TransitionEffect::Expand, AnimationSpec::fast())
    }

    pub fn scale_from_center() -> Self {
        Self::new(TransitionEffect::ScaleFromCenter, AnimationSpec::normal())
    }

    pub fn layout(transition: LayoutTransition) -> Self {
        let spec = transition
            .position
            .or(transition.size)
            .unwrap_or_else(AnimationSpec::normal);
        Self::new(TransitionEffect::Layout(transition), spec)
    }
}

/// Built-in transition families.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionEffect {
    Fade,
    Collapse,
    Expand,
    ScaleFromCenter,
    BorderEmphasis,
    Layout(LayoutTransition),
}

/// Timeline composition for animation orchestration.
#[derive(Debug, Clone, PartialEq)]
pub enum Timeline {
    Single(Transition),
    Sequence(Vec<Timeline>),
    Parallel(Vec<Timeline>),
    Stagger {
        delay: Duration,
        children: Vec<Timeline>,
    },
}

impl Timeline {
    pub fn single(transition: Transition) -> Self {
        Self::Single(transition)
    }

    pub fn sequence(children: Vec<Timeline>) -> Self {
        Self::Sequence(children)
    }

    pub fn parallel(children: Vec<Timeline>) -> Self {
        Self::Parallel(children)
    }

    pub fn stagger(delay: Duration, children: Vec<Timeline>) -> Self {
        Self::Stagger { delay, children }
    }

    fn effective(self, motion_policy: MotionPolicy) -> Self {
        match self {
            Self::Single(mut transition) => {
                transition.spec = motion_policy.effective_spec(transition.spec);
                if let TransitionEffect::Layout(layout) = transition.effect {
                    transition.effect = TransitionEffect::Layout(LayoutTransition {
                        position: layout
                            .position
                            .map(|spec| motion_policy.effective_spec(spec)),
                        size: layout.size.map(|spec| motion_policy.effective_spec(spec)),
                        clip: layout.clip,
                        hit_test: layout.hit_test,
                    });
                }
                Self::Single(transition)
            }
            Self::Sequence(children) => Self::Sequence(
                children
                    .into_iter()
                    .map(|child| child.effective(motion_policy))
                    .collect(),
            ),
            Self::Parallel(children) => Self::Parallel(
                children
                    .into_iter()
                    .map(|child| child.effective(motion_policy))
                    .collect(),
            ),
            Self::Stagger { delay, children } => Self::Stagger {
                delay: motion_policy
                    .effective_interval(delay)
                    .unwrap_or(Duration::ZERO),
                children: children
                    .into_iter()
                    .map(|child| child.effective(motion_policy))
                    .collect(),
            },
        }
    }

    fn scheduled(&self) -> TimelineSchedule {
        let mut transitions = Vec::new();
        let duration = collect_timeline(self, Duration::ZERO, &mut transitions);
        TimelineSchedule {
            transitions,
            duration,
        }
    }
}

impl From<Transition> for Timeline {
    fn from(transition: Transition) -> Self {
        Self::Single(transition)
    }
}

/// Whether a present widget should play its initial enter transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialAnimation {
    Play,
    Skip,
}

/// Presence declaration for widgets that need enter/exit lifecycle animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Presence {
    pub enter: Option<Timeline>,
    pub exit: Option<Timeline>,
    pub initial: InitialAnimation,
}

impl Presence {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(mut self, timeline: impl Into<Timeline>) -> Self {
        self.enter = Some(timeline.into());
        self
    }

    pub fn exit(mut self, timeline: impl Into<Timeline>) -> Self {
        self.exit = Some(timeline.into());
        self
    }

    pub fn initial(mut self, initial: InitialAnimation) -> Self {
        self.initial = initial;
        self
    }
}

impl Default for Presence {
    fn default() -> Self {
        Self {
            enter: None,
            exit: None,
            initial: InitialAnimation::Skip,
        }
    }
}

/// Global motion behavior applied by the runtime before widget animations run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionPolicy {
    pub enabled: bool,
    pub reduced_motion: bool,
    pub speed: f64,
    pub deterministic: bool,
    pub deterministic_step: Duration,
}

impl MotionPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    pub fn reduced_motion() -> Self {
        Self {
            reduced_motion: true,
            ..Self::default()
        }
    }

    pub fn deterministic(step: Duration) -> Self {
        Self {
            deterministic: true,
            deterministic_step: if step.is_zero() {
                Duration::from_millis(16)
            } else {
                step
            },
            ..Self::default()
        }
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = speed.max(VALUE_EPSILON);
        self
    }

    pub fn effective_spec(self, spec: AnimationSpec) -> AnimationSpec {
        if !self.enabled || self.reduced_motion {
            return AnimationSpec {
                duration: Duration::ZERO,
                delay: Duration::ZERO,
                repeat: Repeat::Never,
                direction: Direction::Normal,
                ..spec
            };
        }

        spec.scaled(self.speed)
    }

    pub fn effective_interval(self, interval: Duration) -> Option<Duration> {
        if !self.enabled || self.reduced_motion {
            return None;
        }

        Some(scale_duration(
            interval,
            1.0 / self.speed.max(VALUE_EPSILON),
        ))
    }
}

impl Default for MotionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            reduced_motion: false,
            speed: 1.0,
            deterministic: false,
            deterministic_step: Duration::from_millis(16),
        }
    }
}

/// Theme-level animation defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationTheme {
    pub fast: AnimationSpec,
    pub normal: AnimationSpec,
    pub slow: AnimationSpec,
    pub focus: AnimationSpec,
    pub layout: AnimationSpec,
    pub enter: AnimationSpec,
    pub exit: AnimationSpec,
}

impl Default for AnimationTheme {
    fn default() -> Self {
        Self {
            fast: AnimationSpec::fast(),
            normal: AnimationSpec::normal(),
            slow: AnimationSpec::slow(),
            focus: AnimationSpec::fast(),
            layout: AnimationSpec::new(Duration::from_millis(220), Easing::EaseOut),
            enter: AnimationSpec::new(Duration::from_millis(160), Easing::EaseOut),
            exit: AnimationSpec::new(Duration::from_millis(120), Easing::EaseIn),
        }
    }
}

/// Floating-point cell area used by layout animations before final rounding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaF {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AreaF {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn lerp(self, target: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            x: lerp(self.x, target.x, t),
            y: lerp(self.y, target.y, t),
            width: lerp(self.width, target.width, t),
            height: lerp(self.height, target.height, t),
        }
    }

    pub fn to_area(self) -> Area {
        Area::new(
            Position {
                x: self.x.round().clamp(0.0, u16::MAX as f64) as u16,
                y: self.y.round().clamp(0.0, u16::MAX as f64) as u16,
            },
            Size {
                width: self.width.round().clamp(0.0, u16::MAX as f64) as u16,
                height: self.height.round().clamp(0.0, u16::MAX as f64) as u16,
            },
        )
    }
}

impl From<Area> for AreaF {
    fn from(area: Area) -> Self {
        Self {
            x: area.x() as f64,
            y: area.y() as f64,
            width: area.width() as f64,
            height: area.height() as f64,
        }
    }
}

/// Stored target and displayed areas for layout transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSnapshot {
    pub target: Area,
    pub previous: Option<Area>,
    pub displayed: AreaF,
}

/// Layout transition declaration for future layout-aware containers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutTransition {
    pub position: Option<AnimationSpec>,
    pub size: Option<AnimationSpec>,
    pub clip: ClipMode,
    pub hit_test: HitTestMode,
}

impl LayoutTransition {
    pub fn none() -> Self {
        Self {
            position: None,
            size: None,
            clip: ClipMode::None,
            hit_test: HitTestMode::Target,
        }
    }

    pub fn position(spec: AnimationSpec) -> Self {
        Self {
            position: Some(spec),
            size: None,
            clip: ClipMode::None,
            hit_test: HitTestMode::Target,
        }
    }

    pub fn size(spec: AnimationSpec) -> Self {
        Self {
            position: None,
            size: Some(spec),
            clip: ClipMode::ClipToAnimatedBounds,
            hit_test: HitTestMode::Target,
        }
    }

    pub fn size_and_position(spec: AnimationSpec) -> Self {
        Self {
            position: Some(spec),
            size: Some(spec),
            clip: ClipMode::ClipToAnimatedBounds,
            hit_test: HitTestMode::Target,
        }
    }

    pub fn hit_test(mut self, mode: HitTestMode) -> Self {
        self.hit_test = mode;
        self
    }
}

impl Default for LayoutTransition {
    fn default() -> Self {
        Self::none()
    }
}

/// How a layout transition clips child rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    None,
    ClipToAnimatedBounds,
    ClipToTargetBounds,
}

/// Which geometry should receive pointer events while layout animation is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTestMode {
    /// Use the logical layout target. This preserves pre-animation event behavior.
    Target,
    /// Use the animated displayed area.
    Display,
    /// Use the bounding union of the logical target and animated displayed area.
    TargetAndDisplay,
    /// Exclude this widget from pointer hit testing.
    None,
}

impl Default for HitTestMode {
    fn default() -> Self {
        Self::Target
    }
}

/// A shared area transition joins layout animation state across widget paths.
///
/// This is useful for "shared element" flows where a visual concept moves from
/// one subtree to another, such as a selected list row becoming a detail panel.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedTransition {
    pub id: String,
    pub layout: LayoutTransition,
}

impl SharedTransition {
    pub fn new(id: impl Into<String>, layout: LayoutTransition) -> Self {
        Self {
            id: id.into(),
            layout,
        }
    }
}

/// A transition that is active at a specific timeline instant.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineFrame {
    pub transition: Transition,
    pub progress: f64,
    pub complete: bool,
}

#[derive(Debug, Clone)]
struct ScheduledTransition {
    offset: Duration,
    transition: Transition,
}

#[derive(Debug, Clone)]
struct TimelineSchedule {
    transitions: Vec<ScheduledTransition>,
    duration: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AnimationKey {
    id: WidgetId,
    channel: String,
}

impl AnimationKey {
    fn new(id: &WidgetId, channel: &str) -> Self {
        Self {
            id: id.clone(),
            channel: channel.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TimelineKey {
    path: WidgetPath,
    channel: String,
}

impl TimelineKey {
    fn new(path: &WidgetPath, channel: &str) -> Self {
        Self {
            path: path.clone(),
            channel: channel.to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct ValueAnimation {
    displayed: f64,
    start: f64,
    target: f64,
    started_at: Instant,
    spec: AnimationSpec,
    active: bool,
}

impl ValueAnimation {
    fn new(target: f64, spec: AnimationSpec, now: Instant) -> Self {
        Self {
            displayed: target,
            start: target,
            target,
            started_at: now,
            spec,
            active: false,
        }
    }

    fn current_at(&self, now: Instant) -> f64 {
        if !self.active || nearly_equal(self.displayed, self.target) {
            return self.target;
        }

        let (progress, complete) = self
            .spec
            .progress_at(now.saturating_duration_since(self.started_at));
        if complete {
            return self.target;
        }

        self.start + (self.target - self.start) * progress
    }

    fn update_to(&mut self, target: f64, spec: AnimationSpec, now: Instant) -> bool {
        let before = self.displayed;
        let current = self.current_at(now);
        self.displayed = current;

        if !nearly_equal(target, self.target) {
            self.start = current;
            self.target = target;
            self.started_at = now;
            self.spec = spec;
            self.active = !nearly_equal(current, target) && spec.has_motion();
            if !self.active {
                self.displayed = target;
            }
            return true;
        }

        if self.active {
            let (_, complete) = self
                .spec
                .progress_at(now.saturating_duration_since(self.started_at));
            if complete || nearly_equal(self.displayed, self.target) {
                self.displayed = self.target;
                self.active = false;
            }

            return true;
        }

        !nearly_equal(before, self.displayed)
    }
}

#[derive(Debug, Clone)]
struct PulseAnimation {
    value: f64,
    last_tick: Instant,
}

#[derive(Debug, Clone)]
struct StyleAnimation {
    displayed: Style,
    start: Style,
    target: Style,
    started_at: Instant,
    spec: AnimationSpec,
    active: bool,
}

impl StyleAnimation {
    fn new(target: Style, spec: AnimationSpec, now: Instant) -> Self {
        Self {
            displayed: target,
            start: target,
            target,
            started_at: now,
            spec,
            active: false,
        }
    }

    fn current_at(&self, now: Instant) -> Style {
        if !self.active || self.displayed == self.target {
            return self.target;
        }

        let (progress, complete) = self
            .spec
            .progress_at(now.saturating_duration_since(self.started_at));
        if complete {
            return self.target;
        }

        self.start.interpolate(self.target, progress)
    }

    fn update_to(&mut self, target: Style, spec: AnimationSpec, now: Instant) -> bool {
        let before = self.displayed;
        let current = self.current_at(now);
        self.displayed = current;

        if target != self.target {
            self.start = current;
            self.target = target;
            self.started_at = now;
            self.spec = spec;
            self.active = current != target && spec.has_motion();
            if !self.active {
                self.displayed = target;
            }
            return true;
        }

        if self.active {
            let (_, complete) = self
                .spec
                .progress_at(now.saturating_duration_since(self.started_at));
            if complete || self.displayed == self.target {
                self.displayed = self.target;
                self.active = false;
            }

            return true;
        }

        before != self.displayed
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }

        let before = self.displayed;
        self.displayed = self.current_at(now);
        let (_, complete) = self
            .spec
            .progress_at(now.saturating_duration_since(self.started_at));
        if complete || self.displayed == self.target {
            self.displayed = self.target;
            self.active = false;
        }

        self.active || before != self.displayed
    }
}

#[derive(Debug, Clone)]
struct LayoutAnimation {
    displayed: AreaF,
    start: AreaF,
    target: AreaF,
    previous: Option<Area>,
    started_at: Instant,
    transition: LayoutTransition,
    active: bool,
}

impl LayoutAnimation {
    fn new(target: Area, transition: LayoutTransition, now: Instant) -> Self {
        let target_f = AreaF::from(target);
        Self {
            displayed: target_f,
            start: target_f,
            target: target_f,
            previous: None,
            started_at: now,
            transition,
            active: false,
        }
    }

    fn current_at(&self, now: Instant) -> AreaF {
        if !self.active || self.displayed == self.target {
            return self.target;
        }

        let elapsed = now.saturating_duration_since(self.started_at);
        let position_progress = self
            .transition
            .position
            .map(|spec| spec.progress_at(elapsed).0)
            .unwrap_or(1.0);
        let size_progress = self
            .transition
            .size
            .map(|spec| spec.progress_at(elapsed).0)
            .unwrap_or(1.0);

        AreaF {
            x: lerp(self.start.x, self.target.x, position_progress),
            y: lerp(self.start.y, self.target.y, position_progress),
            width: lerp(self.start.width, self.target.width, size_progress),
            height: lerp(self.start.height, self.target.height, size_progress),
        }
    }

    fn is_complete(&self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.started_at);
        let position_complete = self
            .transition
            .position
            .map(|spec| spec.progress_at(elapsed).1)
            .unwrap_or(true);
        let size_complete = self
            .transition
            .size
            .map(|spec| spec.progress_at(elapsed).1)
            .unwrap_or(true);

        position_complete && size_complete
    }

    fn update_to(&mut self, target: Area, transition: LayoutTransition, now: Instant) -> bool {
        let before = self.displayed;
        let current = self.current_at(now);
        let target_f = AreaF::from(target);
        self.displayed = current;

        if !area_nearly_equal(target_f, self.target) {
            self.previous = Some(self.target.to_area());
            self.start = current;
            self.target = target_f;
            self.started_at = now;
            self.transition = transition;
            self.active = !area_nearly_equal(current, target_f)
                && (transition.position.is_some() || transition.size.is_some());
            if !self.active {
                self.displayed = target_f;
            }
            return true;
        }

        if self.active {
            if self.is_complete(now) || area_nearly_equal(self.displayed, self.target) {
                self.displayed = self.target;
                self.active = false;
            }

            return true;
        }

        !area_nearly_equal(before, self.displayed)
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }

        let before = self.displayed;
        self.displayed = self.current_at(now);
        if self.is_complete(now) || area_nearly_equal(self.displayed, self.target) {
            self.displayed = self.target;
            self.active = false;
        }

        self.active || !area_nearly_equal(before, self.displayed)
    }

    fn snapshot(&self) -> LayoutSnapshot {
        LayoutSnapshot {
            target: self.target.to_area(),
            previous: self.previous,
            displayed: self.displayed,
        }
    }
}

#[derive(Debug, Clone)]
struct TimelineAnimation {
    timeline: Timeline,
    started_at: Instant,
    active: bool,
}

#[derive(Debug, Clone)]
struct SharedLayoutAnimation {
    path: WidgetPath,
    layout: LayoutAnimation,
}

impl SharedLayoutAnimation {
    fn new(path: WidgetPath, target: Area, transition: LayoutTransition, now: Instant) -> Self {
        Self {
            path,
            layout: LayoutAnimation::new(target, transition, now),
        }
    }

    fn update_to(
        &mut self,
        path: WidgetPath,
        target: Area,
        transition: LayoutTransition,
        now: Instant,
    ) -> bool {
        let path_changed = self.path != path;
        let changed = self.layout.update_to(target, transition, now);
        self.path = path;
        changed || path_changed
    }
}

impl TimelineAnimation {
    fn new(timeline: Timeline, now: Instant) -> Self {
        let active = timeline
            .scheduled()
            .duration
            .map(|duration| !duration.is_zero())
            .unwrap_or(true);
        Self {
            timeline,
            started_at: now,
            active,
        }
    }

    fn update_to(&mut self, timeline: Timeline, restart: bool, now: Instant) -> bool {
        if restart || self.timeline != timeline {
            *self = Self::new(timeline, now);
            return true;
        }

        let was_active = self.active;
        self.active = !self.is_complete(now);
        was_active || self.active
    }

    fn frames_at(&self, now: Instant) -> Vec<TimelineFrame> {
        let elapsed = now.saturating_duration_since(self.started_at);
        let schedule = self.timeline.scheduled();
        if schedule.transitions.is_empty() {
            return Vec::new();
        }

        schedule
            .transitions
            .into_iter()
            .filter_map(|scheduled| {
                if elapsed < scheduled.offset {
                    return None;
                }

                let local_elapsed = elapsed.saturating_sub(scheduled.offset);
                let (progress, complete) = scheduled.transition.spec.progress_at(local_elapsed);
                if complete && !transition_is_visible_after_completion(&scheduled.transition) {
                    return None;
                }

                Some(TimelineFrame {
                    transition: scheduled.transition,
                    progress,
                    complete,
                })
            })
            .collect()
    }

    fn is_complete(&self, now: Instant) -> bool {
        let Some(duration) = self.timeline.scheduled().duration else {
            return false;
        };

        now.saturating_duration_since(self.started_at) >= duration
    }

    fn advance(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }

        self.active = !self.is_complete(now);
        true
    }
}

impl PulseAnimation {
    fn new(now: Instant) -> Self {
        Self {
            value: 0.0,
            last_tick: now,
        }
    }

    fn tick(&mut self, interval: Duration, now: Instant) {
        if interval.is_zero() {
            self.value += 1.0;
            self.last_tick = now;
            return;
        }

        let elapsed = now.saturating_duration_since(self.last_tick);
        let steps = elapsed.as_nanos() / interval.as_nanos();
        if steps > 0 {
            self.value += steps as f64;
            self.last_tick += interval.saturating_mul(steps.min(u32::MAX as u128) as u32);
        }
    }
}

/// Stores component animation state keyed by widget identity and named channel.
#[derive(Debug, Default)]
pub struct AnimationStore {
    values: HashMap<AnimationKey, ValueAnimation>,
    pulses: HashMap<AnimationKey, PulseAnimation>,
    styles: HashMap<AnimationKey, StyleAnimation>,
    layouts: HashMap<AnimationKey, LayoutAnimation>,
    shared_layouts: HashMap<String, SharedLayoutAnimation>,
    timelines: HashMap<TimelineKey, TimelineAnimation>,
}

impl AnimationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Move the frozen channels needed by a logically removed subtree into its
    /// exit renderer. New components reusing an identity start independently.
    pub(crate) fn extract(
        &mut self,
        ids: &std::collections::HashSet<WidgetId>,
        paths: &[WidgetPath],
    ) -> Self {
        let mut extracted = Self::new();
        macro_rules! move_channels {
            ($field:ident, $matches:expr) => {
                let keys: Vec<_> = self.$field.keys().filter($matches).cloned().collect();
                for key in keys {
                    extracted
                        .$field
                        .insert(key.clone(), self.$field.remove(&key).unwrap());
                }
            };
        }
        move_channels!(values, |key: &&AnimationKey| ids.contains(&key.id));
        move_channels!(pulses, |key: &&AnimationKey| ids.contains(&key.id));
        move_channels!(styles, |key: &&AnimationKey| ids.contains(&key.id));
        move_channels!(layouts, |key: &&AnimationKey| ids.contains(&key.id));
        move_channels!(timelines, |key: &&TimelineKey| paths
            .iter()
            .any(|path| key.path.starts_with(path)));
        // Exit rendering reads the last displayed channels. Only its explicit
        // exit timeline advances; pulses and unfinished enters cannot keep it alive.
        extracted
    }

    pub fn value(&self, id: impl Into<WidgetId>, channel: &str) -> Option<f64> {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.values
            .get(&key)
            .map(|state| state.displayed)
            .or_else(|| self.pulses.get(&key).map(|state| state.value))
    }

    pub fn layout_snapshot(
        &self,
        id: impl Into<WidgetId>,
        channel: &str,
    ) -> Option<LayoutSnapshot> {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.layouts.get(&key).map(LayoutAnimation::snapshot)
    }

    pub fn shared_layout_snapshot(&self, id: &str) -> Option<LayoutSnapshot> {
        self.shared_layouts
            .get(id)
            .map(|state| state.layout.snapshot())
    }

    pub fn style(&self, id: impl Into<WidgetId>, channel: &str) -> Option<Style> {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.styles.get(&key).map(|state| state.displayed)
    }

    pub(crate) fn timeline_frames(
        &self,
        path: &WidgetPath,
        channel: &str,
        now: Instant,
    ) -> Vec<TimelineFrame> {
        let key = TimelineKey::new(path, channel);
        self.timelines
            .get(&key)
            .map(|state| state.frames_at(now))
            .unwrap_or_default()
    }

    pub(crate) fn timeline_is_active(
        &self,
        path: &WidgetPath,
        channel: &str,
        now: Instant,
    ) -> bool {
        let key = TimelineKey::new(path, channel);
        self.timelines
            .get(&key)
            .map(|state| state.active && !state.is_complete(now))
            .unwrap_or(false)
    }

    fn track_value(
        &mut self,
        id: impl Into<WidgetId>,
        channel: &str,
        target: f64,
        spec: AnimationSpec,
        now: Instant,
    ) -> bool {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.values
            .entry(key)
            .or_insert_with(|| ValueAnimation::new(target, spec, now))
            .update_to(target, spec, now)
    }

    fn pulse(
        &mut self,
        id: impl Into<WidgetId>,
        channel: &str,
        interval: Duration,
        now: Instant,
    ) -> bool {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.pulses
            .entry(key)
            .or_insert_with(|| PulseAnimation::new(now))
            .tick(interval, now);
        true
    }

    fn track_style(
        &mut self,
        id: impl Into<WidgetId>,
        channel: &str,
        target: Style,
        spec: AnimationSpec,
        now: Instant,
    ) -> bool {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.styles
            .entry(key)
            .or_insert_with(|| StyleAnimation::new(target, spec, now))
            .update_to(target, spec, now)
    }

    pub(crate) fn track_timeline(
        &mut self,
        path: &WidgetPath,
        channel: &str,
        timeline: Timeline,
        restart: bool,
        now: Instant,
        motion_policy: MotionPolicy,
    ) -> (Vec<TimelineFrame>, bool) {
        let timeline = timeline.effective(motion_policy);
        let key = TimelineKey::new(path, channel);
        let state = self
            .timelines
            .entry(key)
            .or_insert_with(|| TimelineAnimation::new(timeline.clone(), now));
        let changed = state.update_to(timeline, restart, now);
        (state.frames_at(now), changed || state.active)
    }

    fn remove_channel(&mut self, id: impl Into<WidgetId>, channel: &str) {
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        self.values.remove(&key);
        self.pulses.remove(&key);
        self.styles.remove(&key);
        self.layouts.remove(&key);
    }

    pub fn track_layout(
        &mut self,
        id: impl Into<WidgetId>,
        channel: &str,
        target: Area,
        transition: LayoutTransition,
        now: Instant,
        motion_policy: MotionPolicy,
    ) -> (Area, bool) {
        let transition = LayoutTransition {
            position: transition
                .position
                .map(|spec| motion_policy.effective_spec(spec)),
            size: transition
                .size
                .map(|spec| motion_policy.effective_spec(spec)),
            clip: transition.clip,
            hit_test: transition.hit_test,
        };
        let id = id.into();
        let key = AnimationKey::new(&id, channel);
        let state = self
            .layouts
            .entry(key)
            .or_insert_with(|| LayoutAnimation::new(target, transition, now));
        let changed = state.update_to(target, transition, now);
        (state.displayed.to_area(), changed || state.active)
    }

    pub fn track_shared_layout(
        &mut self,
        id: &str,
        path: &WidgetPath,
        target: Area,
        transition: LayoutTransition,
        now: Instant,
        motion_policy: MotionPolicy,
    ) -> (Area, bool) {
        let transition = LayoutTransition {
            position: transition
                .position
                .map(|spec| motion_policy.effective_spec(spec)),
            size: transition
                .size
                .map(|spec| motion_policy.effective_spec(spec)),
            clip: transition.clip,
            hit_test: transition.hit_test,
        };
        let state = self
            .shared_layouts
            .entry(id.to_owned())
            .or_insert_with(|| SharedLayoutAnimation::new(path.clone(), target, transition, now));
        let changed = state.update_to(path.clone(), target, transition, now);
        (
            state.layout.displayed.to_area(),
            changed || state.layout.active,
        )
    }

    /// Advance tracks that are already active without changing their targets.
    pub fn advance(&mut self, now: Instant) -> bool {
        let mut active = false;

        for value in self.values.values_mut() {
            if !value.active {
                continue;
            }

            let before = value.displayed;
            value.displayed = value.current_at(now);
            let (_, complete) = value
                .spec
                .progress_at(now.saturating_duration_since(value.started_at));
            if complete || nearly_equal(value.displayed, value.target) {
                value.displayed = value.target;
                value.active = false;
            }
            active |= value.active || !nearly_equal(before, value.displayed);
        }

        for layout in self.layouts.values_mut() {
            active |= layout.advance(now);
        }

        for shared in self.shared_layouts.values_mut() {
            active |= shared.layout.advance(now);
        }

        for style in self.styles.values_mut() {
            active |= style.advance(now);
        }

        for timeline in self.timelines.values_mut() {
            active |= timeline.advance(now);
        }

        active
    }

    pub fn has_active_animations(&self) -> bool {
        self.values.values().any(|value| value.active)
            || self.layouts.values().any(|layout| layout.active)
            || self
                .shared_layouts
                .values()
                .any(|layout| layout.layout.active)
            || self.styles.values().any(|style| style.active)
            || self.timelines.values().any(|timeline| timeline.active)
            || !self.pulses.is_empty()
    }

    /// Remove animation channels whose widget id no longer exists.
    pub fn retain_active<F>(&mut self, mut is_active: F)
    where
        F: FnMut(&WidgetId) -> bool,
    {
        self.values.retain(|key, _| is_active(&key.id));
        self.pulses.retain(|key, _| is_active(&key.id));
        self.styles.retain(|key, _| is_active(&key.id));
        self.layouts.retain(|key, _| is_active(&key.id));
    }

    /// Remove timeline channels whose widget path no longer exists.
    pub fn retain_timeline_paths<F>(&mut self, mut is_active: F)
    where
        F: FnMut(&WidgetPath) -> bool,
    {
        self.timelines.retain(|key, _| is_active(&key.path));
    }

    pub fn retain_shared_layouts<F>(&mut self, mut is_active: F)
    where
        F: FnMut(&str) -> bool,
    {
        self.shared_layouts
            .retain(|id, state| is_active(id) || state.layout.active);
    }
}

/// Mutable context passed to a widget's animation hook.
pub struct AnimationCtx<'a> {
    store: &'a mut AnimationStore,
    path: WidgetPath,
    id: WidgetId,
    focused_id: Option<WidgetId>,
    now: Instant,
    motion_policy: MotionPolicy,
    animation_theme: AnimationTheme,
}

impl<'a> AnimationCtx<'a> {
    pub fn new(
        store: &'a mut AnimationStore,
        path: WidgetPath,
        focused_path: Option<&'a WidgetPath>,
        now: Instant,
    ) -> Self {
        let id = WidgetId::from_path(&path);
        let focused_id = focused_path.map(WidgetId::from_path);
        Self::with_identity(
            store,
            path,
            id,
            focused_id,
            now,
            MotionPolicy::default(),
            AnimationTheme::default(),
        )
    }

    pub fn with_policy(
        store: &'a mut AnimationStore,
        path: WidgetPath,
        focused_path: Option<&'a WidgetPath>,
        now: Instant,
        motion_policy: MotionPolicy,
        animation_theme: AnimationTheme,
    ) -> Self {
        let id = WidgetId::from_path(&path);
        let focused_id = focused_path.map(WidgetId::from_path);
        Self::with_identity(
            store,
            path,
            id,
            focused_id,
            now,
            motion_policy,
            animation_theme,
        )
    }

    pub(crate) fn with_identity(
        store: &'a mut AnimationStore,
        path: WidgetPath,
        id: WidgetId,
        focused_id: Option<WidgetId>,
        now: Instant,
        motion_policy: MotionPolicy,
        animation_theme: AnimationTheme,
    ) -> Self {
        Self {
            store,
            path,
            id,
            focused_id,
            now,
            motion_policy,
            animation_theme,
        }
    }

    pub fn path(&self) -> &WidgetPath {
        &self.path
    }

    pub fn id(&self) -> &WidgetId {
        &self.id
    }

    pub fn now(&self) -> Instant {
        self.now
    }

    pub fn motion_policy(&self) -> MotionPolicy {
        self.motion_policy
    }

    pub fn animation_theme(&self) -> AnimationTheme {
        self.animation_theme
    }

    pub fn is_focused(&self) -> bool {
        self.focused_id
            .as_ref()
            .map(|id| id == &self.id)
            .unwrap_or(false)
    }

    pub fn track_value(&mut self, channel: &str, target: f64, spec: AnimationSpec) -> bool {
        let spec = self.motion_policy.effective_spec(spec);
        self.store
            .track_value(&self.id, channel, target, spec, self.now)
    }

    pub fn track_style(&mut self, channel: &str, target: Style, spec: AnimationSpec) -> bool {
        let spec = self.motion_policy.effective_spec(spec);
        self.store
            .track_style(&self.id, channel, target, spec, self.now)
    }

    pub fn pulse(&mut self, channel: &str, interval: Duration) -> bool {
        let Some(interval) = self.motion_policy.effective_interval(interval) else {
            self.store.remove_channel(&self.id, channel);
            return false;
        };

        self.store.pulse(&self.id, channel, interval, self.now)
    }
}

fn nearly_equal(a: f64, b: f64) -> bool {
    (a - b).abs() <= VALUE_EPSILON
}

fn area_nearly_equal(a: AreaF, b: AreaF) -> bool {
    nearly_equal(a.x, b.x)
        && nearly_equal(a.y, b.y)
        && nearly_equal(a.width, b.width)
        && nearly_equal(a.height, b.height)
}

fn scale_duration(duration: Duration, factor: f64) -> Duration {
    if duration.is_zero() {
        return Duration::ZERO;
    }

    Duration::from_secs_f64((duration.as_secs_f64() * factor).max(0.0))
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t
}

fn transition_is_visible_after_completion(_transition: &Transition) -> bool {
    true
}

fn collect_timeline(
    timeline: &Timeline,
    base_offset: Duration,
    transitions: &mut Vec<ScheduledTransition>,
) -> Option<Duration> {
    match timeline {
        Timeline::Single(transition) => {
            transitions.push(ScheduledTransition {
                offset: base_offset,
                transition: transition.clone(),
            });
            transition_total_duration(transition).map(|duration| base_offset + duration)
        }
        Timeline::Sequence(children) => {
            let mut cursor = base_offset;
            for child in children {
                let child_end = collect_timeline(child, cursor, transitions)?;
                cursor = child_end;
            }
            Some(cursor)
        }
        Timeline::Parallel(children) => {
            let mut end = Some(base_offset);
            for child in children {
                let child_end = collect_timeline(child, base_offset, transitions);
                end = max_optional_duration(end, child_end);
            }
            end
        }
        Timeline::Stagger { delay, children } => {
            let mut end = Some(base_offset);
            for (index, child) in children.iter().enumerate() {
                let offset =
                    base_offset + delay.saturating_mul(index.min(u32::MAX as usize) as u32);
                let child_end = collect_timeline(child, offset, transitions);
                end = max_optional_duration(end, child_end);
            }
            end
        }
    }
}

fn transition_total_duration(transition: &Transition) -> Option<Duration> {
    if matches!(transition.spec.repeat, Repeat::Forever) {
        return None;
    }

    let repeat_count = match transition.spec.repeat {
        Repeat::Never => 1,
        Repeat::Count(count) => count.max(1) as u32,
        Repeat::Forever => unreachable!(),
    };

    Some(transition.spec.delay + transition.spec.duration.saturating_mul(repeat_count))
}

fn max_optional_duration(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        _ => None,
    }
}

fn cubic_bezier_y_for_x(x: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);
    let mut t = x;

    for _ in 0..8 {
        let x_t = cubic_bezier(t, 0.0, x1, x2, 1.0);
        let dx = cubic_bezier_derivative(t, 0.0, x1, x2, 1.0);
        if dx.abs() < VALUE_EPSILON {
            break;
        }
        t = (t - (x_t - x) / dx).clamp(0.0, 1.0);
    }

    cubic_bezier(t, 0.0, y1, y2, 1.0).clamp(0.0, 1.0)
}

fn cubic_bezier(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let mt = 1.0 - t;
    mt.powi(3) * p0 + 3.0 * mt.powi(2) * t * p1 + 3.0 * mt * t.powi(2) * p2 + t.powi(3) * p3
}

fn cubic_bezier_derivative(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let mt = 1.0 - t;
    3.0 * mt.powi(2) * (p1 - p0) + 6.0 * mt * t * (p2 - p1) + 3.0 * t.powi(2) * (p3 - p2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> WidgetPath {
        WidgetPath::root().child("progress")
    }

    #[test]
    fn easing_outputs_boundaries() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
            Easing::Steps(4),
        ] {
            assert_eq!(easing.sample(0.0), 0.0);
            assert_eq!(easing.sample(1.0), 1.0);
            let mid = easing.sample(0.5);
            assert!((0.0..=1.0).contains(&mid));
        }
    }

    #[test]
    fn retargets_from_current_displayed_value() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);

        assert!(!store.track_value(&path, "value", 0.0, spec, start));
        assert!(store.track_value(&path, "value", 1.0, spec, start));
        assert!(store.track_value(&path, "value", 0.5, spec, start + Duration::from_millis(50)));

        let displayed = store.value(&path, "value").unwrap();
        assert!((displayed - 0.5).abs() < 0.001);
    }

    #[test]
    fn delay_holds_start_value_before_motion() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear)
            .delay(Duration::from_millis(50));

        store.track_value(&path, "value", 0.0, spec, start);
        assert!(store.track_value(&path, "value", 1.0, spec, start));
        assert!(store.track_value(&path, "value", 1.0, spec, start + Duration::from_millis(25)));
        assert_eq!(store.value(&path, "value"), Some(0.0));
    }

    #[test]
    fn completed_animation_stops_requesting_redraw() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);

        store.track_value(&path, "value", 0.0, spec, start);
        assert!(store.track_value(&path, "value", 1.0, spec, start));
        assert!(store.track_value(
            &path,
            "value",
            1.0,
            spec,
            start + Duration::from_millis(100)
        ));
        assert!(!store.track_value(
            &path,
            "value",
            1.0,
            spec,
            start + Duration::from_millis(120)
        ));
    }

    #[test]
    fn disabled_motion_policy_jumps_to_target() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let policy = MotionPolicy::disabled();
        let spec = policy.effective_spec(AnimationSpec::default());

        store.track_value(&path, "value", 0.0, spec, start);
        assert!(store.track_value(&path, "value", 1.0, spec, start));
        assert_eq!(store.value(&path, "value"), Some(1.0));
    }

    #[test]
    fn disabled_motion_policy_clears_pulse_channels() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();

        {
            let mut ctx = AnimationCtx::new(&mut store, path.clone(), None, start);
            assert!(ctx.pulse("spinner", Duration::from_millis(80)));
        }
        assert_eq!(store.value(&path, "spinner"), Some(0.0));

        {
            let mut ctx = AnimationCtx::with_policy(
                &mut store,
                path.clone(),
                None,
                start,
                MotionPolicy::disabled(),
                AnimationTheme::default(),
            );
            assert!(!ctx.pulse("spinner", Duration::from_millis(80)));
        }
        assert_eq!(store.value(&path, "spinner"), None);
    }

    #[test]
    fn inactive_paths_are_pruned() {
        let mut store = AnimationStore::new();
        let path = path();
        let now = Instant::now();

        store.track_value(&path, "value", 1.0, AnimationSpec::default(), now);
        store.pulse(&path, "spinner", Duration::from_millis(80), now);
        store.track_style(
            &path,
            "style",
            Style::default().fg(crate::style::Color::Red),
            AnimationSpec::default(),
            now,
        );
        store.retain_active(|_| false);

        assert!(store.value(&path, "value").is_none());
        assert!(store.value(&path, "spinner").is_none());
        assert!(store.style(&path, "style").is_none());
    }

    #[test]
    fn area_f_lerps_and_rounds_to_cell_area() {
        let start = AreaF::new(0.0, 0.0, 10.0, 2.0);
        let end = AreaF::new(10.0, 4.0, 20.0, 6.0);
        let area = start.lerp(end, 0.5).to_area();

        assert_eq!(area.x(), 5);
        assert_eq!(area.y(), 2);
        assert_eq!(area.width(), 15);
        assert_eq!(area.height(), 4);
    }

    #[test]
    fn layout_track_retargets_from_displayed_area() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);
        let transition = LayoutTransition::size_and_position(spec);
        let first = Area::new((0, 0).into(), (10, 2).into());
        let second = Area::new((10, 4).into(), (20, 6).into());

        let (area, changed) = store.track_layout(
            &path,
            "layout",
            first,
            transition,
            start,
            MotionPolicy::default(),
        );
        assert_eq!(area, first);
        assert!(!changed);

        let (area, changed) = store.track_layout(
            &path,
            "layout",
            second,
            transition,
            start,
            MotionPolicy::default(),
        );
        assert_eq!(area, first);
        assert!(changed);

        assert!(store.advance(start + Duration::from_millis(50)));
        let snapshot = store.layout_snapshot(&path, "layout").unwrap();
        assert_eq!(snapshot.displayed.to_area().x(), 5);
        assert_eq!(snapshot.displayed.to_area().width(), 15);
    }

    #[test]
    fn shared_layout_retargets_across_widget_paths() {
        let mut store = AnimationStore::new();
        let first_path = WidgetPath::root().child("list").child("row");
        let second_path = WidgetPath::root().child("detail").child("card");
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);
        let transition = LayoutTransition::size_and_position(spec);
        let first = Area::new((0, 0).into(), (10, 2).into());
        let second = Area::new((20, 4).into(), (30, 8).into());

        let (area, changed) = store.track_shared_layout(
            "selection",
            &first_path,
            first,
            transition,
            start,
            MotionPolicy::default(),
        );
        assert_eq!(area, first);
        assert!(!changed);

        let (area, changed) = store.track_shared_layout(
            "selection",
            &second_path,
            second,
            transition,
            start,
            MotionPolicy::default(),
        );
        assert_eq!(area, first);
        assert!(changed);

        assert!(store.advance(start + Duration::from_millis(50)));
        let snapshot = store.shared_layout_snapshot("selection").unwrap();
        assert_eq!(snapshot.displayed.to_area().x(), 10);
        assert_eq!(snapshot.displayed.to_area().width(), 20);
    }

    #[test]
    fn layout_transition_preserves_hit_test_mode_through_policy() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let transition = LayoutTransition::size_and_position(AnimationSpec::default())
            .hit_test(HitTestMode::Display);
        let target = Area::new((0, 0).into(), (10, 2).into());

        store.track_layout(
            &path,
            "layout",
            target,
            transition,
            start,
            MotionPolicy::reduced_motion(),
        );

        let snapshot = store.layout_snapshot(&path, "layout").unwrap();
        assert_eq!(snapshot.target, target);
        assert_eq!(transition.hit_test, HitTestMode::Display);
    }

    #[test]
    fn style_track_interpolates_rgb_colors() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);

        store.track_style(
            &path,
            "style",
            Style::default().fg(crate::style::Color::Rgb(0, 0, 0)),
            spec,
            start,
        );
        assert!(store.track_style(
            &path,
            "style",
            Style::default().fg(crate::style::Color::Rgb(100, 0, 0)),
            spec,
            start,
        ));
        assert!(store.advance(start + Duration::from_millis(50)));

        assert_eq!(
            store.style(&path, "style").unwrap().fg_color,
            Some(crate::style::Color::Rgb(50, 0, 0))
        );
    }

    #[test]
    fn timeline_sequence_activates_transitions_in_order() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);
        let timeline = Timeline::sequence(vec![
            Timeline::single(Transition::new(TransitionEffect::Expand, spec)),
            Timeline::single(Transition::new(TransitionEffect::ScaleFromCenter, spec)),
        ]);

        let (_, active) = store.track_timeline(
            &path,
            "enter",
            timeline.clone(),
            false,
            start,
            MotionPolicy::default(),
        );
        assert!(active);
        let frames = store.timeline_frames(&path, "enter", start + Duration::from_millis(50));
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].transition.effect, TransitionEffect::Expand);
        assert!((frames[0].progress - 0.5).abs() < 0.001);

        assert!(store.advance(start + Duration::from_millis(150)));
        let frames = store.timeline_frames(&path, "enter", start + Duration::from_millis(150));
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].transition.effect, TransitionEffect::Expand);
        assert!(frames[0].complete);
        assert_eq!(
            frames[1].transition.effect,
            TransitionEffect::ScaleFromCenter
        );
        assert!((frames[1].progress - 0.5).abs() < 0.001);

        assert!(
            store
                .track_timeline(
                    &path,
                    "enter",
                    timeline,
                    false,
                    start + Duration::from_millis(220),
                    MotionPolicy::default(),
                )
                .1
        );
        assert!(!store.has_active_animations());
    }

    #[test]
    fn timeline_stagger_offsets_children() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);
        let child = || Timeline::single(Transition::new(TransitionEffect::Expand, spec));
        let timeline = Timeline::stagger(Duration::from_millis(25), vec![child(), child()]);

        store.track_timeline(
            &path,
            "items",
            timeline,
            false,
            start,
            MotionPolicy::default(),
        );

        let frames = store.timeline_frames(&path, "items", start + Duration::from_millis(50));
        assert_eq!(frames.len(), 2);
        assert!((frames[0].progress - 0.5).abs() < 0.001);
        assert!((frames[1].progress - 0.25).abs() < 0.001);
    }

    #[test]
    fn timeline_active_reports_completion() {
        let mut store = AnimationStore::new();
        let path = path();
        let start = Instant::now();
        let spec = AnimationSpec::new(Duration::from_millis(100), Easing::Linear);
        let timeline = Timeline::single(Transition::new(TransitionEffect::Collapse, spec));

        store.track_timeline(
            &path,
            "exit",
            timeline,
            true,
            start,
            MotionPolicy::default(),
        );

        assert!(store.timeline_is_active(&path, "exit", start + Duration::from_millis(50)));
        assert!(!store.timeline_is_active(&path, "exit", start + Duration::from_millis(100)));
    }
}
