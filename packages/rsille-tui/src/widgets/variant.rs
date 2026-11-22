//! Shared API for widgets with visual variants.

/// Common builder API for widgets that expose a primary visual variant.
///
/// Behavior modes such as selection, searching, and navigation intentionally use
/// dedicated `*_mode` methods instead of this trait.
pub trait VariantWidget: Sized {
    /// The visual variant enum supported by this widget.
    type Variant: Copy + Default;

    /// Sets the primary visual variant and returns the updated widget.
    fn variant(self, variant: Self::Variant) -> Self;
}
