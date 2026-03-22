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
    TogglePlot,
    RefreshSelected,
    RefreshAll,
    Rename,
    Delete,
    Character(char),
    Cancel,
}

pub fn map_event(event: &Event) -> Option<InputAction> {
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
        KeyCode::Char('p') | KeyCode::Char('b') => Some(InputAction::TogglePlot),
        KeyCode::Char('u') => Some(InputAction::RefreshSelected),
        KeyCode::Char('a') => Some(InputAction::RefreshAll),
        KeyCode::Char('n') => Some(InputAction::Rename),
        KeyCode::Delete | KeyCode::Char('d') => Some(InputAction::Delete),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(InputAction::Quit),
        KeyCode::Char(ch) => Some(InputAction::Character(*ch)),
        _ => None,
    }
}
