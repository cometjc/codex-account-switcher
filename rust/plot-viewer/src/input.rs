use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    Quit,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    NextFocus,
    PreviousFocus,
    ResetZoom,
    ToggleSolo,
    XWindow(u8),
    FilterEnter,
    ZoomIn,
    ZoomOut,
    YZoomIn,
    YZoomOut,
    RefreshSelected,
    RefreshAll,
    Rename,
    ToggleProfiles,
    Delete,
    Character(char),
    MoveToStart,
    MoveToEnd,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    Normal,
    TextEntry,
}

pub fn map_event(event: &Event, context: InputContext) -> Option<InputAction> {
    let Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        ..
    }) = event
    else {
        return None;
    };

    if *kind != KeyEventKind::Press {
        return None;
    }

    if modifiers.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
        return Some(InputAction::Quit);
    }

    if matches!(context, InputContext::TextEntry) {
        return match code {
            KeyCode::Esc => Some(InputAction::Cancel),
            KeyCode::Enter => Some(InputAction::Enter),
            KeyCode::Backspace | KeyCode::Delete => Some(InputAction::Backspace),
            KeyCode::Home => Some(InputAction::MoveToStart),
            KeyCode::End => Some(InputAction::MoveToEnd),
            KeyCode::Up => Some(InputAction::Up),
            KeyCode::Down => Some(InputAction::Down),
            KeyCode::Left => Some(InputAction::Left),
            KeyCode::Right => Some(InputAction::Right),
            KeyCode::Tab => Some(InputAction::Character('\t')),
            KeyCode::Char(ch) => Some(InputAction::Character(*ch)),
            _ => None,
        };
    }

    match code {
        KeyCode::Char('q') => Some(InputAction::Quit),
        KeyCode::Esc => Some(InputAction::Cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(InputAction::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(InputAction::Down),
        KeyCode::Left | KeyCode::Char('h') => Some(InputAction::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(InputAction::Right),
        KeyCode::Enter => Some(InputAction::Enter),
        KeyCode::Backspace => Some(InputAction::Backspace),
        KeyCode::Tab => Some(InputAction::NextFocus),
        KeyCode::BackTab => Some(InputAction::PreviousFocus),
        KeyCode::Char('z') => Some(InputAction::ResetZoom),
        KeyCode::Char('s') => Some(InputAction::ToggleSolo),
        KeyCode::Char('=') => Some(InputAction::ZoomIn),
        KeyCode::Char('-') => Some(InputAction::ZoomOut),
        KeyCode::Char('[') => Some(InputAction::YZoomOut),
        KeyCode::Char(']') => Some(InputAction::YZoomIn),
        KeyCode::Char('1') => Some(InputAction::XWindow(1)),
        KeyCode::Char('3') => Some(InputAction::XWindow(3)),
        KeyCode::Char('7') => Some(InputAction::XWindow(7)),
        KeyCode::Char('p') => Some(InputAction::ToggleProfiles),
        KeyCode::Char('/') => Some(InputAction::FilterEnter),
        KeyCode::Char('u') => Some(InputAction::RefreshSelected),
        KeyCode::Char('a') => Some(InputAction::RefreshAll),
        KeyCode::Char('r') => Some(InputAction::Rename),
        KeyCode::Delete | KeyCode::Char('d') => Some(InputAction::Delete),
        KeyCode::Char(ch) => Some(InputAction::Character(*ch)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn normal_context_keeps_global_hotkeys() {
        assert_eq!(
            map_event(&key(KeyCode::Char('r')), InputContext::Normal),
            Some(InputAction::Rename)
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('z')), InputContext::Normal),
            Some(InputAction::ResetZoom)
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('q')), InputContext::Normal),
            Some(InputAction::Quit)
        );
    }

    #[test]
    fn text_entry_context_treats_hotkeys_as_characters() {
        assert_eq!(
            map_event(&key(KeyCode::Char('n')), InputContext::TextEntry),
            Some(InputAction::Character('n'))
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('d')), InputContext::TextEntry),
            Some(InputAction::Character('d'))
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('u')), InputContext::TextEntry),
            Some(InputAction::Character('u'))
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('a')), InputContext::TextEntry),
            Some(InputAction::Character('a'))
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('q')), InputContext::TextEntry),
            Some(InputAction::Character('q'))
        );
        assert_eq!(
            map_event(&key(KeyCode::Char('/')), InputContext::TextEntry),
            Some(InputAction::Character('/'))
        );
    }

    #[test]
    fn text_entry_context_maps_home_and_end() {
        assert_eq!(
            map_event(&key(KeyCode::Home), InputContext::TextEntry),
            Some(InputAction::MoveToStart)
        );
        assert_eq!(
            map_event(&key(KeyCode::End), InputContext::TextEntry),
            Some(InputAction::MoveToEnd)
        );
    }
}
