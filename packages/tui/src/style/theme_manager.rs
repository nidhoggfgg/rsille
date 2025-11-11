//! Global theme management

use super::Theme;
use std::sync::{Arc, RwLock};

/// Global theme manager singleton
pub struct ThemeManager {
    current_theme: Arc<RwLock<Theme>>,
}

impl ThemeManager {
    /// Create a new theme manager with the given initial theme
    fn new(theme: Theme) -> Self {
        Self {
            current_theme: Arc::new(RwLock::new(theme)),
        }
    }

    /// Get the global theme manager instance
    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::Lazy<ThemeManager> =
            once_cell::sync::Lazy::new(|| ThemeManager::new(Theme::dark()));
        &INSTANCE
    }

    /// Set the current theme
    ///
    /// # Example
    /// ```no_run
    /// use tui::style::{Theme, ThemeManager};
    ///
    /// ThemeManager::global().set_theme(Theme::light());
    /// ```
    pub fn set_theme(&self, theme: Theme) {
        let mut current = self.current_theme.write().unwrap();
        *current = theme;
    }

    /// Get a clone of the current theme
    ///
    /// # Example
    /// ```no_run
    /// use tui::style::ThemeManager;
    ///
    /// let theme = ThemeManager::global().get_theme();
    /// println!("Current theme: {}", theme.name);
    /// ```
    pub fn get_theme(&self) -> Theme {
        self.current_theme.read().unwrap().clone()
    }

    /// Execute a function with read access to the current theme
    ///
    /// This is more efficient than `get_theme()` when you only need
    /// to read theme values without cloning the entire theme.
    ///
    /// # Example
    /// ```no_run
    /// use tui::style::ThemeManager;
    ///
    /// let primary_color = ThemeManager::global().with_theme(|theme| {
    ///     theme.colors.primary
    /// });
    /// ```
    pub fn with_theme<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Theme) -> R,
    {
        let theme = self.current_theme.read().unwrap();
        f(&theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_default() {
        let manager = ThemeManager::global();
        let theme = manager.get_theme();
        assert_eq!(theme.name, "dark");
    }

    #[test]
    fn test_theme_manager_set_theme() {
        let manager = ThemeManager::global();
        manager.set_theme(Theme::light());
        let theme = manager.get_theme();
        assert_eq!(theme.name, "light");
        // Reset to dark for other tests
        manager.set_theme(Theme::dark());
    }

    #[test]
    fn test_theme_manager_with_theme() {
        let manager = ThemeManager::global();
        // Ensure theme is dark before testing
        manager.set_theme(Theme::dark());
        let name = manager.with_theme(|theme| theme.name.clone());
        assert_eq!(name, "dark");
    }
}
