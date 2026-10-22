//! Shared API for widgets with visual variants.

/// Common builder API for widgets that expose a primary visual variant.
///
/// Behavior modes such as selection, searching, and navigation intentionally use
/// dedicated `*_mode` methods instead of this trait.
pub trait VariantWidget: Sized {
    type Variant: Copy + Default;

    fn variant(self, variant: Self::Variant) -> Self;
}
