//! Terminal setup and rollback, independent of event dispatch.
use crate::render::{DrawErr, InputCapabilities, KeyboardMode};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::warn;
use std::io::{self, stdout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalAction {
    EnterScreen,
    ClearScreen,
    EnableRaw,
    EnablePaste,
    EnableMouse,
    PushKeyboard,
    EnableFocus,
    HideCursor,
    DisableFocus,
    PopKeyboard,
    DisablePaste,
    ShowCursor,
    DisableMouse,
    DisableRaw,
    LeaveScreen,
}

pub(super) trait TerminalBackend {
    fn apply(&mut self, action: TerminalAction) -> io::Result<()>;
    fn keyboard_supported(&mut self) -> io::Result<bool>;
    fn native_key_releases(&self) -> bool;
}

pub(super) struct SystemTerminal;
impl TerminalBackend for SystemTerminal {
    fn apply(&mut self, action: TerminalAction) -> io::Result<()> {
        match action {
            TerminalAction::EnterScreen => execute!(stdout(), EnterAlternateScreen),
            TerminalAction::ClearScreen => execute!(stdout(), Clear(ClearType::All)),
            TerminalAction::EnableRaw => crossterm::terminal::enable_raw_mode(),
            TerminalAction::EnablePaste => execute!(stdout(), EnableBracketedPaste),
            TerminalAction::EnableMouse => execute!(stdout(), EnableMouseCapture),
            TerminalAction::PushKeyboard => {
                execute!(stdout(), PushKeyboardEnhancementFlags(held_key_flags()))
            }
            TerminalAction::EnableFocus => execute!(stdout(), EnableFocusChange),
            TerminalAction::HideCursor => execute!(stdout(), Hide),
            TerminalAction::DisableFocus => execute!(stdout(), DisableFocusChange),
            TerminalAction::PopKeyboard => execute!(stdout(), PopKeyboardEnhancementFlags),
            TerminalAction::DisablePaste => execute!(stdout(), DisableBracketedPaste),
            TerminalAction::ShowCursor => execute!(stdout(), Show),
            TerminalAction::DisableMouse => execute!(stdout(), DisableMouseCapture),
            TerminalAction::DisableRaw => crossterm::terminal::disable_raw_mode(),
            TerminalAction::LeaveScreen => execute!(stdout(), LeaveAlternateScreen),
        }
    }
    fn keyboard_supported(&mut self) -> io::Result<bool> {
        crossterm::terminal::supports_keyboard_enhancement()
    }
    fn native_key_releases(&self) -> bool {
        cfg!(windows)
    }
}

pub(super) struct TerminalGuard<B: TerminalBackend = SystemTerminal> {
    backend: B,
    hide_cursor: bool,
    mouse_capture: bool,
    raw_mode: bool,
    alt_screen: bool,
    bracketed_paste: bool,
    enhanced_keyboard: bool,
    focus_events: bool,
}

impl TerminalGuard {
    pub(super) fn new(
        alt_screen: bool,
        raw_mode: bool,
        mouse_capture: bool,
        hide_cursor: bool,
        keyboard_mode: KeyboardMode,
        focus_events: bool,
    ) -> Result<Self, DrawErr> {
        Self::with_backend(
            SystemTerminal,
            alt_screen,
            raw_mode,
            mouse_capture,
            hide_cursor,
            keyboard_mode,
            focus_events,
        )
    }
}

impl<B: TerminalBackend> TerminalGuard<B> {
    fn with_backend(
        backend: B,
        alt_screen: bool,
        raw_mode: bool,
        mouse_capture: bool,
        hide_cursor: bool,
        keyboard_mode: KeyboardMode,
        focus_events: bool,
    ) -> Result<Self, DrawErr> {
        // Keep the guard live during setup so partial failures roll back.
        let mut guard = Self {
            backend,
            hide_cursor: false,
            mouse_capture: false,
            raw_mode: false,
            alt_screen: false,
            bracketed_paste: false,
            enhanced_keyboard: false,
            focus_events: false,
        };
        if alt_screen {
            guard
                .backend
                .apply(TerminalAction::EnterScreen)
                .map_err(DrawErr::TerminalSetup)?;
            guard.alt_screen = true;
            guard
                .backend
                .apply(TerminalAction::ClearScreen)
                .map_err(DrawErr::TerminalSetup)?;
        }
        if raw_mode {
            guard
                .backend
                .apply(TerminalAction::EnableRaw)
                .map_err(DrawErr::TerminalSetup)?;
            guard.raw_mode = true;
            guard
                .backend
                .apply(TerminalAction::EnablePaste)
                .map_err(DrawErr::TerminalSetup)?;
            guard.bracketed_paste = true;
        }
        if mouse_capture {
            guard
                .backend
                .apply(TerminalAction::EnableMouse)
                .map_err(DrawErr::TerminalSetup)?;
            guard.mouse_capture = true;
        }
        // Query before constructing EventStream: both use crossterm's reader.
        // A timeout or unsupported terminal is a compatibility fallback.
        if raw_mode && keyboard_mode == KeyboardMode::Auto && !guard.backend.native_key_releases() {
            match guard.backend.keyboard_supported() {
                Ok(true) => {
                    guard.enhanced_keyboard = true;
                    guard
                        .backend
                        .apply(TerminalAction::PushKeyboard)
                        .map_err(DrawErr::TerminalSetup)?;
                }
                Ok(false) => {}
                Err(error) => warn!("Keyboard negotiation failed; using legacy input: {error}"),
            }
        }
        if focus_events {
            guard.focus_events = true;
            guard
                .backend
                .apply(TerminalAction::EnableFocus)
                .map_err(DrawErr::TerminalSetup)?;
        }
        if hide_cursor {
            guard
                .backend
                .apply(TerminalAction::HideCursor)
                .map_err(DrawErr::TerminalSetup)?;
            guard.hide_cursor = true;
        }
        Ok(guard)
    }

    pub(super) fn input_capabilities(&self) -> InputCapabilities {
        InputCapabilities {
            key_releases: self.enhanced_keyboard
                || (self.backend.native_key_releases() && self.raw_mode),
            focus_events: self.focus_events,
        }
    }
}

fn held_key_flags() -> KeyboardEnhancementFlags {
    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
}

impl<B: TerminalBackend> Drop for TerminalGuard<B> {
    fn drop(&mut self) {
        if self.focus_events {
            let _ = self.backend.apply(TerminalAction::DisableFocus);
        }
        // Pop on the same screen on which we pushed (the stacks are separate).
        if self.enhanced_keyboard {
            let _ = self.backend.apply(TerminalAction::PopKeyboard);
        }
        if self.bracketed_paste {
            let _ = self.backend.apply(TerminalAction::DisablePaste);
        }
        if self.hide_cursor {
            let _ = self.backend.apply(TerminalAction::ShowCursor);
        }
        if self.mouse_capture {
            let _ = self.backend.apply(TerminalAction::DisableMouse);
        }
        if self.raw_mode {
            let _ = self.backend.apply(TerminalAction::DisableRaw);
        }
        if self.alt_screen {
            let _ = self.backend.apply(TerminalAction::LeaveScreen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use TerminalAction::*;

    struct FakeTerminal {
        actions: Rc<RefCell<Vec<TerminalAction>>>,
        queries: Rc<Cell<usize>>,
        support: Result<bool, io::ErrorKind>,
        native: bool,
        fail: Vec<TerminalAction>,
    }

    impl Default for FakeTerminal {
        fn default() -> Self {
            Self {
                actions: Rc::default(),
                queries: Rc::default(),
                support: Ok(true),
                native: false,
                fail: Vec::new(),
            }
        }
    }

    impl TerminalBackend for FakeTerminal {
        fn apply(&mut self, action: TerminalAction) -> io::Result<()> {
            self.actions.borrow_mut().push(action);
            if self.fail.contains(&action) {
                Err(io::Error::other("injected terminal failure"))
            } else {
                Ok(())
            }
        }
        fn keyboard_supported(&mut self) -> io::Result<bool> {
            self.queries.set(self.queries.get() + 1);
            self.support.map_err(io::Error::from)
        }
        fn native_key_releases(&self) -> bool {
            self.native
        }
    }

    fn enter(backend: FakeTerminal) -> Result<TerminalGuard<FakeTerminal>, DrawErr> {
        TerminalGuard::with_backend(backend, true, true, true, true, KeyboardMode::Auto, true)
    }

    #[test]
    fn normal_exit_and_unwinding_restore_the_keyboard_on_its_screen() {
        for panic in [false, true] {
            let backend = FakeTerminal::default();
            let actions = backend.actions.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let guard = enter(backend).unwrap();
                assert_eq!(
                    guard.input_capabilities(),
                    InputCapabilities {
                        key_releases: true,
                        focus_events: true,
                    }
                );
                if panic {
                    panic!("injected application panic");
                }
            }));
            assert_eq!(result.is_err(), panic);
            assert!(actions.borrow().ends_with(&[
                DisableFocus,
                PopKeyboard,
                DisablePaste,
                ShowCursor,
                DisableMouse,
                DisableRaw,
                LeaveScreen,
            ]));
        }
    }

    #[test]
    fn partial_setup_rolls_back_and_cleanup_continues_after_errors() {
        for failure in [
            ClearScreen,
            EnablePaste,
            EnableMouse,
            PushKeyboard,
            EnableFocus,
            HideCursor,
        ] {
            let backend = FakeTerminal {
                fail: vec![failure, DisableFocus, PopKeyboard],
                ..FakeTerminal::default()
            };
            let actions = backend.actions.clone();
            assert!(enter(backend).is_err());
            let actions = actions.borrow();
            assert_eq!(actions.last(), Some(&LeaveScreen));
            if [PushKeyboard, EnableFocus, HideCursor].contains(&failure) {
                assert!(actions.contains(&PopKeyboard));
            } else {
                assert!(!actions.contains(&PopKeyboard));
            }
            if failure == EnableFocus || failure == HideCursor {
                assert!(actions.contains(&DisableFocus));
            }
            if failure != ClearScreen {
                assert!(actions.contains(&DisableRaw));
            }
        }
    }

    #[test]
    fn unsupported_and_failed_queries_fall_back_without_popping_another_mode() {
        for support in [Ok(false), Err(io::ErrorKind::TimedOut)] {
            let backend = FakeTerminal {
                support,
                ..FakeTerminal::default()
            };
            let actions = backend.actions.clone();
            let queries = backend.queries.clone();
            let guard = enter(backend).unwrap();
            assert!(!guard.input_capabilities().key_releases);
            drop(guard);
            assert_eq!(queries.get(), 1);
            assert!(!actions.borrow().contains(&PushKeyboard));
            assert!(!actions.borrow().contains(&PopKeyboard));
        }
    }

    #[test]
    fn legacy_non_raw_and_native_backends_do_not_query_the_protocol() {
        for (mode, raw, native) in [
            (KeyboardMode::Legacy, true, false),
            (KeyboardMode::Auto, false, false),
            (KeyboardMode::Auto, true, true),
        ] {
            let backend = FakeTerminal {
                native,
                ..FakeTerminal::default()
            };
            let queries = backend.queries.clone();
            let actions = backend.actions.clone();
            let guard = TerminalGuard::with_backend(backend, false, raw, false, false, mode, false)
                .unwrap();
            assert_eq!(guard.input_capabilities().key_releases, native && raw);
            drop(guard);
            assert_eq!(queries.get(), 0);
            assert!(!actions.borrow().contains(&PushKeyboard));
            assert!(!actions.borrow().contains(&PopKeyboard));
        }
    }
}
