//! Theme system for global styling

use super::{Color, Style};
use crate::animation::AnimationTheme;
use crate::widgets::visual::{GradientDirection, VisualAnchor, VisualEffect, WipeDirection};

#[derive(Debug, Clone, Copy)]
struct Palette {
    primary: Color,
    secondary: Color,
    danger: Color,
    info: Color,
    success: Color,
    warning: Color,
    action_focus_foreground: Color,
    text: Color,
    text_muted: Color,
    background: Color,
    surface: Color,
    surface_elevated: Color,
    surface_modal: Color,
    surface_popup: Color,
    surface_tooltip: Color,
    border: Color,
    focus_ring: Color,
    focus_background: Color,
    selected_foreground: Color,
    selected_background: Color,
    selected_focused_foreground: Color,
    selected_focused_background: Color,
    active_foreground: Color,
    active_background: Color,
    active_focused_foreground: Color,
    active_focused_background: Color,
    overlay_backdrop: Color,
    scrollbar_track: Color,
    scrollbar_thumb: Color,
}

impl Palette {
    fn dark() -> Self {
        Self {
            primary: Color::Rgb(96, 165, 250),
            secondary: Color::Rgb(168, 85, 247),
            danger: Color::Rgb(248, 113, 113),
            info: Color::Rgb(34, 211, 238),
            success: Color::Rgb(74, 222, 128),
            warning: Color::Rgb(251, 191, 36),
            action_focus_foreground: Color::Rgb(24, 24, 27),
            text: Color::Rgb(244, 244, 245),
            text_muted: Color::Rgb(161, 161, 170),
            background: Color::Rgb(24, 24, 27),
            surface: Color::Rgb(39, 39, 42),
            surface_elevated: Color::Rgb(55, 55, 61),
            surface_modal: Color::Rgb(63, 63, 70),
            surface_popup: Color::Rgb(49, 49, 56),
            surface_tooltip: Color::Rgb(17, 24, 39),
            border: Color::Rgb(82, 82, 91),
            focus_ring: Color::Rgb(147, 197, 253),
            focus_background: Color::Rgb(30, 64, 175),
            selected_foreground: Color::White,
            selected_background: Color::Rgb(37, 99, 235),
            selected_focused_foreground: Color::White,
            selected_focused_background: Color::Rgb(59, 130, 246),
            active_foreground: Color::Rgb(244, 244, 245),
            active_background: Color::Rgb(63, 63, 70),
            active_focused_foreground: Color::White,
            active_focused_background: Color::Rgb(37, 99, 235),
            overlay_backdrop: Color::Rgb(9, 9, 11),
            scrollbar_track: Color::Rgb(63, 63, 70),
            scrollbar_thumb: Color::Rgb(96, 165, 250),
        }
    }

    fn light() -> Self {
        Self {
            primary: Color::Rgb(79, 70, 229),
            secondary: Color::Rgb(124, 58, 237),
            danger: Color::Rgb(220, 38, 38),
            info: Color::Rgb(2, 132, 199),
            success: Color::Rgb(22, 163, 74),
            warning: Color::Rgb(217, 119, 6),
            action_focus_foreground: Color::Rgb(24, 24, 27),
            text: Color::Rgb(24, 24, 27),
            text_muted: Color::Rgb(113, 113, 122),
            background: Color::Rgb(250, 250, 250),
            surface: Color::Rgb(255, 255, 255),
            surface_elevated: Color::Rgb(244, 244, 245),
            surface_modal: Color::Rgb(255, 255, 255),
            surface_popup: Color::Rgb(248, 250, 252),
            surface_tooltip: Color::Rgb(39, 39, 42),
            border: Color::Rgb(212, 212, 216),
            focus_ring: Color::Rgb(67, 56, 202),
            focus_background: Color::Rgb(224, 231, 255),
            selected_foreground: Color::White,
            selected_background: Color::Rgb(79, 70, 229),
            selected_focused_foreground: Color::White,
            selected_focused_background: Color::Rgb(67, 56, 202),
            active_foreground: Color::Rgb(24, 24, 27),
            active_background: Color::Rgb(224, 231, 255),
            active_focused_foreground: Color::White,
            active_focused_background: Color::Rgb(79, 70, 229),
            overlay_backdrop: Color::Rgb(228, 228, 231),
            scrollbar_track: Color::Rgb(212, 212, 216),
            scrollbar_thumb: Color::Rgb(79, 70, 229),
        }
    }

    fn one_dark() -> Self {
        Self {
            primary: Color::Rgb(97, 175, 239),
            secondary: Color::Rgb(198, 120, 221),
            danger: Color::Rgb(224, 108, 117),
            info: Color::Rgb(86, 182, 194),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            action_focus_foreground: Color::Rgb(40, 44, 52),
            text: Color::Rgb(171, 178, 191),
            text_muted: Color::Rgb(130, 137, 151),
            background: Color::Rgb(40, 44, 52),
            surface: Color::Rgb(44, 49, 60),
            surface_elevated: Color::Rgb(53, 59, 69),
            surface_modal: Color::Rgb(62, 68, 81),
            surface_popup: Color::Rgb(49, 54, 64),
            surface_tooltip: Color::Rgb(33, 37, 43),
            border: Color::Rgb(92, 99, 112),
            focus_ring: Color::Rgb(97, 175, 239),
            focus_background: Color::Rgb(49, 88, 120),
            selected_foreground: Color::Rgb(40, 44, 52),
            selected_background: Color::Rgb(97, 175, 239),
            selected_focused_foreground: Color::Rgb(40, 44, 52),
            selected_focused_background: Color::Rgb(86, 182, 194),
            active_foreground: Color::Rgb(229, 232, 239),
            active_background: Color::Rgb(62, 68, 81),
            active_focused_foreground: Color::Rgb(40, 44, 52),
            active_focused_background: Color::Rgb(97, 175, 239),
            overlay_backdrop: Color::Rgb(33, 37, 43),
            scrollbar_track: Color::Rgb(62, 68, 81),
            scrollbar_thumb: Color::Rgb(97, 175, 239),
        }
    }

    fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),
            secondary: Color::Rgb(255, 121, 198),
            danger: Color::Rgb(255, 85, 85),
            info: Color::Rgb(139, 233, 253),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            action_focus_foreground: Color::Rgb(40, 42, 54),
            text: Color::Rgb(248, 248, 242),
            text_muted: Color::Rgb(189, 147, 249),
            background: Color::Rgb(40, 42, 54),
            surface: Color::Rgb(48, 50, 65),
            surface_elevated: Color::Rgb(68, 71, 90),
            surface_modal: Color::Rgb(73, 77, 100),
            surface_popup: Color::Rgb(56, 59, 76),
            surface_tooltip: Color::Rgb(25, 26, 34),
            border: Color::Rgb(98, 114, 164),
            focus_ring: Color::Rgb(139, 233, 253),
            focus_background: Color::Rgb(68, 71, 90),
            selected_foreground: Color::Rgb(40, 42, 54),
            selected_background: Color::Rgb(189, 147, 249),
            selected_focused_foreground: Color::Rgb(40, 42, 54),
            selected_focused_background: Color::Rgb(139, 233, 253),
            active_foreground: Color::Rgb(248, 248, 242),
            active_background: Color::Rgb(68, 71, 90),
            active_focused_foreground: Color::Rgb(40, 42, 54),
            active_focused_background: Color::Rgb(189, 147, 249),
            overlay_backdrop: Color::Rgb(25, 26, 34),
            scrollbar_track: Color::Rgb(68, 71, 90),
            scrollbar_thumb: Color::Rgb(189, 147, 249),
        }
    }

    fn tokyo_night() -> Self {
        Self {
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(187, 154, 247),
            danger: Color::Rgb(247, 118, 142),
            info: Color::Rgb(125, 207, 255),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            action_focus_foreground: Color::Rgb(26, 27, 38),
            text: Color::Rgb(192, 202, 245),
            text_muted: Color::Rgb(169, 177, 214),
            background: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(31, 35, 53),
            surface_elevated: Color::Rgb(36, 40, 59),
            surface_modal: Color::Rgb(41, 46, 66),
            surface_popup: Color::Rgb(35, 38, 56),
            surface_tooltip: Color::Rgb(22, 22, 30),
            border: Color::Rgb(65, 72, 104),
            focus_ring: Color::Rgb(125, 207, 255),
            focus_background: Color::Rgb(46, 60, 100),
            selected_foreground: Color::Rgb(26, 27, 38),
            selected_background: Color::Rgb(122, 162, 247),
            selected_focused_foreground: Color::Rgb(26, 27, 38),
            selected_focused_background: Color::Rgb(125, 207, 255),
            active_foreground: Color::Rgb(192, 202, 245),
            active_background: Color::Rgb(51, 70, 124),
            active_focused_foreground: Color::Rgb(26, 27, 38),
            active_focused_background: Color::Rgb(122, 162, 247),
            overlay_backdrop: Color::Rgb(15, 15, 21),
            scrollbar_track: Color::Rgb(41, 46, 66),
            scrollbar_thumb: Color::Rgb(122, 162, 247),
        }
    }
}

/// Semantic style roles for theming
///
/// This structure provides semantic styling that can be used by any widget,
/// including user-defined custom widgets. Instead of hardcoding styles for
/// specific components, we provide semantic roles based on the purpose and
/// context of UI elements.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeStyles {
    // === Action Styles ===
    /// Primary action style (e.g., primary buttons, key actions)
    pub primary_action: Style,
    /// Primary action hover state
    pub primary_action_hover: Style,
    /// Primary action focused state
    pub primary_action_focused: Style,
    /// Secondary action style (e.g., secondary buttons)
    pub secondary_action: Style,
    /// Secondary action hover state
    pub secondary_action_hover: Style,
    /// Secondary action focused state
    pub secondary_action_focused: Style,
    /// Destructive action style (e.g., dangerous buttons)
    pub destructive_action: Style,
    /// Destructive action hover state
    pub destructive_action_hover: Style,
    /// Destructive action focused state
    pub destructive_action_focused: Style,

    // === Interactive Element Styles ===
    /// Interactive element style (e.g., inputs, checkboxes, sliders)
    pub interactive: Style,
    /// Interactive element focused state
    pub interactive_focused: Style,
    /// Interactive element disabled state
    pub interactive_disabled: Style,

    // === Text Styles ===
    /// Regular text style (e.g., labels, paragraphs)
    pub text: Style,
    /// Muted/secondary text style
    pub text_muted: Style,
    /// Placeholder text style
    pub text_placeholder: Style,
    /// Heading text style
    pub text_heading: Style,

    // === Container Styles ===
    /// Surface style (e.g., container backgrounds)
    pub surface: Style,
    /// Elevated surface style (e.g., modals, popups, cards)
    pub surface_elevated: Style,
    /// Header surface style for table/calendar headings
    pub surface_header: Style,
    /// Modal/dialog surface style
    pub surface_modal: Style,
    /// Popup/popover surface style
    pub surface_popup: Style,
    /// Tooltip surface style
    pub surface_tooltip: Style,
    /// Backdrop style used behind overlay layers
    pub overlay_backdrop: Style,

    // === State Styles ===
    /// Selected/highlighted state
    pub selected: Style,
    /// Selected state while the widget is focused
    pub selected_focused: Style,
    /// Active list row when the widget is not focused
    pub list_active: Style,
    /// Active list row when the widget is focused
    pub list_active_focused: Style,
    /// Informational feedback state
    pub status_info: Style,
    /// Success feedback state
    pub status_success: Style,
    /// Warning feedback state
    pub status_warning: Style,
    /// Error/destructive feedback state
    pub status_error: Style,
    /// Hover state (generic)
    pub hover: Style,
    /// Disabled state (generic)
    pub disabled: Style,
    /// Default border style
    pub border: Style,
    /// Focused border style
    pub border_focused: Style,
    /// Text cursor style
    pub cursor: Style,

    // === Editing & Validation Styles ===
    /// Selection range highlight for editable text/content
    pub selection_range: Style,
    /// Informational validation or helper emphasis
    pub validation_info: Style,
    /// Success validation or helper emphasis
    pub validation_success: Style,
    /// Warning validation or helper emphasis
    pub validation_warning: Style,
    /// Error validation or helper emphasis
    pub validation_error: Style,

    // === Menu & Scroll Styles ===
    /// Hovered menu item style
    pub menu_item_hover: Style,
    /// Active menu item when the menu is not focused
    pub menu_item_active: Style,
    /// Active menu item when the menu is focused
    pub menu_item_active_focused: Style,
    /// Scrollbar track style
    pub scrollbar_track: Style,
    /// Scrollbar thumb style
    pub scrollbar_thumb: Style,
}

impl ThemeStyles {
    /// Create semantic styles for dark theme
    pub fn dark() -> Self {
        let palette = Palette::dark();
        Self::from_palette(palette, palette.text, palette.background)
    }

    /// Create semantic styles for light theme
    pub fn light() -> Self {
        let palette = Palette::light();
        Self::from_palette(palette, Color::White, Color::White)
    }

    /// Create semantic styles for the One Dark theme.
    pub fn one_dark() -> Self {
        let palette = Palette::one_dark();
        Self::from_palette(palette, palette.background, palette.background)
    }

    /// Create semantic styles for the Dracula theme.
    pub fn dracula() -> Self {
        let palette = Palette::dracula();
        Self::from_palette(palette, palette.background, palette.background)
    }

    /// Create semantic styles for the Tokyo Night theme.
    pub fn tokyo_night() -> Self {
        let palette = Palette::tokyo_night();
        Self::from_palette(palette, palette.background, palette.background)
    }

    fn from_palette(palette: Palette, action_fg: Color, status_fg: Color) -> Self {
        Self {
            // Action styles
            primary_action: Style::default().fg(action_fg).bg(palette.primary),
            primary_action_hover: Style::default()
                .fg(palette.selected_focused_foreground)
                .bg(palette.selected_focused_background)
                .bold(),
            primary_action_focused: Style::default()
                .fg(palette.action_focus_foreground)
                .bg(palette.warning)
                .bold(),
            secondary_action: Style::default().fg(action_fg).bg(palette.secondary),
            secondary_action_hover: Style::default()
                .fg(palette.active_focused_foreground)
                .bg(palette.active_focused_background)
                .bold(),
            secondary_action_focused: Style::default()
                .fg(palette.action_focus_foreground)
                .bg(palette.warning)
                .bold(),
            destructive_action: Style::default().fg(Color::White).bg(palette.danger),
            destructive_action_hover: Style::default().fg(Color::White).bg(palette.danger).bold(),
            destructive_action_focused: Style::default()
                .fg(palette.action_focus_foreground)
                .bg(palette.warning)
                .bold(),

            // Interactive element styles
            interactive: Style::default().fg(palette.text).bg(palette.surface),
            interactive_focused: Style::default()
                .fg(palette.text)
                .bg(palette.surface_elevated)
                .bold(),
            interactive_disabled: Style::default().fg(palette.text_muted).bg(palette.surface),

            // Text styles
            text: Style::default().fg(palette.text),
            text_muted: Style::default().fg(palette.text_muted),
            text_placeholder: Style::default().fg(palette.text_muted),
            text_heading: Style::default().fg(palette.text).bold(),

            // Container styles
            surface: Style::default().bg(palette.background).fg(palette.text),
            surface_elevated: Style::default()
                .bg(palette.surface_elevated)
                .fg(palette.text),
            surface_header: Style::default()
                .bg(palette.surface_elevated)
                .fg(palette.text)
                .bold(),
            surface_modal: Style::default().bg(palette.surface_modal).fg(palette.text),
            surface_popup: Style::default().bg(palette.surface_popup).fg(palette.text),
            surface_tooltip: Style::default().fg(action_fg).bg(palette.surface_tooltip),
            overlay_backdrop: Style::default().bg(palette.overlay_backdrop),

            // State styles
            selected: Style::default()
                .fg(palette.selected_foreground)
                .bg(palette.selected_background)
                .bold(),
            selected_focused: Style::default()
                .fg(palette.selected_focused_foreground)
                .bg(palette.selected_focused_background)
                .bold(),
            list_active: Style::default()
                .fg(palette.active_foreground)
                .bg(palette.active_background)
                .bold(),
            list_active_focused: Style::default()
                .fg(palette.active_focused_foreground)
                .bg(palette.active_focused_background)
                .bold(),
            status_info: Style::default().fg(status_fg).bg(palette.info).bold(),
            status_success: Style::default().fg(status_fg).bg(palette.success).bold(),
            status_warning: Style::default().fg(status_fg).bg(palette.warning).bold(),
            status_error: Style::default().fg(status_fg).bg(palette.danger).bold(),
            hover: Style::default().fg(action_fg).bg(palette.primary).bold(),
            disabled: Style::default().fg(palette.text_muted),
            border: Style::default().fg(palette.border),
            border_focused: Style::default().fg(palette.focus_ring),
            cursor: Style::default().fg(palette.background).bg(palette.text),
            selection_range: Style::default().bg(palette.focus_background),
            validation_info: Style::default().fg(palette.info).bold(),
            validation_success: Style::default().fg(palette.success).bold(),
            validation_warning: Style::default().fg(palette.warning).bold(),
            validation_error: Style::default().fg(palette.danger).bold(),
            menu_item_hover: Style::default()
                .fg(palette.active_foreground)
                .bg(palette.active_background)
                .bold(),
            menu_item_active: Style::default()
                .fg(palette.active_foreground)
                .bg(palette.active_background)
                .bold(),
            menu_item_active_focused: Style::default()
                .fg(palette.active_focused_foreground)
                .bg(palette.active_focused_background)
                .bold(),
            scrollbar_track: Style::default()
                .fg(palette.scrollbar_track)
                .bg(palette.surface),
            scrollbar_thumb: Style::default()
                .fg(palette.scrollbar_thumb)
                .bg(palette.surface_elevated),
        }
    }
}

/// A complete theme definition
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Shared terminal text width and tab-stop policy.
    pub text_metrics: render::text::TextMetrics,
    /// Theme name
    pub name: String,
    /// Semantic style roles
    pub styles: ThemeStyles,
    /// Theme-level animation timing defaults.
    pub animations: AnimationTheme,
    /// Theme-level defaults for visual post-processing effects.
    pub effects: ThemeEffects,
}

impl Theme {
    /// Create a new theme with the given name and semantic styles
    pub fn new(name: impl Into<String>, styles: ThemeStyles) -> Self {
        Self {
            text_metrics: render::text::TextMetrics::default(),
            name: name.into(),
            styles,
            animations: AnimationTheme::default(),
            effects: ThemeEffects::default(),
        }
    }

    pub fn with_text_metrics(mut self, metrics: render::text::TextMetrics) -> Self {
        self.text_metrics = metrics;
        self
    }

    /// Set theme-level animation defaults.
    pub fn with_animations(mut self, animations: AnimationTheme) -> Self {
        self.animations = animations;
        self
    }

    /// Set theme-level visual effect defaults.
    pub fn with_effects(mut self, effects: ThemeEffects) -> Self {
        self.effects = effects;
        self
    }

    /// Create the built-in dark theme
    pub fn dark() -> Self {
        Self::new("dark", ThemeStyles::dark())
    }

    /// Create the built-in light theme
    pub fn light() -> Self {
        Self::new("light", ThemeStyles::light())
    }

    /// Create the built-in One Dark theme.
    pub fn one_dark() -> Self {
        Self::new("one-dark", ThemeStyles::one_dark())
    }

    /// Create the built-in Dracula theme.
    pub fn dracula() -> Self {
        Self::new("dracula", ThemeStyles::dracula())
    }

    /// Create the built-in Tokyo Night theme.
    pub fn tokyo_night() -> Self {
        Self::new("tokyo-night", ThemeStyles::tokyo_night())
    }

    /// Create a theme builder for custom themes
    pub fn builder() -> ThemeBuilder {
        ThemeBuilder::new()
    }
}

/// Builder for creating custom themes
#[derive(Debug)]
pub struct ThemeBuilder {
    name: String,
    styles: Option<ThemeStyles>,
    animations: Option<AnimationTheme>,
    effects: Option<ThemeEffects>,
}

impl ThemeBuilder {
    /// Create a new theme builder
    pub fn new() -> Self {
        Self {
            name: "custom".to_string(),
            styles: None,
            animations: None,
            effects: None,
        }
    }

    /// Set the theme name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the style roles
    pub fn styles(mut self, styles: ThemeStyles) -> Self {
        self.styles = Some(styles);
        self
    }

    /// Set animation timing defaults.
    pub fn animations(mut self, animations: AnimationTheme) -> Self {
        self.animations = Some(animations);
        self
    }

    /// Set visual effect defaults.
    pub fn effects(mut self, effects: ThemeEffects) -> Self {
        self.effects = Some(effects);
        self
    }

    /// Build the theme, using dark theme defaults for unset fields
    pub fn build(self) -> Theme {
        let styles = self.styles.unwrap_or_else(ThemeStyles::dark);
        let animations = self.animations.unwrap_or_default();
        let effects = self.effects.unwrap_or_default();
        Theme::new(self.name, styles)
            .with_animations(animations)
            .with_effects(effects)
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme-level defaults for visual post-processing effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeEffects {
    /// Terminal cell width divided by height as used by geometry effects.
    ///
    /// The default keeps historical behavior. Applications can tune this to
    /// match their terminal font, for example `0.5` for narrow cells.
    pub cell_aspect: f64,
    /// Default visual effect for modal/dialog entry.
    pub modal_enter: VisualEffect,
    /// Default visual effect for modal/dialog exit.
    pub modal_exit: VisualEffect,
    /// Default visual effect for toast/notification entry.
    pub toast_enter: VisualEffect,
    /// Default visual effect for toast/notification exit.
    pub toast_exit: VisualEffect,
    /// Default visual effect for focus emphasis.
    pub focus_pulse: VisualEffect,
    /// Default visual effect for full-screen or route transitions.
    pub screen_transition: VisualEffect,
}

impl Default for ThemeEffects {
    fn default() -> Self {
        Self {
            cell_aspect: 1.0,
            modal_enter: VisualEffect::parallel(vec![
                VisualEffect::fade_in(),
                VisualEffect::magic_lamp(VisualAnchor::Center).squeeze(0.18),
            ]),
            modal_exit: VisualEffect::parallel(vec![
                VisualEffect::fade_out(),
                VisualEffect::magic_lamp(VisualAnchor::Center).squeeze(0.18),
            ]),
            toast_enter: VisualEffect::stagger_rows(
                0.04,
                VisualEffect::reveal(WipeDirection::LeftToRight).softness(0.08),
            ),
            toast_exit: VisualEffect::dissolve_out(),
            focus_pulse: VisualEffect::highlight_sweep()
                .color(Color::Rgb(255, 255, 180))
                .width(0.22)
                .direction(GradientDirection::Horizontal),
            screen_transition: VisualEffect::reveal(WipeDirection::TopToBottom).softness(0.05),
        }
    }
}

impl ThemeEffects {
    /// Override the terminal cell aspect used by geometry effects.
    pub fn cell_aspect(mut self, cell_aspect: f64) -> Self {
        self.cell_aspect = sanitize_cell_aspect(cell_aspect);
        self
    }

    /// Override the modal enter preset.
    pub fn modal_enter(mut self, effect: VisualEffect) -> Self {
        self.modal_enter = effect;
        self
    }

    /// Override the modal exit preset.
    pub fn modal_exit(mut self, effect: VisualEffect) -> Self {
        self.modal_exit = effect;
        self
    }

    /// Override the toast enter preset.
    pub fn toast_enter(mut self, effect: VisualEffect) -> Self {
        self.toast_enter = effect;
        self
    }

    /// Override the toast exit preset.
    pub fn toast_exit(mut self, effect: VisualEffect) -> Self {
        self.toast_exit = effect;
        self
    }

    /// Override the focus pulse preset.
    pub fn focus_pulse(mut self, effect: VisualEffect) -> Self {
        self.focus_pulse = effect;
        self
    }

    /// Override the screen transition preset.
    pub fn screen_transition(mut self, effect: VisualEffect) -> Self {
        self.screen_transition = effect;
        self
    }

    /// Resolve a named theme effect slot at render time.
    pub fn get(&self, slot: EffectSlot) -> VisualEffect {
        match slot {
            EffectSlot::ModalEnter => self.modal_enter.clone(),
            EffectSlot::ModalExit => self.modal_exit.clone(),
            EffectSlot::ToastEnter => self.toast_enter.clone(),
            EffectSlot::ToastExit => self.toast_exit.clone(),
            EffectSlot::FocusPulse => self.focus_pulse.clone(),
            EffectSlot::ScreenTransition => self.screen_transition.clone(),
        }
    }
}

/// Named visual effect presets supplied by [`ThemeEffects`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectSlot {
    ModalEnter,
    ModalExit,
    ToastEnter,
    ToastExit,
    FocusPulse,
    ScreenTransition,
}

fn sanitize_cell_aspect(cell_aspect: f64) -> f64 {
    if cell_aspect.is_finite() {
        cell_aspect.max(f64::EPSILON)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_creation() {
        let theme = Theme::dark();
        assert_eq!(theme.name, "dark");
    }

    #[test]
    fn test_light_theme_creation() {
        let theme = Theme::light();
        assert_eq!(theme.name, "light");
    }

    #[test]
    fn test_named_theme_creation() {
        assert_eq!(Theme::one_dark().name, "one-dark");
        assert_eq!(Theme::dracula().name, "dracula");
        assert_eq!(Theme::tokyo_night().name, "tokyo-night");
    }

    #[test]
    fn test_selection_states_are_visually_distinct() {
        for styles in [
            ThemeStyles::dark(),
            ThemeStyles::light(),
            ThemeStyles::one_dark(),
            ThemeStyles::dracula(),
            ThemeStyles::tokyo_night(),
        ] {
            assert_ne!(styles.selected.bg_color, styles.surface.bg_color);
            assert_ne!(styles.selected_focused.bg_color, styles.surface.bg_color);
            assert_ne!(styles.list_active_focused.bg_color, styles.surface.bg_color);
            assert!(styles.selected.modifiers.contains_bold());
            assert!(styles.selected_focused.modifiers.contains_bold());
        }
    }

    #[test]
    fn test_button_focus_states_are_visually_distinct() {
        for styles in [
            ThemeStyles::dark(),
            ThemeStyles::light(),
            ThemeStyles::one_dark(),
            ThemeStyles::dracula(),
            ThemeStyles::tokyo_night(),
        ] {
            assert_ne!(
                styles.primary_action_focused.bg_color,
                styles.primary_action.bg_color
            );
            assert_ne!(
                styles.secondary_action_focused.bg_color,
                styles.secondary_action.bg_color
            );
            assert_ne!(
                styles.destructive_action_focused.bg_color,
                styles.destructive_action.bg_color
            );
            assert_ne!(
                styles.primary_action_hover.bg_color,
                styles.primary_action.bg_color
            );
            assert!(styles.primary_action_focused.modifiers.contains_bold());
            assert!(styles.secondary_action_focused.modifiers.contains_bold());
            assert!(styles.destructive_action_focused.modifiers.contains_bold());
        }
    }

    #[test]
    fn test_custom_theme_builder() {
        let theme = Theme::builder()
            .name("custom")
            .styles(ThemeStyles::dark())
            .build();
        assert_eq!(theme.name, "custom");
    }

    #[test]
    fn test_theme_builder_accepts_animation_defaults() {
        let animations = AnimationTheme {
            fast: crate::animation::AnimationSpec::new(
                std::time::Duration::from_millis(50),
                crate::animation::Easing::Linear,
            ),
            ..AnimationTheme::default()
        };
        let theme = Theme::builder().animations(animations).build();

        assert_eq!(
            theme.animations.fast.duration,
            std::time::Duration::from_millis(50)
        );
    }

    #[test]
    fn test_theme_builder_accepts_effect_defaults() {
        let effects = ThemeEffects::default().cell_aspect(0.5);
        let theme = Theme::builder().effects(effects).build();

        assert_eq!(theme.effects.cell_aspect, 0.5);
    }

    #[test]
    fn test_theme_effect_slots_have_defaults() {
        let effects = ThemeEffects::default();

        assert_eq!(effects.get(EffectSlot::ModalEnter), effects.modal_enter);
        assert_eq!(effects.get(EffectSlot::ModalExit), effects.modal_exit);
        assert_eq!(effects.get(EffectSlot::ToastEnter), effects.toast_enter);
        assert_eq!(effects.get(EffectSlot::ToastExit), effects.toast_exit);
        assert_eq!(effects.get(EffectSlot::FocusPulse), effects.focus_pulse);
        assert_eq!(
            effects.get(EffectSlot::ScreenTransition),
            effects.screen_transition
        );
    }

    #[test]
    fn test_theme_effect_slots_are_overridable() {
        let effects =
            ThemeEffects::default().modal_enter(crate::widgets::visual::VisualEffect::reveal(
                crate::widgets::visual::WipeDirection::CenterOut,
            ));

        assert_eq!(effects.get(EffectSlot::ModalEnter), effects.modal_enter);
    }

    #[test]
    fn test_extended_semantic_tokens_are_configured() {
        let styles = ThemeStyles::dark();

        assert!(!styles.surface_modal.is_empty());
        assert!(!styles.surface_popup.is_empty());
        assert!(!styles.surface_tooltip.is_empty());
        assert!(!styles.overlay_backdrop.is_empty());
        assert!(!styles.status_info.is_empty());
        assert!(!styles.status_success.is_empty());
        assert!(!styles.status_warning.is_empty());
        assert!(!styles.status_error.is_empty());
        assert!(!styles.validation_error.is_empty());
        assert!(!styles.menu_item_active_focused.is_empty());
        assert!(!styles.scrollbar_track.is_empty());
        assert!(!styles.scrollbar_thumb.is_empty());
    }
}
