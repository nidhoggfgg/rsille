use crossterm::style::{Attribute, Attributes, Color, Colors};
use rsille::{
    canvas::{Canvas, Encoding},
    render::{
        area::{Area, Size},
        chunk::Chunk,
        style::Style,
        text::TextMetrics,
        Draw, DrawErr,
    },
};

use crate::doctor::RasterSupport;

use super::{
    fx,
    scenes::{ExperienceId, SceneEngine, EXPERIENCE_COUNT},
    state::{ExperienceSpec, HitRegions, LauncherState, WIDE_LAYOUT_MIN_WIDTH},
};

// Cohesive dark midnight studio surface hierarchy.
const BG_SHELL: [u8; 3] = [9, 12, 20];
const BG_HEADER: [u8; 3] = [13, 18, 30];
const BG_CARD: [u8; 3] = [13, 18, 30];
const BG_CARD_HOVER: [u8; 3] = [19, 26, 42];
const BG_STAGE_CANVAS: [u8; 3] = [8, 11, 19];
const BG_HUD_PILL: [u8; 3] = [12, 17, 29];
const BG_CMD_PILL: [u8; 3] = [18, 25, 40];
const BG_DOCK: [u8; 3] = [11, 15, 25];
const BG_KEYCAP: [u8; 3] = [27, 36, 56];

const FG_PRIMARY: [u8; 3] = [242, 247, 255];
const FG_SECONDARY: [u8; 3] = [176, 190, 214];
const FG_MUTED: [u8; 3] = [112, 127, 155];
const FG_DIM: [u8; 3] = [74, 88, 116];
const BORDER_DIM: [u8; 3] = [44, 56, 82];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Compact,
}

#[derive(Clone, Copy, Debug)]
pub struct ComputedLayout {
    pub mode: LayoutMode,
    pub header: Area,
    pub header_divider_y: Option<u16>,
    pub rail_header: Option<Area>,
    pub rail_footer: Option<Area>,
    pub regions: HitRegions,
    pub dock: Area,
}

impl ComputedLayout {
    pub fn compute(size: Size, selected: ExperienceId) -> Self {
        let w = size.width;
        let h = size.height;
        let empty = Self {
            mode: if w >= WIDE_LAYOUT_MIN_WIDTH {
                LayoutMode::Wide
            } else {
                LayoutMode::Compact
            },
            header: Area::default(),
            header_divider_y: None,
            rail_header: None,
            rail_footer: None,
            dock: Area::default(),
            regions: HitRegions {
                size,
                ..HitRegions::default()
            },
        };
        if w < 10 || h < 4 {
            return empty;
        }

        let wide = w >= WIDE_LAYOUT_MIN_WIDTH && h >= 10;
        let mode = if wide {
            LayoutMode::Wide
        } else {
            LayoutMode::Compact
        };

        let header_h: u16 = if wide && h >= 16 { 3 } else { 2 };
        let header_divider_y = (header_h == 3).then_some(2);
        let dock_h: u16 = if h >= header_h + 2 { 1 } else { 0 };
        let middle_y = header_h;
        let middle_h = h.saturating_sub(header_h + dock_h);

        let header = Area::new((0, 0).into(), (w, header_h).into());
        let dock = if dock_h > 0 {
            Area::new((0, h - 1).into(), (w, 1).into())
        } else {
            Area::default()
        };

        let pill_w = if w >= 52 { 15u16 } else { 10u16 }.min(w / 2);
        let margin_x: u16 = if w >= 50 { 1 } else { 0 };
        let tour_pill = Area::new(
            (w.saturating_sub(margin_x + pill_w), 0).into(),
            (pill_w, 1).into(),
        );

        let mut cards = [Area::default(); EXPERIENCE_COUNT];
        let mut stage = Area::default();
        let mut rail_header = None;
        let mut rail_footer = None;

        if wide {
            let outer_x = if w >= 82 { 1u16 } else { 0u16 };
            let inner_w = w.saturating_sub(outer_x * 2);
            let gap = if w >= 104 { 2u16 } else { 1u16 };
            let max_rail = inner_w.saturating_sub(gap + 24);
            let rail_w = ((inner_w as usize * 34) / 100)
                .clamp(30, 36)
                .min(max_rail as usize) as u16;
            let stage_x = outer_x + rail_w + gap;
            let stage_w = inner_w.saturating_sub(rail_w + gap);
            stage = Area::new((stage_x, middle_y).into(), (stage_w, middle_h).into());

            let count_u16 = EXPERIENCE_COUNT as u16;
            if middle_h >= 19 {
                // Standard 23-26 row frame: header (1) + subtle rule (1) +
                // 8 cards * 2 rows (16) + footer (1) = 19 rows; all 8 visible at once!
                rail_header = Some(Area::new((outer_x, middle_y).into(), (rail_w, 1).into()));
                let cards_y = middle_y + 2;
                for (i, slot) in cards.iter_mut().enumerate() {
                    let y = cards_y + (i as u16) * 2;
                    *slot = Area::new((outer_x, y).into(), (rail_w, 2).into());
                }
                let footer_y = middle_y + middle_h - 1;
                if footer_y >= cards_y + count_u16 * 2 {
                    rail_footer = Some(Area::new((outer_x, footer_y).into(), (rail_w, 1).into()));
                }
            } else if middle_h >= 17 {
                // Header (1) + 8 cards * 2 rows (16) = 17 rows.
                rail_header = Some(Area::new((outer_x, middle_y).into(), (rail_w, 1).into()));
                let cards_y = middle_y + 1;
                for (i, slot) in cards.iter_mut().enumerate() {
                    let y = cards_y + (i as u16) * 2;
                    *slot = Area::new((outer_x, y).into(), (rail_w, 2).into());
                }
            } else if middle_h > count_u16 {
                // Header (1) + 8 single-row cards so every experience stays clickable.
                rail_header = Some(Area::new((outer_x, middle_y).into(), (rail_w, 1).into()));
                let cards_y = middle_y + 1;
                for (i, slot) in cards.iter_mut().enumerate() {
                    let y = cards_y + i as u16;
                    *slot = Area::new((outer_x, y).into(), (rail_w, 1).into());
                }
            } else if middle_h >= 1 {
                // Sliding window around the selected item for ultra-short terminals.
                let visible = usize::from(middle_h).min(EXPERIENCE_COUNT);
                let sel = selected.index();
                let start = sel
                    .saturating_sub(visible / 2)
                    .min(EXPERIENCE_COUNT - visible);
                for offset in 0..visible {
                    let idx = start + offset;
                    cards[idx] = Area::new(
                        (outer_x, middle_y + offset as u16).into(),
                        (rail_w, 1).into(),
                    );
                }
            }
        } else {
            // Compact layout: top expanding tab strip + full-width live HalfBlock stage.
            let outer_x = if w >= 50 { 1u16 } else { 0u16 };
            let inner_w = w.saturating_sub(outer_x * 2);
            let strip_h: u16 = if middle_h >= 6 {
                2
            } else if middle_h >= 2 {
                1
            } else {
                0
            };
            if strip_h > 0 && inner_w >= EXPERIENCE_COUNT as u16 * 3 {
                // Give the active tab extra breathing room for its title while keeping all 8 clickable.
                let sel = selected.index();
                let active_w = if inner_w >= 54 {
                    15u16
                } else if inner_w >= 40 {
                    12u16
                } else {
                    inner_w / 3
                };
                let rest_count = (EXPERIENCE_COUNT - 1) as u16;
                let rem_w = inner_w.saturating_sub(active_w);
                let base_other = rem_w / rest_count;
                let extra_other = rem_w % rest_count;
                let mut cursor_x = outer_x;
                let mut other_i = 0u16;
                for (i, slot) in cards.iter_mut().enumerate() {
                    let tw = if i == sel {
                        active_w
                    } else {
                        let w = base_other + u16::from(other_i < extra_other);
                        other_i += 1;
                        w
                    };
                    *slot = Area::new((cursor_x, middle_y).into(), (tw, strip_h).into());
                    cursor_x = cursor_x.saturating_add(tw);
                }
            } else if strip_h > 0 && inner_w >= 8 {
                // Very narrow terminal: 4-slot sliding tab window centered on `selected`.
                let visible = 4usize;
                let sel = selected.index();
                let start = sel
                    .saturating_sub(visible / 2)
                    .min(EXPERIENCE_COUNT - visible);
                let base_w = inner_w / visible as u16;
                let extra = inner_w % visible as u16;
                let mut cursor_x = outer_x;
                for offset in 0..visible {
                    let idx = start + offset;
                    let tw = base_w + u16::from((offset as u16) < extra);
                    cards[idx] = Area::new((cursor_x, middle_y).into(), (tw, strip_h).into());
                    cursor_x = cursor_x.saturating_add(tw);
                }
            }
            let stage_y = middle_y + strip_h;
            let stage_h = middle_h.saturating_sub(strip_h);
            if stage_h >= 3 && inner_w >= 6 {
                stage = Area::new((outer_x, stage_y).into(), (inner_w, stage_h).into());
            }
        }

        Self {
            mode,
            header,
            header_divider_y,
            rail_header,
            rail_footer,
            dock,
            regions: HitRegions {
                size,
                cards,
                stage,
                tour_pill,
            },
        }
    }
}

/// Reusable render buffers & canvases so 30 FPS drawing stays allocation-free.
pub struct LauncherRenderer {
    stage_canvas: Canvas,
    emblem_canvas: Canvas,
    emblem_pixels: Vec<[u8; 3]>,
    prev_pixels: Vec<[u8; 3]>,
    next_pixels: Vec<[u8; 3]>,
    blended_pixels: Vec<[u8; 3]>,
}

impl Default for LauncherRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherRenderer {
    pub fn new() -> Self {
        Self {
            stage_canvas: Canvas::with_encoding(Encoding::HalfBlock),
            emblem_canvas: Canvas::with_encoding(Encoding::HalfBlock),
            emblem_pixels: Vec::new(),
            prev_pixels: Vec::new(),
            next_pixels: Vec::new(),
            blended_pixels: Vec::new(),
        }
    }

    pub fn draw(
        &mut self,
        state: &LauncherState,
        scenes: &mut SceneEngine,
        mut chunk: Chunk<'_>,
    ) -> Result<ComputedLayout, DrawErr> {
        let size = chunk.area().size();
        let layout = ComputedLayout::compute(size, state.selected);
        if size.width == 0 || size.height == 0 {
            return Ok(layout);
        }

        chunk.fill(0, 0, size.width, size.height, ' ', bg_style(BG_SHELL))?;

        if size.width < 10 || size.height < 4 {
            let _ = chunk.set_string(
                0,
                0,
                truncate_to_width("rsille", size.width as usize),
                fg_bg_bold(FG_PRIMARY, BG_SHELL),
            );
            return Ok(layout);
        }

        let (accent, secondary) = active_palette(state);

        self.draw_header(state, &layout, accent, secondary, &mut chunk)?;

        match layout.mode {
            LayoutMode::Wide => {
                self.draw_rail(state, &layout, accent, secondary, &mut chunk)?;
            }
            LayoutMode::Compact => {
                self.draw_compact_strip(state, &layout, accent, secondary, &mut chunk)?;
            }
        }

        self.draw_stage(state, scenes, &layout, accent, secondary, &mut chunk)?;
        self.draw_dock(state, &layout, accent, &mut chunk)?;

        Ok(layout)
    }

    fn draw_header(
        &mut self,
        state: &LauncherState,
        layout: &ComputedLayout,
        accent: [u8; 3],
        secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let w = layout.header.width();
        if w == 0 || layout.header.height() < 2 {
            return Ok(());
        }
        chunk.fill(0, 0, w, 2, ' ', bg_style(BG_HEADER))?;

        let margin_x: u16 = if w >= 50 { 1 } else { 0 };
        let emblem_w: u16 = if w >= 88 {
            8
        } else if w >= 64 {
            6
        } else if w >= 42 {
            4
        } else {
            0
        };

        if emblem_w > 0 {
            let pw = emblem_w as usize;
            let ph = 4usize;
            fx::render_emblem(
                pw,
                ph,
                state.time,
                accent,
                secondary,
                &mut self.emblem_pixels,
            );
            let emblem_area = local_area(chunk, margin_x, 0, emblem_w, 2);
            let emblem_chunk = chunk.from_area(emblem_area)?;
            blit_halfblock(
                &mut self.emblem_canvas,
                &self.emblem_pixels,
                pw,
                ph,
                BG_HEADER,
                emblem_chunk,
            )?;
        }

        let title_x = if emblem_w > 0 {
            margin_x + emblem_w + 2
        } else {
            margin_x
        };

        let breath = (state.time * 3.2).sin() as f32 * 0.5 + 0.5;
        let pill_rect = layout.regions.tour_pill;
        let pill_x = pill_rect.x();
        let pill_w = pill_rect.width();
        if pill_w >= 8 && pill_x > title_x + 4 {
            let (pill_bg, glyph_col, label_col, text_short, text_full) = if state.auto_tour {
                let bg = fx::lerp_rgb(BG_HEADER, accent, 0.22 + 0.08 * breath);
                let spark = fx::lerp_rgb(accent, [255, 255, 255], 0.35 + 0.35 * breath);
                let fg = fx::lerp_rgb(FG_PRIMARY, accent, 0.22);
                (bg, spark, fg, "✦ TOUR", "✦ AUTO-TOUR")
            } else {
                let bg = fx::lerp_rgb(BG_HEADER, secondary, 0.18);
                let spark = fx::lerp_rgb(accent, secondary, 0.4);
                (bg, spark, FG_SECONDARY, "● LIVE", "● INTERACTIVE")
            };
            chunk.fill(pill_x, 0, pill_w, 1, ' ', bg_style(pill_bg))?;
            let label = if pill_w >= 15 { text_full } else { text_short };
            let label_w = display_width(label) as u16;
            let pad = pill_w.saturating_sub(label_w) / 2;
            let mut cx = pill_x + pad;
            let mut chars = label.chars();
            if let Some(first) = chars.next() {
                let _ = chunk.set_char(cx, 0, first, fg_bg_bold(glyph_col, pill_bg));
                cx += 1;
            }
            let rest: String = chars.collect();
            let _ = chunk.set_string(cx, 0, &rest, fg_bg_bold(label_col, pill_bg));
        }

        let title_budget = pill_x.saturating_sub(title_x + 1) as usize;
        let title = if title_budget >= 17 {
            "RSILLE PLAYGROUND"
        } else if title_budget >= 6 {
            "RSILLE"
        } else {
            ""
        };
        let char_count = title.chars().count().max(1);
        for (i, ch) in title.chars().enumerate() {
            let pos = i as f32 / char_count as f32;
            let base = fx::aurora_color(pos, state.time * 0.9, accent, secondary);
            let bright = fx::lerp_rgb(base, FG_PRIMARY, 0.32);
            let _ = chunk.set_char(title_x + i as u16, 0, ch, fg_bg_bold(bright, BG_HEADER));
        }

        let target = state.current_launch_target();
        let cmd_text = target.display_command;
        let max_cmd_w = if w >= 76 {
            (w as usize)
                .saturating_sub(title_x as usize + 22)
                .clamp(16, 48)
        } else {
            (w as usize).saturating_sub(title_x as usize + margin_x as usize + 2)
        };

        let mut cmd_start_x = w.saturating_sub(margin_x);
        if max_cmd_w >= 12 {
            let inner_budget = max_cmd_w.saturating_sub(4);
            let shown_cmd = truncate_with_ellipsis(&cmd_text, inner_budget);
            let shown_w = display_width(&shown_cmd);
            let box_w = (shown_w + 4) as u16;
            if layout.mode == LayoutMode::Wide {
                cmd_start_x = w.saturating_sub(margin_x + box_w);
            } else {
                cmd_start_x = title_x;
            }
            if cmd_start_x + box_w <= w {
                let cmd_bg = fx::lerp_rgb(BG_CMD_PILL, accent, 0.08);
                chunk.fill(cmd_start_x, 1, box_w, 1, ' ', bg_style(cmd_bg))?;
                let _ = chunk.set_char(cmd_start_x + 1, 1, '$', fg_bg_bold(accent, cmd_bg));
                let _ = chunk.set_string(cmd_start_x + 3, 1, &shown_cmd, fg_bg(FG_PRIMARY, cmd_bg));
            }
        }

        if layout.mode == LayoutMode::Wide {
            let tag_budget = cmd_start_x.saturating_sub(title_x + 2) as usize;
            if tag_budget >= 10 {
                let dot_col = fx::lerp_rgb(accent, secondary, 0.4);
                let _ = chunk.set_char(title_x, 1, '●', fg_bg(dot_col, BG_HEADER));
                let subtitle = match state.caps.raster {
                    RasterSupport::Kitty | RasterSupport::Both => {
                        "Terminal UI, 3D & Overlay · Kitty ready"
                    }
                    RasterSupport::Sixel => "Terminal UI, 3D & Overlay · Sixel ready",
                    RasterSupport::None => "Terminal UI, 3D & Overlay · Unicode mode",
                    RasterSupport::Unknown => "Interactive Terminal Graphics Studio",
                };
                let shown = truncate_with_ellipsis(subtitle, tag_budget.saturating_sub(2));
                let _ = chunk.set_string(title_x + 2, 1, &shown, fg_bg(FG_MUTED, BG_HEADER));
            }
        }

        if let Some(div_y) = layout.header_divider_y {
            for x in 0..w {
                let pos = x as f32 / w.max(1) as f32;
                let glow = fx::aurora_color(pos, state.time * 0.7, accent, secondary);
                let edge_fade = 1.0 - ((pos - 0.5).abs() * 1.4).clamp(0.0, 0.72);
                let col = fx::lerp_rgb(BG_SHELL, glow, 0.55 * edge_fade);
                let _ = chunk.set_char(x, div_y, '─', fg_bg(col, BG_SHELL));
            }
        }

        Ok(())
    }

    fn draw_rail(
        &self,
        state: &LauncherState,
        layout: &ComputedLayout,
        accent: [u8; 3],
        secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        if let Some(hdr) = layout.rail_header {
            let x = hdr.x();
            let y = hdr.y();
            let w = hdr.width();
            let _ = chunk.set_string(x + 1, y, "EXPERIENCES", fg_bg_bold(FG_DIM, BG_SHELL));
            let counter = format!("{}/{}", state.selected.index() + 1, EXPERIENCE_COUNT);
            let cw = counter.len() as u16;
            if w > cw + 14 {
                let _ = chunk.set_string(x + w - cw - 1, y, &counter, fg_bg(accent, BG_SHELL));
            }
            if layout.regions.cards[0].y() >= y + 2 && w > 2 {
                let span = (w - 2) as f32;
                for dx in 0..(w - 2) {
                    let frac = dx as f32 / span.max(1.0);
                    let fg = fx::gradient_color(frac, BORDER_DIM, BG_SHELL);
                    let _ = chunk.set_char(
                        x + 1 + dx,
                        y + 1,
                        '─',
                        Style::with_colors(Colors::new(fg, rgb(BG_SHELL))),
                    );
                }
            }
        }

        let breath = (state.time * 3.0).sin() as f32 * 0.5 + 0.5;
        let pulse = state.active_pulse_strength();
        let raster_missing = state.caps.raster == RasterSupport::None;

        for exp in ExperienceId::ALL {
            let card = layout.regions.cards[exp.index()];
            if card.width() < 8 || card.height() == 0 {
                continue;
            }
            let spec = ExperienceSpec::for_id(exp);
            let is_selected = exp == state.selected;
            let is_hovered = state.hovered_card == Some(exp);
            let exp_accent = exp.accent_rgb();
            let exp_secondary = exp.secondary_rgb();

            let card_bg = if is_selected {
                fx::lerp_rgb(
                    [15, 21, 35],
                    exp_accent,
                    0.15 + 0.04 * breath + 0.08 * pulse,
                )
            } else if is_hovered {
                BG_CARD_HOVER
            } else {
                BG_CARD
            };

            chunk.fill(
                card.x(),
                card.y(),
                card.width(),
                card.height(),
                ' ',
                bg_style(card_bg),
            )?;

            for dy in 0..card.height() {
                let frac = if card.height() <= 1 {
                    0.0
                } else {
                    dy as f32 / (card.height() - 1) as f32
                };
                if is_selected {
                    let bar_base = fx::lerp_rgb(exp_accent, exp_secondary, frac);
                    let bar_col =
                        fx::lerp_rgb(bar_base, [255, 255, 255], (0.14 * breath) + (0.28 * pulse));
                    let _ = chunk.set_char(card.x(), card.y() + dy, '▌', fg_bg(bar_col, card_bg));
                } else if is_hovered {
                    let bar_col = fx::lerp_rgb(BORDER_DIM, exp_accent, 0.55);
                    let _ = chunk.set_char(card.x(), card.y() + dy, '▎', fg_bg(bar_col, card_bg));
                } else {
                    let _ =
                        chunk.set_char(card.x(), card.y() + dy, '│', fg_bg(BORDER_DIM, card_bg));
                }
            }

            // Row 0: index + icon + title + badge/chevron.
            let num_ch = char::from(b'1' + exp.index() as u8);
            let num_col = if is_selected {
                exp_accent
            } else if is_hovered {
                FG_SECONDARY
            } else {
                FG_DIM
            };
            let icon_col = if is_selected {
                fx::lerp_rgb(exp_accent, [255, 255, 255], 0.2)
            } else {
                fx::lerp_rgb(card_bg, exp_accent, 0.68)
            };
            let title_col = if is_selected {
                FG_PRIMARY
            } else if is_hovered {
                [224, 232, 246]
            } else if spec.requires_raster && raster_missing {
                FG_MUTED
            } else {
                [194, 206, 226]
            };

            let row0 = card.y();
            let _ = chunk.set_char(card.x() + 2, row0, num_ch, fg_bg_bold(num_col, card_bg));
            let _ = chunk.set_string(card.x() + 4, row0, spec.icon, fg_bg_bold(icon_col, card_bg));

            // Right-side badge on inactive cards (or chevron on active card)
            let right_reserve: usize = if is_selected {
                3
            } else if card.width() >= 28 {
                display_width(spec.badge) + 2
            } else {
                2
            };
            let title_budget = (card.width() as usize).saturating_sub(6 + right_reserve);
            let shown_title = truncate_with_ellipsis(spec.title, title_budget);
            let title_style = if is_selected {
                fg_bg_bold(title_col, card_bg)
            } else {
                fg_bg(title_col, card_bg)
            };
            let _ = chunk.set_string(card.x() + 6, row0, &shown_title, title_style);

            if is_selected && card.width() >= 10 {
                let arrow_col = fx::lerp_rgb(exp_accent, [255, 255, 255], 0.25 * breath);
                let _ = chunk.set_char(
                    card.x() + card.width() - 2,
                    row0,
                    '▸',
                    fg_bg_bold(arrow_col, card_bg),
                );
            } else if card.width() >= 28 {
                let bw = display_width(spec.badge) as u16;
                let bx = card.x() + card.width().saturating_sub(bw + 1);
                let badge_fg = if spec.requires_raster && raster_missing {
                    [198, 138, 92]
                } else {
                    FG_DIM
                };
                let _ = chunk.set_string(bx, row0, spec.badge, fg_bg(badge_fg, card_bg));
            } else if is_hovered && card.width() >= 10 {
                let _ = chunk.set_char(
                    card.x() + card.width() - 2,
                    row0,
                    '›',
                    fg_bg(FG_MUTED, card_bg),
                );
            }

            // Row 1: active variant selector pill OR concise subtitle.
            if card.height() >= 2 {
                let row1 = card.y() + 1;
                let budget = card.width().saturating_sub(6) as usize;
                if is_selected {
                    let variant = state.interaction(exp).variant;
                    let vlabel = SceneEngine::variant_label(exp, variant);
                    let chip_bg = fx::lerp_rgb(BG_SHELL, exp_accent, 0.24);
                    let chip_fg = fx::lerp_rgb(FG_PRIMARY, exp_accent, 0.18);
                    let arrow_fg = fx::lerp_rgb(exp_accent, exp_secondary, 0.35);
                    let count = exp.variant_count().max(1);
                    let text = if count > 1 {
                        format!(
                            " ◂ {} ▸ ",
                            truncate_with_ellipsis(vlabel, budget.saturating_sub(6))
                        )
                    } else {
                        format!(
                            " {} ",
                            truncate_with_ellipsis(vlabel, budget.saturating_sub(2))
                        )
                    };
                    let tw = display_width(&text).min(budget) as u16;
                    if tw > 0 {
                        chunk.fill(card.x() + 4, row1, tw, 1, ' ', bg_style(chip_bg))?;
                        let _ =
                            chunk.set_string(card.x() + 4, row1, &text, fg_bg(chip_fg, chip_bg));
                        if count > 1 && tw >= 5 {
                            let _ =
                                chunk.set_char(card.x() + 5, row1, '◂', fg_bg(arrow_fg, chip_bg));
                            let _ = chunk.set_char(
                                card.x() + 4 + tw - 2,
                                row1,
                                '▸',
                                fg_bg(arrow_fg, chip_bg),
                            );
                        }
                    }
                } else {
                    let sub_text = if spec.requires_raster && raster_missing {
                        "Needs Kitty or Sixel terminal"
                    } else {
                        spec.subtitle
                    };
                    let shown_sub = truncate_with_ellipsis(sub_text, budget);
                    let sub_col = if spec.requires_raster && raster_missing {
                        [150, 122, 98]
                    } else {
                        FG_MUTED
                    };
                    let _ =
                        chunk.set_string(card.x() + 4, row1, &shown_sub, fg_bg(sub_col, card_bg));
                }
            }
        }

        if let Some(ftr) = layout.rail_footer {
            let x = ftr.x();
            let y = ftr.y();
            let w = ftr.width() as usize;
            let key_bg = fx::lerp_rgb(BG_KEYCAP, accent, 0.25);
            let _ = chunk.set_string(x + 1, y, " ↵ Enter ", fg_bg_bold(FG_PRIMARY, key_bg));
            let rest_x = x + 11;
            let rest_budget = w.saturating_sub(12);
            if rest_budget >= 6 {
                let spec = ExperienceSpec::for_id(state.selected);
                let label = format!("Launch {}", spec.title);
                let shown = truncate_with_ellipsis(&label, rest_budget);
                let tint = fx::lerp_rgb(FG_SECONDARY, secondary, 0.35);
                let _ = chunk.set_string(rest_x, y, &shown, fg_bg(tint, BG_SHELL));
            }
        }

        Ok(())
    }

    fn draw_compact_strip(
        &self,
        state: &LauncherState,
        layout: &ComputedLayout,
        _accent: [u8; 3],
        _secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let breath = (state.time * 3.0).sin() as f32 * 0.5 + 0.5;
        for exp in ExperienceId::ALL {
            let tab = layout.regions.cards[exp.index()];
            if tab.width() < 2 || tab.height() == 0 {
                continue;
            }
            let spec = ExperienceSpec::for_id(exp);
            let is_selected = exp == state.selected;
            let is_hovered = state.hovered_card == Some(exp);
            let exp_accent = exp.accent_rgb();
            let exp_secondary = exp.secondary_rgb();

            let tab_bg = if is_selected {
                fx::lerp_rgb([15, 21, 35], exp_accent, 0.20 + 0.05 * breath)
            } else if is_hovered {
                BG_CARD_HOVER
            } else {
                BG_CARD
            };

            chunk.fill(tab.x(), tab.y(), tab.width(), 1, ' ', bg_style(tab_bg))?;

            let budget = tab.width().saturating_sub(1) as usize;
            let full_label = format!("{} {} {}", exp.index() + 1, spec.icon, spec.compact_title);
            let icon_label = format!("{} {}", exp.index() + 1, spec.icon);
            let label = if display_width(&full_label) <= budget {
                full_label
            } else if display_width(&icon_label) <= budget {
                icon_label
            } else {
                format!("{}", exp.index() + 1)
            };

            let lw = display_width(&label) as u16;
            let pad = tab.width().saturating_sub(lw) / 2;
            let style = if is_selected {
                fg_bg_bold(FG_PRIMARY, tab_bg)
            } else if is_hovered {
                fg_bg(FG_SECONDARY, tab_bg)
            } else {
                fg_bg(FG_MUTED, tab_bg)
            };
            let _ = chunk.set_string(tab.x() + pad, tab.y(), &label, style);

            if tab.height() >= 2 {
                let uy = tab.y() + 1;
                for dx in 0..tab.width() {
                    if is_selected {
                        let t = if tab.width() <= 1 {
                            0.0
                        } else {
                            dx as f32 / (tab.width() - 1) as f32
                        };
                        let col = fx::lerp_rgb(exp_accent, exp_secondary, t);
                        let _ = chunk.set_char(tab.x() + dx, uy, '━', fg_bg(col, BG_SHELL));
                    } else {
                        let col = if is_hovered { FG_DIM } else { BORDER_DIM };
                        let _ = chunk.set_char(tab.x() + dx, uy, '─', fg_bg(col, BG_SHELL));
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_stage(
        &mut self,
        state: &LauncherState,
        scenes: &mut SceneEngine,
        layout: &ComputedLayout,
        accent: [u8; 3],
        secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let stage = layout.regions.stage;
        if stage.width() < 6 || stage.height() < 3 {
            return Ok(());
        }

        let sx = stage.x();
        let sy = stage.y();
        let sw = stage.width();
        let sh = stage.height();
        let inner_w = sw - 2;
        let inner_h = sh - 2;

        let pulse = state.active_pulse_strength();
        let hover_boost = if state.hovered_stage { 0.14 } else { 0.0 };
        let border_mix = (0.52 + 0.30 * pulse + hover_boost).clamp(0.0, 1.0);

        let border_color_at = |u: f32| -> [u8; 3] {
            let base = fx::lerp_rgb(accent, secondary, u);
            let tinted = fx::lerp_rgb(BORDER_DIM, base, border_mix);
            fx::lerp_rgb(tinted, [255, 255, 255], 0.16 * pulse)
        };

        let _ = chunk.set_char(sx, sy, '╭', fg_bg(border_color_at(0.0), BG_SHELL));
        let _ = chunk.set_char(sx + sw - 1, sy, '╮', fg_bg(border_color_at(1.0), BG_SHELL));
        let _ = chunk.set_char(sx, sy + sh - 1, '╰', fg_bg(border_color_at(0.25), BG_SHELL));
        let _ = chunk.set_char(
            sx + sw - 1,
            sy + sh - 1,
            '╯',
            fg_bg(border_color_at(0.75), BG_SHELL),
        );

        for dx in 1..sw - 1 {
            let u = dx as f32 / (sw - 1) as f32;
            let _ = chunk.set_char(sx + dx, sy, '─', fg_bg(border_color_at(u), BG_SHELL));
        }
        for dy in 1..sh - 1 {
            let v = dy as f32 / (sh - 1) as f32;
            let left_col = border_color_at(v * 0.35);
            let right_col = border_color_at(1.0 - v * 0.35);
            let _ = chunk.set_char(sx, sy + dy, '│', fg_bg(left_col, BG_SHELL));
            let _ = chunk.set_char(sx + sw - 1, sy + dy, '│', fg_bg(right_col, BG_SHELL));
        }

        self.draw_stage_top_bar(state, stage, accent, secondary, chunk)?;
        self.draw_stage_bottom_bar(state, stage, accent, secondary, chunk)?;

        let pw = inner_w as usize;
        let ph = inner_h as usize * 2;
        let pixels = if let Some(tr) = state.transition {
            let from_int = state.interaction(tr.from);
            let to_int = state.interaction(tr.to);
            scenes.render_into(tr.from, pw, ph, state.time, from_int, &mut self.prev_pixels);
            scenes.render_into(tr.to, pw, ph, state.time, to_int, &mut self.next_pixels);
            fx::blend_transition(
                &self.prev_pixels,
                &self.next_pixels,
                pw,
                ph,
                tr.progress,
                tr.to.accent_rgb(),
                &mut self.blended_pixels,
            );
            &self.blended_pixels
        } else {
            scenes.render_into(
                state.selected,
                pw,
                ph,
                state.time,
                state.active_interaction(),
                &mut self.next_pixels,
            );
            &self.next_pixels
        };

        let inner_area = local_area(chunk, sx + 1, sy + 1, inner_w, inner_h);
        let inner_chunk = chunk.from_area(inner_area)?;
        blit_halfblock(
            &mut self.stage_canvas,
            pixels,
            pw,
            ph,
            BG_STAGE_CANVAS,
            inner_chunk,
        )?;

        if inner_h >= 6 && inner_w >= 20 {
            self.draw_stage_hud_pills(state, sx + 1, sy + inner_h, inner_w, accent, chunk)?;
        }

        Ok(())
    }

    fn draw_stage_top_bar(
        &self,
        state: &LauncherState,
        stage: Area,
        accent: [u8; 3],
        secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let sx = stage.x();
        let sy = stage.y();
        let sw = stage.width();
        let inner_w = sw.saturating_sub(4) as usize;
        if inner_w < 8 {
            return Ok(());
        }

        let spec = ExperienceSpec::for_id(state.selected);
        let variant = state.active_interaction().variant;
        let vlabel = SceneEngine::variant_label(state.selected, variant);
        let vcount = state.selected.variant_count().max(1);

        let dots_w: usize = if vcount > 1 && inner_w >= 28 {
            vcount * 2 + 1
        } else {
            0
        };
        if dots_w > 0 {
            let dots_x = sx + sw - 2 - dots_w as u16;
            let _ = chunk.set_char(dots_x, sy, ' ', bg_style(BG_SHELL));
            for i in 0..vcount {
                let cx = dots_x + 1 + (i as u16) * 2;
                let active_dot = i == variant % vcount;
                let (ch, col) = if active_dot {
                    ('●', fx::lerp_rgb(accent, [255, 255, 255], 0.25))
                } else {
                    ('○', FG_DIM)
                };
                let _ = chunk.set_char(cx, sy, ch, fg_bg(col, BG_SHELL));
                let _ = chunk.set_char(cx + 1, sy, ' ', bg_style(BG_SHELL));
            }
        }

        let left_budget = inner_w.saturating_sub(dots_w + 1);
        let mut cx = sx + 2;
        let title_piece = format!(" {} {} ", spec.icon, spec.title.to_uppercase());
        let title_w = display_width(&title_piece);
        if title_w <= left_budget {
            chunk.fill(cx, sy, title_w as u16, 1, ' ', bg_style(BG_SHELL))?;
            let _ = chunk.set_string(cx + 1, sy, spec.icon, fg_bg_bold(accent, BG_SHELL));
            let upper = spec.title.to_uppercase();
            let _ = chunk.set_string(cx + 3, sy, &upper, fg_bg_bold(FG_PRIMARY, BG_SHELL));
            cx += title_w as u16;

            let remaining = left_budget.saturating_sub(title_w);
            if remaining >= 6 && !vlabel.is_empty() {
                let sep = "· ";
                let shown_v = truncate_with_ellipsis(vlabel, remaining.saturating_sub(3));
                let vcol = fx::lerp_rgb(accent, secondary, 0.45);
                let _ = chunk.set_string(cx, sy, sep, fg_bg(FG_DIM, BG_SHELL));
                let _ = chunk.set_string(cx + 2, sy, &shown_v, fg_bg(vcol, BG_SHELL));
                let end_x = cx + 2 + display_width(&shown_v) as u16;
                if end_x < sx + sw - 2 {
                    let _ = chunk.set_char(end_x, sy, ' ', bg_style(BG_SHELL));
                }
            }
        } else {
            let shown = truncate_with_ellipsis(&format!(" {} ", spec.title), left_budget);
            let _ = chunk.set_string(cx, sy, &shown, fg_bg_bold(FG_PRIMARY, BG_SHELL));
        }

        Ok(())
    }

    fn draw_stage_bottom_bar(
        &self,
        state: &LauncherState,
        stage: Area,
        accent: [u8; 3],
        secondary: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let sx = stage.x();
        let by = stage.y() + stage.height() - 1;
        let sw = stage.width();
        let inner_w = sw.saturating_sub(2) as usize;
        if inner_w == 0 {
            return Ok(());
        }

        let badge = if state.auto_tour {
            let rem = (super::state::AUTO_TOUR_INTERVAL_SECS - state.tour_elapsed)
                .clamp(0.0, super::state::AUTO_TOUR_INTERVAL_SECS);
            if inner_w >= 26 {
                Some((format!(" ✦ NEXT {:.1}s ", rem), accent))
            } else {
                None
            }
        } else if inner_w >= 28 {
            Some((" [a] AUTO-TOUR ".to_owned(), FG_MUTED))
        } else {
            None
        };

        let badge_w = badge.as_ref().map_or(0, |(s, _)| display_width(s));
        let bar_w = if badge_w > 0 && inner_w > badge_w + 6 {
            inner_w - badge_w - 1
        } else {
            inner_w
        };

        if state.auto_tour {
            let progress = state.tour_progress();
            let filled = ((progress * bar_w as f32).round() as usize).min(bar_w);
            for i in 0..inner_w {
                let x = sx + 1 + i as u16;
                if i < filled {
                    let pos = i as f32 / bar_w.max(1) as f32;
                    let col = if i + 1 == filled && filled < bar_w {
                        fx::lerp_rgb(accent, [255, 255, 255], 0.55)
                    } else {
                        fx::aurora_color(pos, state.time * 1.2, accent, secondary)
                    };
                    let ch = if i + 1 == filled && filled < bar_w {
                        '╸'
                    } else {
                        '━'
                    };
                    let _ = chunk.set_char(x, by, ch, fg_bg(col, BG_SHELL));
                } else {
                    let _ = chunk.set_char(x, by, '─', fg_bg(BORDER_DIM, BG_SHELL));
                }
            }
        } else {
            let pulse = state.active_pulse_strength();
            for i in 0..inner_w {
                let u = i as f32 / inner_w.max(1) as f32;
                let base = fx::lerp_rgb(accent, secondary, u);
                let col = fx::lerp_rgb(BORDER_DIM, base, 0.36 + 0.30 * pulse);
                let _ = chunk.set_char(sx + 1 + i as u16, by, '─', fg_bg(col, BG_SHELL));
            }
        }

        if let Some((text, col)) = badge {
            if inner_w > badge_w + 6 {
                let bx = sx + sw - 2 - badge_w as u16;
                let _ = chunk.set_string(bx, by, &text, fg_bg(col, BG_SHELL));
            }
        }

        Ok(())
    }

    fn draw_stage_hud_pills(
        &self,
        state: &LauncherState,
        inner_x: u16,
        row_y: u16,
        inner_w: u16,
        accent: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let interaction = state.active_interaction();
        let caption = SceneEngine::overlay_caption(state.selected, state.time, interaction);
        let hint = SceneEngine::interaction_hint(state.selected);

        let pill_bg = fx::lerp_rgb(BG_HUD_PILL, accent, 0.12);
        let mut left_end_x = inner_x + 1;

        if let Some(cap) = caption {
            if !cap.is_empty() {
                let (glyph, body) = if let Some(rest) = cap.strip_prefix("✦ ") {
                    ('✦', rest)
                } else if let Some(rest) = cap.strip_prefix("~ ") {
                    ('~', rest)
                } else {
                    ('✦', cap)
                };
                let max_cap = (inner_w as usize).saturating_sub(6);
                let shown = truncate_with_ellipsis(body, max_cap);
                let pill_w = (display_width(&shown) + 4) as u16;
                let px = inner_x + 1;
                if px + pill_w < inner_x + inner_w {
                    chunk.fill(px, row_y, pill_w, 1, ' ', bg_style(pill_bg))?;
                    let _ = chunk.set_char(px + 1, row_y, glyph, fg_bg_bold(accent, pill_bg));
                    let _ = chunk.set_string(px + 3, row_y, &shown, fg_bg(FG_PRIMARY, pill_bg));
                    left_end_x = px + pill_w;
                }
            }
        }

        let hint_body = hint
            .strip_prefix("Space: ")
            .or_else(|| hint.strip_prefix("Space "))
            .unwrap_or(hint);
        if !hint_body.is_empty() {
            let key_tag = "Space";
            let full_w = (display_width(key_tag) + display_width(hint_body) + 3) as u16;
            let hx = inner_x + inner_w - 1 - full_w;
            if hx > left_end_x + 1 && hx > inner_x {
                let hint_bg = BG_HUD_PILL;
                chunk.fill(hx, row_y, full_w, 1, ' ', bg_style(hint_bg))?;
                let _ = chunk.set_string(hx + 1, row_y, key_tag, fg_bg_bold(accent, hint_bg));
                let _ = chunk.set_string(
                    hx + 1 + display_width(key_tag) as u16 + 1,
                    row_y,
                    hint_body,
                    fg_bg(FG_SECONDARY, hint_bg),
                );
            }
        }

        Ok(())
    }

    fn draw_dock(
        &self,
        state: &LauncherState,
        layout: &ComputedLayout,
        accent: [u8; 3],
        chunk: &mut Chunk<'_>,
    ) -> Result<(), DrawErr> {
        let dock = layout.dock;
        if dock.width() < 8 || dock.height() == 0 {
            return Ok(());
        }
        let y = dock.y();
        let w = dock.width();
        chunk.fill(0, y, w, 1, ' ', bg_style(BG_DOCK))?;

        let tour_label = if state.auto_tour {
            "Pause Tour"
        } else {
            "Auto-Tour"
        };
        let items: &[(&str, &str)] = if w >= 98 {
            &[
                ("↑↓", "Move"),
                ("Enter", "Launch"),
                ("Space", "Interact"),
                ("←→", "Variant"),
                ("a", tour_label),
                ("1-8", "Jump"),
                ("q", "Quit"),
            ]
        } else if w >= 72 {
            &[
                ("↑↓", "Move"),
                ("↵", "Launch"),
                ("Space", "Interact"),
                ("←→", "Variant"),
                ("a", "Tour"),
                ("q", "Quit"),
            ]
        } else {
            &[
                ("↑↓", "Switch"),
                ("↵", "Launch"),
                ("Space", "Act"),
                ("a", "Tour"),
                ("q", "Quit"),
            ]
        };

        let margin_x: u16 = if w >= 50 { 1 } else { 0 };
        let limit_x = w.saturating_sub(margin_x);
        let mut cx = margin_x;

        for (idx, &(key, label)) in items.iter().enumerate() {
            let key_w = display_width(key) as u16 + 2;
            let label_w = display_width(label) as u16;
            let sep_w: u16 = if idx == 0 { 0 } else { 2 };
            let total = sep_w + key_w + 1 + label_w;
            if cx + total > limit_x {
                break;
            }
            if idx > 0 {
                let _ = chunk.set_char(cx, y, '·', fg_bg(FG_DIM, BG_DOCK));
                cx += 2;
            }
            let cap_bg = if key == "Enter" || key == "↵" {
                fx::lerp_rgb(BG_KEYCAP, accent, 0.30)
            } else {
                BG_KEYCAP
            };
            let cap_fg = if key == "Enter" || key == "↵" {
                FG_PRIMARY
            } else {
                [218, 228, 246]
            };
            chunk.fill(cx, y, key_w, 1, ' ', bg_style(cap_bg))?;
            let _ = chunk.set_string(cx + 1, y, key, fg_bg_bold(cap_fg, cap_bg));
            cx += key_w + 1;
            let _ = chunk.set_string(cx, y, label, fg_bg(FG_MUTED, BG_DOCK));
            cx += label_w + 1;
        }

        Ok(())
    }
}

fn active_palette(state: &LauncherState) -> ([u8; 3], [u8; 3]) {
    if let Some(tr) = state.transition {
        let ease = 1.0 - (1.0 - tr.progress.clamp(0.0, 1.0)).powi(3);
        (
            fx::lerp_rgb(tr.from.accent_rgb(), tr.to.accent_rgb(), ease),
            fx::lerp_rgb(tr.from.secondary_rgb(), tr.to.secondary_rgb(), ease),
        )
    } else {
        (state.selected.accent_rgb(), state.selected.secondary_rgb())
    }
}

fn blit_halfblock(
    canvas: &mut Canvas,
    pixels: &[[u8; 3]],
    pw: usize,
    ph: usize,
    fallback_bg: [u8; 3],
    chunk: Chunk<'_>,
) -> Result<(), DrawErr> {
    let size = chunk.area().size();
    if size.width == 0 || size.height == 0 || pw == 0 || ph == 0 {
        return Ok(());
    }
    canvas.clear();
    canvas.set_background(rgb(fallback_bg));
    canvas.set_bound((0, size.width as i32 - 1), (0, size.height as i32 - 1));
    let total_h = size.height as usize * 2;
    let sample_w = pw.min(size.width as usize);
    let sample_h = ph.min(total_h);
    for y in 0..sample_h {
        let row_offset = y * pw;
        let canvas_y = (total_h - 1 - y) as f64;
        for x in 0..sample_w {
            if let Some(&pixel) = pixels.get(row_offset + x) {
                canvas.set_color(x as f64, canvas_y, rgb(pixel));
            }
        }
    }
    canvas.draw(chunk)?;
    Ok(())
}

fn local_area(chunk: &Chunk<'_>, x: u16, y: u16, width: u16, height: u16) -> Area {
    let origin = chunk.area().pos();
    Area::new((origin.x + x, origin.y + y).into(), (width, height).into())
}

fn rgb([r, g, b]: [u8; 3]) -> Color {
    Color::Rgb { r, g, b }
}

fn bg_style(bg: [u8; 3]) -> Style {
    Style::with_colors(Colors::new(rgb(FG_PRIMARY), rgb(bg)))
}

fn fg_bg(fg: [u8; 3], bg: [u8; 3]) -> Style {
    Style::with_colors(Colors::new(rgb(fg), rgb(bg)))
}

fn fg_bg_bold(fg: [u8; 3], bg: [u8; 3]) -> Style {
    Style::with_both(
        Colors::new(rgb(fg), rgb(bg)),
        Attributes::from(Attribute::Bold),
    )
}

fn display_width(text: &str) -> usize {
    TextMetrics::default().width(text)
}

fn truncate_to_width(text: &str, max_cols: usize) -> &str {
    TextMetrics::default().truncate(text, max_cols)
}

fn truncate_with_ellipsis(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let metrics = TextMetrics::default();
    if metrics.width(text) <= max_cols {
        return text.to_owned();
    }
    if max_cols == 1 {
        return "…".to_owned();
    }
    let prefix = metrics.truncate(text, max_cols - 1);
    format!("{prefix}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsille::render::buffer::Buffer;

    fn render_buffer(state: &LauncherState, size: Size) -> (Buffer, ComputedLayout) {
        let mut buffer = Buffer::new(size);
        let mut renderer = LauncherRenderer::new();
        let mut scenes = SceneEngine::new();
        let layout = {
            let chunk = Chunk::new(&mut buffer, size.into()).unwrap();
            renderer.draw(state, &mut scenes, chunk).unwrap()
        };
        (buffer, layout)
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let size = buffer.size();
        let mut out = String::new();
        for y in 0..size.height {
            for x in 0..size.width {
                if let Some(cell) = buffer.get((x, y).into()) {
                    out.push_str(&cell.symbol());
                }
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn wide_layout_renders_header_all_eight_cards_stage_and_dock_cleanly() {
        let mut state = LauncherState::new();
        state.set_time(1.25);
        let (buffer, layout) = render_buffer(&state, (112, 24).into());
        assert_eq!(layout.mode, LayoutMode::Wide);
        assert!(layout.regions.stage.width() >= 55);
        assert!(layout.regions.stage.height() >= 18);
        for card in layout.regions.cards {
            assert_eq!(card.height(), 2);
            assert!(card.width() >= 30);
            assert!(!card.intersects(&layout.regions.stage));
        }
        let text = buffer_text(&buffer);
        assert!(text.contains("RSILLE PLAYGROUND"));
        assert!(text.contains("AUTO-TOUR"));
        assert!(text.contains("Voxel Sandbox"));
        assert!(text.contains("Clockwork"));
        assert!(text.contains("Lumen Desk"));
        assert!(text.contains("Shell HUD"));
        assert!(text.contains("DeepSeek Pet"));
        assert!(text.contains("Function"));
        assert!(text.contains("Jellyfish"));
        assert!(text.contains("Terminal"));
        assert!(text.contains("rsille voxel"));
        let expected_top = format!(
            "▣ VOXEL SANDBOX · {}",
            SceneEngine::variant_label(ExperienceId::Voxel, 0)
        );
        assert!(text.contains(&expected_top));
        assert!(text.contains('╭') && text.contains('╯'));
    }

    #[test]
    fn compact_layout_stacks_strip_above_full_width_stage() {
        let mut state = LauncherState::new();
        state.select_immediate(ExperienceId::Desk);
        state.set_time(2.0);
        let (buffer, layout) = render_buffer(&state, (68, 18).into());
        assert_eq!(layout.mode, LayoutMode::Compact);
        assert!(layout.regions.stage.width() >= 60);
        assert!(layout.regions.stage.height() >= 11);
        for card in layout.regions.cards {
            assert_eq!(card.height(), 2);
            assert!(card.y() + card.height() <= layout.regions.stage.y());
        }
        let text = buffer_text(&buffer);
        assert!(text.contains("RSILLE PLAYGROUND"));
        assert!(text.contains("LUMEN DESK"));
    }

    #[test]
    fn handles_tiny_and_extreme_sizes_without_panicking() {
        let mut state = LauncherState::new();
        state.set_transition(ExperienceId::Voxel, ExperienceId::Doctor, 0.45);
        for (w, h) in [
            (0, 0),
            (1, 1),
            (8, 3),
            (20, 6),
            (40, 10),
            (60, 14),
            (75, 16),
            (76, 16),
            (80, 22),
            (140, 24),
        ] {
            let (buffer, _layout) = render_buffer(&state, (w, h).into());
            assert_eq!(buffer.size(), Size::from((w, h)));
        }
    }

    #[test]
    fn stage_hud_pills_never_duplicate_space_or_caption_glyphs() {
        for exp in ExperienceId::ALL {
            let mut state = LauncherState::new();
            state.select_immediate(exp);
            state.set_time(1.0);
            state.trigger_interaction();
            state.set_time(1.4);
            let (buffer, _) = render_buffer(&state, (112, 24).into());
            let text = buffer_text(&buffer);
            assert!(
                !text.contains("Space Space"),
                "duplicate Space prefix in HUD for {exp:?}"
            );
            assert!(
                !text.contains("✦ ✦") && !text.contains("✦ ~"),
                "duplicate caption prefix in HUD for {exp:?}"
            );
        }
        let mut pet = LauncherState::new();
        pet.select_immediate(ExperienceId::DeepseekPet);
        pet.set_time(1.2);
        let (buffer, _) = render_buffer(&pet, (112, 24).into());
        let text = buffer_text(&buffer);
        assert!(text.contains("~ patrolling your terminal"));
        assert!(!text.contains("✦ ~"));
        assert!(text.contains("Space splash hop"));
    }

    #[test]
    fn experience_icons_are_single_column_unicode() {
        let metrics = TextMetrics::default();
        for exp in ExperienceId::ALL {
            let spec = ExperienceSpec::for_id(exp);
            assert_eq!(
                metrics.width(spec.icon),
                1,
                "icon {} for {:?} must occupy exactly 1 cell",
                spec.icon,
                exp
            );
        }
    }
}
