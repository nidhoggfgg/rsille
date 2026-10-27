//! The visual language is shared by ordinary rsille widgets and canvas ink.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub bg: Color,
    pub card: Color,
    pub raised: Color,
    pub text: Color,
    pub muted: Color,
    pub line: Color,
    pub mint: Color,
    pub violet: Color,
    pub peach: Color,
}

pub(super) fn palette(index: usize) -> Palette {
    match index % 3 {
        1 => Palette {
            bg: Color::Rgb(24, 21, 34),
            card: Color::Rgb(33, 29, 45),
            raised: Color::Rgb(46, 40, 61),
            text: Color::Rgb(235, 228, 245),
            muted: Color::Rgb(164, 151, 183),
            line: Color::Rgb(66, 54, 83),
            mint: Color::Rgb(180, 204, 164),
            violet: Color::Rgb(191, 165, 238),
            peach: Color::Rgb(236, 177, 156),
        },
        2 => Palette {
            bg: Color::Rgb(238, 237, 230),
            card: Color::Rgb(249, 248, 242),
            raised: Color::Rgb(225, 229, 217),
            text: Color::Rgb(40, 55, 53),
            muted: Color::Rgb(98, 110, 104),
            line: Color::Rgb(192, 201, 188),
            mint: Color::Rgb(38, 115, 91),
            violet: Color::Rgb(116, 87, 159),
            peach: Color::Rgb(171, 92, 56),
        },
        _ => Palette {
            bg: Color::Rgb(15, 22, 29),
            card: Color::Rgb(21, 31, 40),
            raised: Color::Rgb(31, 45, 55),
            text: Color::Rgb(226, 235, 235),
            muted: Color::Rgb(135, 157, 166),
            line: Color::Rgb(62, 83, 96),
            mint: Color::Rgb(151, 220, 190),
            violet: Color::Rgb(177, 166, 227),
            peach: Color::Rgb(229, 185, 150),
        },
    }
}

impl Palette {
    pub fn surface(self) -> Style {
        Style::default().bg(self.card).fg(self.text)
    }
    pub fn theme(self) -> Theme {
        let mut t = Theme::dark();
        let s = &mut t.styles;
        s.surface = Style::default().bg(self.bg).fg(self.text);
        s.surface_elevated = self.surface();
        s.surface_header = self.surface().fg(self.muted);
        s.surface_modal = self.surface();
        s.surface_popup = self.surface();
        s.text = Style::default().fg(self.text);
        s.text_muted = Style::default().fg(self.muted);
        s.text_placeholder = s.text_muted;
        s.text_heading = s.text.bold();
        s.border = Style::default().fg(self.line);
        s.border_focused = Style::default().fg(self.mint);
        s.interactive = Style::default().bg(self.raised).fg(self.text);
        s.interactive_focused = Style::default().bg(self.raised).fg(self.mint).bold();
        s.primary_action = Style::default().bg(self.mint).fg(self.bg).bold();
        s.primary_action_focused = Style::default().bg(self.peach).fg(self.bg).bold();
        s.primary_action_hover = s.primary_action_focused;
        s.secondary_action = Style::default().bg(self.raised).fg(self.text);
        s.secondary_action_focused = Style::default().bg(self.raised).fg(self.mint).bold();
        s.secondary_action_hover = s.secondary_action_focused;
        s.selected = Style::default().bg(self.raised).fg(self.mint);
        s.selected_focused = s.selected.bold();
        s.list_active = s.selected;
        s.list_active_focused = s.selected_focused;
        s.cursor = Style::default().bg(self.mint).fg(self.bg);
        s.selection_range = Style::default().bg(self.raised);
        s.scrollbar_track = Style::default().fg(self.line);
        s.scrollbar_thumb = Style::default().fg(self.muted);
        s.disabled = s.text_muted;
        t
    }
}

fn card(title: &str, p: Palette) -> Panel<Msg> {
    panel()
        .title(title)
        .border(BorderStyle::Rounded)
        .padding(Padding::new(0, 2, 0, 2))
        .style(p.surface())
        .hover_border_style(Style::default().fg(p.mint))
        .gap(0)
}
fn muted(text: impl Into<String>, p: Palette) -> Label<Msg> {
    label(text).fg(p.muted)
}
fn heading(text: impl Into<String>, p: Palette) -> Label<Msg> {
    label(text).fg(p.text).bold()
}
fn motion_spec(s: &State, millis: u64) -> AnimationSpec {
    AnimationSpec::new(
        Duration::from_millis(if s.quiet { 0 } else { millis }),
        Easing::CubicBezier(0.22, 1.0, 0.36, 1.0),
    )
}
fn enter(s: &State, child: impl IntoWidget<Msg>, key: &str, delay: u64) -> Visual<Msg> {
    visual(child)
        .key(key)
        .animation(motion_spec(s, 550).delay(Duration::from_millis(if s.quiet {
            0
        } else {
            delay
        })))
        .enter(VisualEffect::fade_in())
}
fn run_button(s: &State) -> Button<Msg> {
    button(if s.running.is_some() {
        "Running..."
    } else {
        "Run flow  >"
    })
    .key("run")
    .disabled(s.running.is_some())
    .on_click(|| Msg::Run)
}

fn sidebar(s: &State, p: Palette) -> impl Widget<Msg> {
    let mut navigation = col()
        .gap(1)
        .child(label("  ◈  L U M E N").fg(p.mint).bold())
        .child(muted("  A space to build.", p))
        .child(spacer().height(1))
        .child(muted("  WORKSPACE", p));
    for (index, (id, title)) in PAGES.iter().enumerate() {
        let id = id.to_string();
        navigation = navigation.child(NavLink {
            id: id.clone(),
            title: format!("{}  {}", ["◉", "⇄", "✧", "≡"][index], title),
            selected: s.page == id,
            quiet: s.quiet,
            palette: p,
        });
    }
    grid()
        .columns("1fr")
        .rows("1fr 9")
        .padding(Padding::new(1, 1, 0, 0))
        .gap(1)
        .child(navigation)
        .child(
            card("LOCAL STUDIO", p)
                .child(spacer().height(1))
                .child(label("●  All calm").fg(p.mint))
                .child(muted("No cloud needed.", p))
                .child(spacer().height(1))
                .child(muted("Built on rsille", p))
                .child(muted("Rust, in motion", p)),
        )
}

pub(super) fn view(s: &State) -> impl Widget<Msg> {
    let p = palette(s.theme);
    let title = PAGES
        .iter()
        .find(|(id, _)| *id == s.page)
        .map_or("Overview", |(_, name)| name);
    let top = row()
        .gap(1)
        .align_items(AlignItems::Center)
        .child(label(format!("  studio  /  {title}")).fg(p.muted))
        .child(spacer().fill())
        .child(
            button("⌕  Commands")
                .key("commands")
                .variant(ButtonVariant::Ghost)
                .on_click(|| Msg::Open(Modal::Commands)),
        )
        .child(
            button(["Moon", "Dusk", "Paper"][s.theme])
                .key("theme")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::Theme),
        )
        .child(
            button("+ New flow")
                .key("new")
                .on_click(|| Msg::Open(Modal::NewFlow)),
        );
    let content: Box<dyn Widget<Msg>> = match s.page.as_str() {
        "flows" => Box::new(flows(s, p)),
        "motion" => Box::new(motion_lab(s, p)),
        "notes" => Box::new(notes(s, p)),
        _ => Box::new(overview(s, p)),
    };
    // Fixed natural content dimensions keep every control reachable on small TTYs.
    let body = scroll_view(content)
        .key(format!("viewport-{}", s.page))
        .focusable(true)
        .both()
        .content_width(if s.width < 84 { 78 } else { 0 })
        .content_height(35)
        .scrollbars(ScrollbarVisibility::Auto);
    let workspace = grid()
        .columns("1fr")
        .rows("3 1fr")
        .gap(1)
        .child(top)
        .child(body);
    let layout: Box<dyn Widget<Msg>> = if s.width >= 108 {
        Box::new(
            grid()
                .columns("22 1fr")
                .rows("1fr")
                .gap(2)
                .child(sidebar(s, p))
                .child(workspace),
        )
    } else {
        Box::new(
            grid()
                .columns("1fr")
                .rows("3 1fr")
                .gap(0)
                .child(
                    tabs()
                        .key("navigation")
                        .tabs(PAGES.map(|(id, title)| TabItem::new(id, title)))
                        .selected(&s.page)
                        .on_change(Msg::Page),
                )
                .child(workspace),
        )
    };
    let footer = row()
        .child(muted(
            "  F1 commands   F2 palette   F3 motion   F4 explore",
            p,
        ))
        .child(spacer().fill())
        .child(
            label(if s.quiet {
                "STILL  ·  Esc exit "
            } else {
                "LIVE  ·  Esc exit "
            })
            .fg(p.mint),
        );
    let base = grid()
        .columns("1fr")
        .rows("1fr 1")
        .gap(1)
        .padding(Padding::new(0, 2, 0, 2))
        .style(Style::default().bg(p.bg).fg(p.text))
        .child(layout)
        .child(footer);
    let base: Box<dyn Widget<Msg>> = if s.modal.is_some() {
        Box::new(Backdrop(Box::new(base)))
    } else {
        Box::new(base)
    };
    let mut root = overlay(base);
    if let Some((id, text)) = &s.toast {
        root = root.layer(
            OverlayLayer::new(enter(
                s,
                card("  ✓  A LITTLE UPDATE", p)
                    .child(label(text).wrap_word())
                    .child(muted("Saved in this session", p)),
                &format!("toast-{id}"),
                0,
            ))
            .floating(OverlayAnchor::BottomRight)
            .offset(-3, -2)
            .size(48.min(s.width.saturating_sub(4)), 5),
        );
    }
    if let Some(modal) = s.modal {
        let (widget, height): (Box<dyn Widget<Msg>>, u16) = match modal {
            Modal::Commands => (
                Box::new(
                    command_palette()
                        .key("palette")
                        .height(17)
                        .title("  Go somewhere. Make something.")
                        .placeholder("Search pages or actions...")
                        .search_mode(SelectSearchMode::Fuzzy)
                        .items(
                            PAGES
                                .map(|(id, title)| {
                                    CommandItem::new(id, format!("Explore / {title}"))
                                })
                                .into_iter()
                                .chain([
                                    CommandItem::new("new", "Create a new flow"),
                                    CommandItem::new("run", "Run selected flow"),
                                    CommandItem::new("theme", "Change color palette"),
                                    CommandItem::new("quiet", "Toggle reduced motion"),
                                    CommandItem::new("quit", "Leave Lumen"),
                                ]),
                        )
                        .on_submit(Msg::Command)
                        .on_close(|| Msg::Close),
                ),
                17,
            ),
            Modal::NewFlow => (
                Box::new(
                    dialog()
                        .key("new-flow")
                        .title("  A fresh little beginning")
                        .border(BorderStyle::Rounded)
                        .padding(Padding::uniform(1))
                        .gap(1)
                        .style(p.surface())
                        .child(heading("What would you like to automate?", p))
                        .child(
                            text_input()
                                .key("flow-name")
                                .value(&s.name)
                                .placeholder("Give your flow a name...")
                                .on_change(Msg::Name),
                        )
                        .child(muted("On-demand trigger · lives in this session", p))
                        .child(
                            row()
                                .gap(2)
                                .child(
                                    button("Cancel")
                                        .key("cancel")
                                        .variant(ButtonVariant::Secondary)
                                        .on_click(|| Msg::Close),
                                )
                                .child(
                                    button("Create flow  >")
                                        .key("create")
                                        .disabled(s.name.trim().is_empty())
                                        .on_click(|| Msg::Create),
                                ),
                        ),
                ),
                13,
            ),
        };
        root = root
            .layer(
                OverlayLayer::new(enter(
                    s,
                    animate(widget).enter(Transition::new(
                        TransitionEffect::ScaleFromCenter,
                        motion_spec(s, 220),
                    )),
                    "modal",
                    0,
                ))
                .floating(OverlayAnchor::Center)
                .size(60.min(s.width.saturating_sub(2)), height.min(s.height)),
            )
            .trap_focus();
    }
    Viewport([Box::new(root)])
}

fn overview(s: &State, p: Palette) -> impl Widget<Msg> {
    let count = if s.quiet {
        1284
    } else {
        (1284.0 * Easing::EaseOut.sample((s.age / 1.4).min(1.0))) as u32
    };
    let hero = grid()
        .columns("1.25fr 1fr")
        .rows("1fr")
        .gap(1)
        .child(
            col()
                .gap(1)
                .padding(Padding::new(1, 0, 0, 0))
                .child(label("Y O U R   Q U I E T   C O R N E R").fg(p.mint))
                .child(heading("Less noise. More possibility.", p))
                .child(
                    muted(
                        "A living space, inside your terminal.\nSmall details. A little momentum.",
                        p,
                    )
                    .height(2),
                )
                .child(
                    row().gap(2).child(run_button(s)).child(
                        button("Explore motion")
                            .key("explore")
                            .variant(ButtonVariant::Ghost)
                            .on_click(|| Msg::Page("motion".into())),
                    ),
                ),
        )
        .child(
            col()
                .child(art::orbit(s, p).height(9))
                .child(muted("MOVE TO BEND  ·  CLICK TO RIPPLE", p).align(HorizontalAlign::Center)),
        );
    let metrics = grid()
        .columns("1fr 1fr 1fr")
        .rows("1fr")
        .gap(1)
        .child(metric(
            s,
            p,
            "01 / MOMENTUM",
            format!("{}", count + s.completed),
            "+12.8%  this week",
            p.mint,
            80,
        ))
        .child(metric(
            s,
            p,
            "02 / FLOWS",
            format!("{:02}", s.flows.len()),
            "Room for your routines",
            p.violet,
            140,
        ))
        .child(metric(
            s,
            p,
            "03 / FOCUS",
            "98.6%".into(),
            "Less friction, more flow",
            p.peach,
            200,
        ));
    let signal = card("  SIGNAL / last 60 moments", p)
        .child(
            row()
                .child(heading("Finding a steady rhythm", p))
                .child(spacer().fill())
                .child(label("●  SIMULATED").fg(p.mint)),
        )
        .child(art::signal(s, p).height(7))
        .child(
            row()
                .child(muted("00:00", p))
                .child(spacer().fill())
                .child(muted("00:30", p))
                .child(spacer().fill())
                .child(muted("NOW", p)),
        );
    let activity = card("  IN THE LOOP", p)
        .child(spacer().height(1))
        .child(
            label(if s.running.is_some() {
                "◌  A flow is taking shape"
            } else {
                "✓  All clear. Keep creating."
            })
            .fg(p.mint),
        )
        .child(spacer().height(1))
        .child(
            progress_bar(if s.running.is_some() { s.progress } else { 1.0 })
                .variant(ProgressBarVariant::Line)
                .label(if s.running.is_some() {
                    "Working locally"
                } else {
                    "Workspace ready"
                }),
        )
        .child(spacer().height(1))
        .child(muted(format!("{} session runs completed", s.completed), p))
        .child(spacer().height(1))
        .child(
            button("View all flows  →")
                .key("view-flows")
                .variant(ButtonVariant::Ghost)
                .on_click(|| Msg::Page("flows".into())),
        );
    grid()
        .columns("1fr")
        .rows("14 6 13")
        .gap(1)
        .child(enter(
            s,
            card("  HELLO, EXPLORER", p).child(hero),
            "welcome",
            0,
        ))
        .child(metrics)
        .child(
            grid()
                .columns("1.5fr 1fr")
                .rows("1fr")
                .gap(1)
                .child(enter(s, signal, "signal", 240))
                .child(enter(s, activity, "activity", 300)),
        )
}

fn metric(
    s: &State,
    p: Palette,
    title: &str,
    value: String,
    detail: &str,
    color: Color,
    delay: u64,
) -> impl Widget<Msg> {
    enter(
        s,
        card(title, p)
            .child(spacer().height(1))
            .child(label(value).fg(color).bold())
            .child(muted(detail, p)),
        title,
        delay,
    )
}

fn flows(s: &State, p: Palette) -> impl Widget<Msg> {
    let table = data_table()
        .key("flows-table")
        .borderless()
        .height(12)
        .columns([
            DataTableColumn::new("Flow").id("name").width(24),
            DataTableColumn::new("Trigger").width(13),
            DataTableColumn::new("Runs")
                .width(5)
                .align(TableAlign::Right),
        ])
        .rows(s.flows.iter().map(|f| {
            DataTableRow::new(
                &f.id,
                [f.name.clone(), f.trigger.clone(), f.runs.to_string()],
            )
        }))
        .filter_query(&s.filter)
        .sort(DataTableSort::new(
            "name",
            if s.sort_desc {
                DataTableSortDirection::Desc
            } else {
                DataTableSortDirection::Asc
            },
        ))
        .empty_message("Nothing here yet. Try another search.")
        .on_change(Msg::Inspect)
        .on_submit(Msg::Inspect);
    let selected = s
        .flows
        .iter()
        .find(|f| f.id == s.running.as_deref().unwrap_or(&s.inspected))
        .unwrap_or(&s.flows[0]);
    let phase = if s.running.is_some() {
        (s.progress * 4.0).floor() as usize
    } else {
        4
    };
    let mut steps = col()
        .gap(0)
        .child(heading(&selected.name, p))
        .child(muted("A complete local simulation", p))
        .child(spacer().height(1));
    for (i, title) in [
        "Collect the little things",
        "Find a clearer shape",
        "Bring it all together",
        "Leave something useful",
    ]
    .iter()
    .enumerate()
    {
        steps = steps.child(
            label(format!(
                "{}  {:02}  {title}",
                if i < phase {
                    "✓"
                } else if i == phase {
                    "◌"
                } else {
                    "·"
                },
                i + 1
            ))
            .fg(if i <= phase { p.mint } else { p.muted }),
        );
    }
    steps = steps
        .child(spacer().height(1))
        .child(progress_bar(s.progress).label(if s.running.is_some() {
            "In progress"
        } else {
            "Ready when you are"
        }))
        .child(spacer().height(1))
        .child(run_button(s));
    grid()
        .columns("1fr")
        .rows("4 21 8")
        .gap(1)
        .child(
            col()
                .gap(1)
                .child(heading("Good things, on repeat.", p))
                .child(muted(
                    "Find a flow, give it a nudge, and watch it come together.",
                    p,
                )),
        )
        .child(
            grid()
                .columns("1.3fr 1fr")
                .rows("1fr")
                .gap(1)
                .child(enter(
                    s,
                    card("  YOUR FLOWS", p)
                        .child(
                            row()
                                .gap(1)
                                .child(
                                    text_input()
                                        .key("flow-filter")
                                        .value(&s.filter)
                                        .placeholder("Filter flows...")
                                        .on_change(Msg::Filter),
                                )
                                .child(
                                    button(if s.sort_desc { "Z ↓ A" } else { "A ↑ Z" })
                                        .key("sort")
                                        .variant(ButtonVariant::Secondary)
                                        .on_click(|| Msg::Sort),
                                ),
                        )
                        .child(spacer().height(1))
                        .child(ControlledTable {
                            inner: table,
                            active: s.inspected.clone(),
                        })
                        .child(muted("↑ ↓ inspect  ·  Enter select", p)),
                    "flows",
                    0,
                ))
                .child(enter(
                    s,
                    card("  THE JOURNEY", p).child(steps),
                    "journey",
                    100,
                )),
        )
        .child(enter(
            s,
            card("  RECENT ACTIVITY", p).child(
                log_viewer()
                    .key("flow-log")
                    .height(6)
                    .borderless()
                    .lines(s.logs.clone()),
            ),
            "log",
            180,
        ))
}

fn motion_lab(s: &State, p: Palette) -> impl Widget<Msg> {
    let mut effects = row().gap(1);
    for (i, name) in ["Reveal", "Dissolve", "Sweep", "Type"].iter().enumerate() {
        effects = effects.child(
            button(*name)
                .key(format!("effect-{i}"))
                .variant(if s.effect == i {
                    ButtonVariant::Primary
                } else {
                    ButtonVariant::Secondary
                })
                .on_click(move || Msg::Effect(i)),
        );
    }
    let effect = match s.effect {
        1 => VisualEffect::dissolve(),
        2 => VisualEffect::highlight_sweep().color(p.peach),
        3 => VisualEffect::typewriter(),
        _ => VisualEffect::reveal(WipeDirection::LeftToRight).softness(0.14),
    };
    let progress = if s.quiet {
        1.0
    } else {
        Easing::EaseInOut.sample((s.effect_age / 1.8).min(1.0))
    };
    let preview = card("  A MOMENT OF DELIGHT", p)
        .child(spacer().height(1))
        .child(
            label("S o m e t h i n g   g o o d   i s   t a k i n g   s h a p e .")
                .fg(p.mint)
                .bold(),
        )
        .child(spacer().height(1))
        .child(
            muted(
                "A soft entrance. A thoughtful transition.\nRoom for the content to be itself.",
                p,
            )
            .height(2),
        )
        .child(spacer().height(1))
        .child(
            row()
                .gap(2)
                .child(label("●  Clear").fg(p.mint))
                .child(label("●  Calm").fg(p.violet))
                .child(label("●  Curious").fg(p.peach)),
        );
    let size = if s.expanded { "2fr 1fr" } else { "1fr 1fr" };
    let live = card("  ORBIT / a living canvas", p)
        .child(art::orbit(s, p).height(11))
        .child(
            muted("Move to bend. Click to ripple.", p)
                .wrap_word()
                .height(2),
        );
    let settings = card("  FIND YOUR RHYTHM", p)
        .child(spacer().height(1))
        .child(heading("Motion with intention.", p))
        .child(spacer().height(1))
        .child(
            switch("Reduced motion")
                .key("quiet")
                .checked(s.quiet)
                .on_change(Msg::Quiet),
        )
        .child(spacer().height(1))
        .child(
            button(if s.expanded {
                "Restore balance  ↔"
            } else {
                "More orbit room  ↔"
            })
            .key("expand")
            .variant(ButtonVariant::Secondary)
            .on_click(|| Msg::Expand),
        )
        .child(spacer().height(1))
        .child(
            muted("Try resizing the terminal, too.", p)
                .wrap_word()
                .height(2),
        )
        .child(spacer().height(1))
        .child(
            button("Say hello  ↗")
                .key("notify")
                .variant(ButtonVariant::Ghost)
                .on_click(|| Msg::Notify("Hello from the other side of the terminal.".into())),
        );
    grid()
        .columns("1fr")
        .rows("4 14 15")
        .gap(1)
        .child(
            col()
                .gap(1)
                .child(heading("A terminal with a little soul.", p))
                .child(muted(
                    "Explore the details. Replay them. Make the space your own.",
                    p,
                )),
        )
        .child(
            card("  TRANSITION PLAYGROUND", p)
                .child(
                    row().child(effects).child(spacer().fill()).child(
                        button("↻ Replay")
                            .key("replay")
                            .variant(ButtonVariant::Ghost)
                            .on_click(|| Msg::Replay),
                    ),
                )
                .child(
                    visual(preview)
                        .key("effect-preview")
                        .effect(effect)
                        .progress(progress),
                ),
        )
        .child(
            grid()
                .columns(size)
                .rows("1fr")
                .gap(1)
                .child(
                    animate(live)
                        .key("orbit-layout")
                        .layout(motion_spec(s, 500))
                        .hit_test(HitTestMode::Display),
                )
                .child(
                    animate(settings)
                        .key("settings-layout")
                        .layout(motion_spec(s, 500))
                        .hit_test(HitTestMode::Display),
                ),
        )
}

fn notes(s: &State, p: Palette) -> impl Widget<Msg> {
    let status = if s.revision == s.saved_revision {
        "✓  All changes saved in memory"
    } else {
        "◌  Saving after a little pause..."
    };
    grid()
        .columns("1fr")
        .rows("4 23 6")
        .gap(1)
        .child(
            col()
                .gap(1)
                .child(heading("Catch a thought before it drifts.", p))
                .child(muted(
                    "A Unicode editor, a live preview, and a quiet place for your ideas.",
                    p,
                )),
        )
        .child(
            grid()
                .columns("1fr 1fr")
                .rows("1fr")
                .gap(1)
                .child(enter(
                    s,
                    card("  YOUR WORDS", p)
                        .child(
                            grid().columns("1fr").rows("1fr").child(
                                textarea()
                                    .key("notes-editor")
                                    .height(19)
                                    .value(&s.notes)
                                    .on_change(Msg::Notes),
                            ),
                        )
                        .child(spacer().height(1))
                        .child(label(status).fg(p.mint)),
                    "editor",
                    0,
                ))
                .child(enter(
                    s,
                    card("  A DIFFERENT VIEW", p).child(
                        markdown_viewer(&s.notes)
                            .key("notes-preview")
                            .height(21)
                            .borderless(),
                    ),
                    "preview",
                    120,
                )),
        )
        .child(
            card("  LITTLE DETAILS", p)
                .child(spacer().height(1))
                .child(muted(
                    "450 ms debounce → background task → versioned result",
                    p,
                ))
                .child(muted(
                    "Edit, switch pages, come back. Your words stay with you during this session.",
                    p,
                )),
        )
}

// Hide the background from focus and event routing while preserving its render.
struct Backdrop(Box<dyn Widget<Msg>>);
impl Widget<Msg> for Backdrop {
    fn constraints(&self) -> Constraints {
        self.0.constraints()
    }
    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        self.0.render(chunk, &ctx.child_ctx("preview"));
    }
    fn handle_event(&self, _: &Event, _: &mut EventCtx<Msg>) {}
}
struct Viewport([Box<dyn Widget<Msg>>; 1]);
impl Widget<Msg> for Viewport {
    fn constraints(&self) -> Constraints {
        Constraints::min(1, 44)
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.0
    }
    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        ctx.render_children(chunk, self);
    }
    fn handle_event(&self, _: &Event, _: &mut EventCtx<Msg>) {}
}

// A small custom widget gives sidebar links a quiet, animated focus surface.
struct NavLink {
    id: String,
    title: String,
    selected: bool,
    quiet: bool,
    palette: Palette,
}
impl Widget<Msg> for NavLink {
    fn key(&self) -> Option<&str> {
        Some(&self.id)
    }
    fn focus_config(&self) -> FocusConfig {
        FocusConfig::Leaf
    }
    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 1,
            min_height: 3,
            max_height: Some(3),
            ..Constraints::content()
        }
    }
    fn animate(&self, ctx: &mut rsille::tui::animation::AnimationCtx) -> bool {
        let target = if self.selected || ctx.is_focused() || ctx.is_hovered() {
            self.palette
                .surface()
                .bg(self.palette.raised)
                .fg(self.palette.mint)
        } else {
            self.palette.surface().bg(self.palette.bg)
        };
        ctx.track_style(
            "surface",
            target,
            AnimationSpec::new(
                Duration::from_millis(if self.quiet { 0 } else { 160 }),
                Easing::EaseOut,
            ),
        )
    }
    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        let active = self.selected || ctx.is_focused() || ctx.is_hovered();
        let style = ctx.animation_style("surface").unwrap_or(if active {
            ctx.theme().styles.selected
        } else {
            ctx.theme().styles.surface
        });
        let style = style.to_render_style();
        let _ = chunk.fill(
            0,
            0,
            chunk.area().width(),
            chunk.area().height(),
            ' ',
            style,
        );
        let prefix = if self.selected {
            "▍"
        } else if ctx.is_focused() {
            "›"
        } else {
            " "
        };
        let _ = chunk.set_string(
            0,
            chunk.area().height() / 2,
            &format!("{prefix}  {}", self.title),
            style,
        );
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if ctx.phase() != EventPhase::Target {
            return;
        }
        if matches!(
            event,
            Event::Key(KeyEvent {
                code: KeyCode::Enter | KeyCode::Char(' '),
                kind: KeyEventKind::Press,
                ..
            }) | Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                ..
            })
        ) {
            ctx.emit(Msg::Page(self.id.clone()));
            ctx.set_handled();
        }
    }
}

// Synchronize application selection through the public widget-state lifecycle.
// Render and input still come from the built-in DataTable.
struct ControlledTable {
    inner: DataTable<Msg>,
    active: String,
}
impl Widget<Msg> for ControlledTable {
    fn key(&self) -> Option<&str> {
        Widget::key(&self.inner)
    }
    fn lifecycle(&self, ctx: &mut StateCtx) {
        if ctx.lifecycle() != Lifecycle::Unmount {
            let state = ctx.state_mut::<DataTableState>();
            state.active_row = Some(self.active.clone());
            state.selection.set_cursor(Some(self.active.clone()));
        }
    }
    fn constraints(&self) -> Constraints {
        self.inner.constraints()
    }
    fn layout_style_with(&self, ctx: &MeasureCtx) -> LayoutStyle {
        self.inner.layout_style_with(ctx)
    }
    fn focus_config(&self) -> FocusConfig {
        self.inner.focus_config()
    }
    fn render(&self, chunk: &mut rsille::render::chunk::Chunk, ctx: &RenderCtx) {
        self.inner.render(chunk, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        self.inner.handle_event(event, ctx);
    }
}
