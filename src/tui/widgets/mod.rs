//! Built-in widgets

pub mod collections;
pub mod controls;
pub mod display;
pub mod layout;
pub mod motion;
pub mod navigation;
mod variant;

pub use collections::{data_table, file_explorer, list, selection, tree};
pub use controls::{button, calendar, checkbox, radio_group, select, switch, text_input, textarea};
pub use display::{canvas, content, divider, label, progress, spacer};
pub use layout::{collapsible, dialog, panel};
pub use motion::{animated, visual};
pub use navigation::{command_palette, menu, tabs};

pub use animated::{animate, Animated};
pub use button::{button, Button, ButtonVariant};
pub use calendar::{calendar, Calendar, CalendarDate, CalendarState};
pub use canvas::{canvas, CanvasContext, CanvasWidget};
pub use checkbox::{checkbox, Checkbox};
pub use collapsible::{collapsible, Collapsible};
pub use command_palette::{command_palette, CommandItem, CommandPalette, CommandPaletteState};
pub use content::{
    code_viewer, diff_viewer, log_viewer, markdown_viewer, CodeViewer, ContentViewerState,
    DiffViewer, LogLevel, LogLine, LogViewer, MarkdownViewer,
};
pub use data_table::{
    data_table, DataTable, DataTableColumn, DataTableNavigationMode, DataTableRow, DataTableSort,
    DataTableSortDirection, DataTableState, TableAlign,
};
pub use dialog::{dialog, Dialog};
pub use divider::{divider, Divider, DividerDirection, DividerTextPosition, DividerVariant};
pub use file_explorer::{
    file_explorer, FileExplorer, FileExplorerItem, FileExplorerItemKind, FileExplorerState,
};
pub use label::{label, Label, TextWrap};
pub use list::{list, List, ListItem, ListState};
pub use menu::{menu, Menu, MenuItem, MenuState};
pub use panel::{panel, Panel};
pub use progress::{
    loading_indicator, progress_bar, LoadingIndicator, ProgressBar, ProgressBarVariant,
};
pub use radio_group::{radio_group, RadioGroup, RadioGroupState, RadioOption};
pub use select::{select, Select, SelectOption, SelectSearchMode, SelectState};
pub use selection::{SelectionMode, SelectionState};
pub use spacer::{spacer, Spacer};
pub use switch::{switch, toggle, Switch};
pub use tabs::{tabs, TabItem, Tabs, TabsState};
pub use text_input::{text_input, TextInput, TextInputState, TextInputVariant};
pub use textarea::{textarea, TextArea, TextAreaState, TextAreaVariant};
pub use tree::{tree, Tree, TreeItem, TreeState};
pub use variant::VariantWidget;
pub use visual::{
    looping_visual_spec, visual, BlurMode, CellEffect, CellSample, CustomCellEffect, DissolveMode,
    GradientDirection, GradientTarget, IntoVisualEnter, IntoVisualExit, LargeAreaPolicy,
    StaggerMode, TerminalVisualCapabilities, TypewriterMode, Visual, VisualAnchor, VisualConfig,
    VisualCtx, VisualDegradation, VisualEffect, VisualEffectCost, VisualPerformanceConfig,
    VisualProfile, WaveAxis, WipeDirection, WipeMode,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_variant_widget<W, V>()
    where
        W: VariantWidget<Variant = V>,
        V: Copy + Default,
    {
    }

    #[test]
    fn built_in_visual_variants_use_shared_api() {
        assert_variant_widget::<Button<()>, ButtonVariant>();
        assert_variant_widget::<Divider<()>, DividerVariant>();
        assert_variant_widget::<ProgressBar<()>, ProgressBarVariant>();
        assert_variant_widget::<TextArea<()>, TextAreaVariant>();
        assert_variant_widget::<TextInput<()>, TextInputVariant>();
    }
}
