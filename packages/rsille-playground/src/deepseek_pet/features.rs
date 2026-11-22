//! Behaviors that can be selected independently for a pet session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Feature {
    /// Move around the terminal, including toward interactive targets.
    Roam,
    /// Pet the whale, open its interaction menu, drag a treat, or lead it around.
    Mouse,
    /// Show command dialogue in a speech bubble.
    Speech,
    /// React to shell command events with gestures.
    CommandReactions,
    /// Look up occasionally when the user types.
    Typing,
    /// Startle when the terminal bell rings.
    Bell,
    /// Eat recently printed characters within reach.
    Forage,
}
