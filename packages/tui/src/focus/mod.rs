/// Focus path: index path from root container to focused widget
/// Example: [0, 2, 1] means root.children[0].children[2].children[1]
pub type FocusPath = Vec<usize>;

/// Focus manager for handling keyboard focus navigation
#[derive(Debug, Clone)]
pub struct FocusManager {
    /// Current focus path
    focus_path: Option<FocusPath>,

    /// Focus chain: all focusable widgets' paths (built during render)
    focus_chain: Vec<FocusPath>,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focus_path: None,
            focus_chain: Vec::new(),
        }
    }

    /// Focus the next widget (Tab key)
    pub fn focus_next(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }

        let current_idx = self.current_index();
        let next_idx = (current_idx + 1) % self.focus_chain.len();
        self.focus_path = Some(self.focus_chain[next_idx].clone());
    }

    /// Focus the previous widget (Shift+Tab)
    pub fn focus_prev(&mut self) {
        if self.focus_chain.is_empty() {
            return;
        }

        let current_idx = self.current_index();
        let prev_idx = if current_idx == 0 {
            self.focus_chain.len() - 1
        } else {
            current_idx - 1
        };
        self.focus_path = Some(self.focus_chain[prev_idx].clone());
    }

    /// Check if the given path is focused
    pub fn is_focused(&self, path: &[usize]) -> bool {
        self.focus_path.as_ref().map(|p| p.as_slice()) == Some(path)
    }

    /// Check if focus is within the given path (for containers)
    pub fn is_focus_within(&self, path: &[usize]) -> bool {
        self.focus_path.as_ref().map_or(false, |focus| {
            focus.starts_with(path)
        })
    }

    /// Get current focus path
    pub fn focus_path(&self) -> Option<&FocusPath> {
        self.focus_path.as_ref()
    }

    /// Set focus chain (called after rebuilding widget tree)
    pub fn set_focus_chain(&mut self, chain: Vec<FocusPath>) {
        self.focus_chain = chain;

        // If current focus path is invalid, focus first widget
        if let Some(ref path) = self.focus_path {
            if !self.focus_chain.contains(path) {
                self.focus_path = self.focus_chain.first().cloned();
            }
        } else if !self.focus_chain.is_empty() {
            // Auto-focus first widget if nothing is focused
            self.focus_path = Some(self.focus_chain[0].clone());
        }
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.focus_path = None;
    }

    /// Get focus chain
    pub fn focus_chain(&self) -> &[FocusPath] {
        &self.focus_chain
    }

    fn current_index(&self) -> usize {
        self.focus_path.as_ref()
            .and_then(|path| {
                self.focus_chain.iter().position(|p| p == path)
            })
            .unwrap_or(0)
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_navigation() {
        let mut manager = FocusManager::new();

        // Set focus chain
        manager.set_focus_chain(vec![
            vec![0],
            vec![1],
            vec![2],
        ]);

        // Should auto-focus first widget
        assert_eq!(manager.focus_path(), Some(&vec![0]));
        assert!(manager.is_focused(&[0]));

        // Focus next
        manager.focus_next();
        assert_eq!(manager.focus_path(), Some(&vec![1]));

        manager.focus_next();
        assert_eq!(manager.focus_path(), Some(&vec![2]));

        // Wrap around
        manager.focus_next();
        assert_eq!(manager.focus_path(), Some(&vec![0]));

        // Focus previous
        manager.focus_prev();
        assert_eq!(manager.focus_path(), Some(&vec![2]));
    }

    #[test]
    fn test_focus_within() {
        let mut manager = FocusManager::new();
        manager.set_focus_chain(vec![vec![0, 1, 2]]);

        assert!(manager.is_focus_within(&[]));
        assert!(manager.is_focus_within(&[0]));
        assert!(manager.is_focus_within(&[0, 1]));
        assert!(manager.is_focus_within(&[0, 1, 2]));
        assert!(!manager.is_focus_within(&[0, 1, 2, 3]));
        assert!(!manager.is_focus_within(&[1]));
    }
}
