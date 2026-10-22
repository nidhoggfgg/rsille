//! Generic animation wrapper widgets.

use render::area::Area;

use crate::animation::{
    AnimationSpec, ClipMode, HitTestMode, InitialAnimation, LayoutTransition, Presence,
    SharedTransition, Timeline, TimelineFrame, TransitionEffect,
};
use crate::focus::FocusConfig;
use crate::layout::Constraints;
use crate::widget::{hit_area_for, IntoWidget, RenderCtx, Widget, WidgetKey};

/// Wrap a widget with framework-level animation declarations.
pub fn animate<M>(child: impl IntoWidget<M>) -> Animated<M> {
    Animated::new(child)
}

/// Generic animation wrapper.
///
/// The wrapper keeps the child identity stable under its own path and can
/// animate the area passed to the child during render. Presence declarations
/// are stored here so lifecycle-aware runtimes can honor the same widget API.
pub struct Animated<M = ()> {
    child: Box<dyn Widget<M>>,
    layout_transition: Option<LayoutTransition>,
    shared_transition: Option<SharedTransition>,
    presence: Presence,
    hit_test_mode: HitTestMode,
    widget_key: Option<String>,
}

impl<M> std::fmt::Debug for Animated<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animated")
            .field("layout_transition", &self.layout_transition)
            .field("presence", &self.presence)
            .finish()
    }
}

impl<M> Animated<M> {
    pub fn new(child: impl IntoWidget<M>) -> Self {
        Self {
            child: child.into_widget(),
            layout_transition: None,
            shared_transition: None,
            presence: Presence::default(),
            hit_test_mode: HitTestMode::Target,
            widget_key: None,
        }
    }

    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.widget_key = Some(name.into());
        self
    }

    pub fn layout(mut self, spec: AnimationSpec) -> Self {
        let transition = LayoutTransition::size_and_position(spec).hit_test(self.hit_test_mode);
        self.layout_transition = Some(transition);
        if let Some(shared) = self.shared_transition.as_mut() {
            shared.layout = transition;
        }
        self
    }

    pub fn layout_transition(mut self, transition: LayoutTransition) -> Self {
        self.hit_test_mode = transition.hit_test;
        self.layout_transition = Some(transition);
        if let Some(shared) = self.shared_transition.as_mut() {
            shared.layout = transition;
        }
        self
    }

    pub fn shared(mut self, id: impl Into<String>) -> Self {
        self.shared_transition = Some(SharedTransition::new(
            id,
            self.layout_transition.unwrap_or_else(|| {
                LayoutTransition::size_and_position(AnimationSpec::normal())
                    .hit_test(self.hit_test_mode)
            }),
        ));
        self
    }

    pub fn shared_transition(
        mut self,
        id: impl Into<String>,
        transition: LayoutTransition,
    ) -> Self {
        self.hit_test_mode = transition.hit_test;
        self.shared_transition = Some(SharedTransition::new(id, transition));
        self
    }

    pub fn hit_test_mode(mut self, mode: HitTestMode) -> Self {
        self.hit_test_mode = mode;
        if let Some(transition) = self.layout_transition.as_mut() {
            transition.hit_test = mode;
        }
        if let Some(shared) = self.shared_transition.as_mut() {
            shared.layout.hit_test = mode;
        }
        self
    }

    pub fn hit_test(self, mode: HitTestMode) -> Self {
        self.hit_test_mode(mode)
    }

    pub fn enter(mut self, timeline: impl Into<Timeline>) -> Self {
        self.presence = self.presence.enter(timeline);
        self.presence.initial = InitialAnimation::Play;
        self
    }

    pub fn exit(mut self, timeline: impl Into<Timeline>) -> Self {
        self.presence = self.presence.exit(timeline);
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = presence;
        self
    }

    pub fn presence_config(&self) -> &Presence {
        &self.presence
    }

    fn render_child(
        &self,
        chunk: &mut render::chunk::Chunk,
        ctx: &RenderCtx,
        hit_area: Option<Area>,
    ) {
        let child_ctx = ctx
            .child_ctx(WidgetKey::for_child(0, self.child.as_ref()))
            .with_hit_area(hit_area);
        self.child.render(chunk, &child_ctx);
    }

    fn render_with_area(
        &self,
        chunk: &mut render::chunk::Chunk,
        ctx: &RenderCtx,
        area: Area,
        hit_area: Option<Area>,
    ) {
        if area.width() == 0 || area.height() == 0 {
            return;
        }

        if let Ok(mut child_chunk) = chunk.from_area(area) {
            self.render_child(&mut child_chunk, ctx, hit_area);
        }
    }

    fn render_area_with_clip(
        &self,
        chunk: &mut render::chunk::Chunk,
        ctx: &RenderCtx,
        area: Area,
        clip: ClipMode,
        target: Area,
        hit_area: Option<Area>,
    ) {
        match clip {
            ClipMode::None => self.render_with_area(chunk, ctx, area, hit_area),
            ClipMode::ClipToAnimatedBounds => {
                let _ = chunk.with_clip(area, |child_chunk| {
                    self.render_child(child_chunk, ctx, hit_area)
                });
            }
            ClipMode::ClipToTargetBounds => {
                if let Some(clipped) = area.clamp_to(&target) {
                    let _ = chunk.with_clip(clipped, |child_chunk| {
                        self.render_child(child_chunk, ctx, hit_area)
                    });
                }
            }
        }
    }

    fn enter_frames(&self, ctx: &RenderCtx) -> Vec<TimelineFrame> {
        if ctx.is_exit_render() {
            return Vec::new();
        }
        let Some(timeline) = self.presence.enter.clone() else {
            return Vec::new();
        };

        if self.presence.initial == InitialAnimation::Skip {
            return Vec::new();
        }

        ctx.track_timeline("enter", timeline, false)
    }
}

impl<M> Widget<M> for Animated<M> {
    fn render(&self, chunk: &mut render::chunk::Chunk, ctx: &RenderCtx) {
        let target = ctx.layout_target().unwrap_or_else(|| chunk.area());
        if target.width() == 0 || target.height() == 0 {
            return;
        }

        ctx.record_bounds(target);

        let (mut display_area, clip, hit_test_mode) =
            if let Some(shared) = self.shared_transition.as_ref() {
                if ctx.layout_is_managed() {
                    (chunk.area(), shared.layout.clip, shared.layout.hit_test)
                } else {
                    (
                        ctx.track_shared_layout(&shared.id, target, shared.layout),
                        shared.layout.clip,
                        shared.layout.hit_test,
                    )
                }
            } else if let Some(transition) = self.layout_transition {
                if ctx.layout_is_managed() {
                    (chunk.area(), transition.clip, transition.hit_test)
                } else {
                    (
                        ctx.track_layout("layout", target, transition),
                        transition.clip,
                        transition.hit_test,
                    )
                }
            } else {
                (target, ClipMode::None, self.hit_test_mode)
            };

        for frame in self.enter_frames(ctx) {
            display_area = apply_enter_effect(display_area, &frame);
        }

        let hit_area = hit_area_for(hit_test_mode, target, display_area);
        ctx.record_hit_bounds(target, display_area, hit_test_mode);
        self.render_area_with_clip(chunk, ctx, display_area, clip, target, hit_area);
    }

    fn constraints(&self) -> Constraints {
        self.child.constraints()
    }

    fn presence(&self) -> Option<&Presence> {
        Some(&self.presence)
    }

    fn layout_transition(&self) -> Option<LayoutTransition> {
        self.layout_transition
    }

    fn shared_transition(&self) -> Option<&SharedTransition> {
        self.shared_transition.as_ref()
    }

    fn hit_test_mode(&self) -> HitTestMode {
        self.hit_test_mode
    }

    fn focus_config(&self) -> FocusConfig {
        FocusConfig::None
    }

    fn children(&self) -> &[Box<dyn Widget<M>>] {
        std::slice::from_ref(&self.child)
    }

    fn key(&self) -> Option<&str> {
        self.widget_key.as_deref()
    }
}

fn apply_enter_effect(area: Area, frame: &TimelineFrame) -> Area {
    match frame.transition.effect {
        TransitionEffect::Collapse | TransitionEffect::Expand => {
            vertical_reveal(area, frame.progress)
        }
        TransitionEffect::ScaleFromCenter => scale_from_center(area, frame.progress),
        TransitionEffect::Layout(_) | TransitionEffect::Fade | TransitionEffect::BorderEmphasis => {
            area
        }
    }
}

fn vertical_reveal(area: Area, progress: f64) -> Area {
    let height = ((area.height() as f64) * progress.clamp(0.0, 1.0)).round() as u16;
    Area::new(area.pos(), (area.width(), height).into())
}

fn scale_from_center(area: Area, progress: f64) -> Area {
    let progress = progress.clamp(0.0, 1.0);
    let width = ((area.width() as f64) * progress).round() as u16;
    let height = ((area.height() as f64) * progress).round() as u16;
    let x = area.x() + area.width().saturating_sub(width) / 2;
    let y = area.y() + area.height().saturating_sub(height) / 2;

    Area::new((x, y).into(), (width, height).into())
}
