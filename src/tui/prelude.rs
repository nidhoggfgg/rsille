//! Prelude — import everything you need with `use rsille::tui::prelude::*;`

pub use crate::canvas::Encoding;

pub use crate::tui::animation::{
    AnimationSpec, AnimationTheme, AreaF, ClipMode, Direction as AnimationDirection, Easing,
    HitTestMode, InitialAnimation, LayoutSnapshot, LayoutTransition, MotionPolicy, Presence,
    Repeat, SharedTransition, Timeline, Transition, TransitionEffect,
};
pub use crate::tui::app::{App, AppExit, Application, ExitReason, FrameInfo, Mode, QuitBehavior};
pub use crate::tui::effect::{
    CancellationToken, Effect, Request, RequestContext, RequestEvent, RequestId, RequestOutcome,
    RequestPhase, RequestState, RetryPolicy, Task, TaskContext, TaskId, TaskOutcome, TaskState,
    TaskStatus, UpdateCtx,
};
pub use crate::tui::error::{WidgetError, WidgetResult};
pub use crate::tui::event::{
    Event, InputCapabilities, InputEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    KeyboardMode, MouseButton, MouseEvent, MouseEventKind,
};
pub use crate::tui::focus::{FocusConfig, FocusScope, ScopeEntry};
pub use crate::tui::layout::{
    clamp_scroll_offset, col, ensure_item_visible, grid, max_scroll_offset, overlay, row,
    scroll_lines, scroll_offset_for_item, scroll_view, scrollbar, split, stack, AlignItems,
    AnchorRect, AxisLimit, ChildLayout, Constraints, Direction, Flex, Grid, GridLine,
    GridPlacement, GridTrack, HorizontalAlign, Invalidation, JustifyContent, Layout,
    LayoutGeometry, LayoutStats, LayoutStyle, MeasuredSize, Overflow, Overlay, OverlayAnchor,
    OverlayLayer, OverlayPlacement, ScrollAxis, ScrollState, ScrollView, Scrollbar,
    ScrollbarOrientation, ScrollbarVisibility, SizeProposal, Split, SplitDirection, SplitSize,
    SplitState, Stack, VerticalAlign,
};
pub use crate::tui::runtime::{AppHandle, Runtime, StepResult};
pub use crate::tui::shell::{
    CommandDescriptor, CommandMatch, CommandRouter, Hotkey, HotkeyRegistry, ModalManager,
    Navigator, Notification, NotificationCenter, NotificationId, NotificationLevel,
};
pub use crate::tui::state::{Derived, FormState, Store, StoreKey};
pub use crate::tui::style::{BorderStyle, Color, EffectSlot, Padding, Style, Theme, ThemeEffects};
pub use crate::tui::widget::{
    EventCtx, EventPhase, FocusRequest, IntoWidget, Lifecycle, MappedWidget, MeasureCtx, RenderCtx,
    ScopedWidget, StateCtx, Widget, WidgetExt, WidgetId, WidgetKey, WidgetPath, WidgetStore,
};
pub use crate::tui::widgets::{
    animate, button, calendar, canvas, checkbox, code_viewer, collapsible, command_palette,
    data_table, dialog, diff_viewer, divider, file_explorer, label, list, loading_indicator,
    log_viewer, looping_visual_spec, markdown_viewer, menu, panel, progress_bar, radio_group,
    select, spacer, switch, tabs, text_input, textarea, toggle, tree, visual, Animated, BlurMode,
    Button, ButtonVariant, Calendar, CalendarDate, CalendarState, CanvasContext, CanvasWidget,
    CellEffect, CellSample, Checkbox, CodeViewer, Collapsible, CommandItem, CommandPalette,
    CommandPaletteState, ContentViewerState, CustomCellEffect, DataTable, DataTableColumn,
    DataTableNavigationMode, DataTableRow, DataTableSort, DataTableSortDirection, DataTableState,
    Dialog, DiffViewer, DissolveMode, Divider, DividerDirection, DividerTextPosition,
    DividerVariant, FileExplorer, FileExplorerItem, FileExplorerItemKind, FileExplorerState,
    GradientDirection, GradientTarget, IntoVisualEnter, IntoVisualExit, Label, LargeAreaPolicy,
    List, ListItem, ListState, LoadingIndicator, LogLevel, LogLine, LogViewer, MarkdownViewer,
    Menu, MenuItem, MenuState, Panel, ProgressBar, ProgressBarVariant, RadioGroup, RadioGroupState,
    RadioOption, Select, SelectOption, SelectSearchMode, SelectState, SelectionMode,
    SelectionState, Spacer, StaggerMode, Switch, TabItem, TableAlign, Tabs, TabsState,
    TerminalVisualCapabilities, TextArea, TextAreaState, TextAreaVariant, TextInput,
    TextInputVariant, TextWrap, Tree, TreeItem, TreeState, TypewriterMode, VariantWidget, Visual,
    VisualAnchor, VisualConfig, VisualCtx, VisualDegradation, VisualEffect, VisualEffectCost,
    VisualPerformanceConfig, VisualProfile, WaveAxis, WipeDirection, WipeMode,
};
pub use crate::tui::InlineMouseMode;

pub use crate::tui::scope::Scope;
pub use crate::tui::subscription::Subscription;

pub use crate::tui::text::{AmbiguousWidth, EditBuffer, Line, Span, Text, TextMetrics};

pub use crate::tui::layout::{portal, Portal, PortalLayer};
pub use crate::tui::widgets::{
    CanvasPoint, CanvasPointer, CanvasViewport, DataSource, RowData, VirtualList,
    VirtualListConfig, VirtualListState, WorldCanvas,
};

pub use crate::tui::gesture::{
    CellDelta, CellPoint, DragAxis, DragCancelReason, DragEvent, DragState, DragUpdate, ResizeEdge,
    WindowRect,
};

pub use crate::tui::widgets::{drag_region, DragRegion};

pub use crate::tui::layout::{window_host, WindowHost, WindowHostState};
pub use crate::tui::widgets::{
    floating_window, FloatingWindow, FloatingWindowState, WindowVisibility,
};
