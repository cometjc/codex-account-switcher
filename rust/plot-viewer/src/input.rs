use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Quit,
    NextProfile,
    PreviousProfile,
    NextFocus,
    PreviousFocus,
}

pub fn map_event(event: &Event) -> Option<InputAction> {
    let Event::Key(KeyEvent {
        code,
        modifiers,
        ..
    }) = event else {
        return None;
    };

    match code {
        KeyCode::Char('q') | KeyCode::Esc => Some(InputAction::Quit),
        KeyCode::Left => Some(InputAction::PreviousProfile),
        KeyCode::Right => Some(InputAction::NextProfile),
        KeyCode::Tab => Some(InputAction::NextFocus),
        KeyCode::BackTab => Some(InputAction::PreviousFocus),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::Quit)
        }
        _ => None,
    }
}
