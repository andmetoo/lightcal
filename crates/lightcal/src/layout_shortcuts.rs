//! Text-editing shortcuts on non-Latin keyboard layouts.
//!
//! Slint 1.18 picks copy, cut, paste, select all, undo and redo by the text a
//! key produces: Ctrl+V pastes only when the key reports "v". On a Russian or
//! Ukrainian layout the same key reports "м", so the shortcut never fires.
//! Windows edit controls and macOS text views go by the key instead. This
//! filter does the same: a letter from a non-Latin script typed with the
//! shortcut modifier (Ctrl, or Command on macOS) reaches Slint as the
//! US-layout letter of the same physical key. Latin layouts pass through
//! unchanged, including those with accented letters on the letter keys
//! (Turkish F puts "ö" where QWERTY has X), since their own letter is the
//! shortcut there.

use slint::platform::WindowEvent as SlintEvent;
use slint::winit_030::winit::event::{ElementState, WindowEvent};
use slint::winit_030::winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};
use slint::winit_030::EventResult;

/// Tracks the modifier state that winit reports apart from key events, and
/// the keys whose key-down reached Slint remapped.
#[derive(Default)]
pub struct LayoutShortcuts {
    modifiers: ModifiersState,
    /// Remapped key-downs still held, with the letter sent for each, so the
    /// key-up reaches Slint as the same letter whatever the modifiers are then.
    held: Vec<(KeyCode, char)>,
}

impl LayoutShortcuts {
    /// Handles one winit event; `PreventDefault` means it was sent to Slint
    /// already in its Latin form.
    pub fn filter(&mut self, window: &slint::Window, event: &WindowEvent) -> EventResult {
        let Some((code, remapped)) = self.remap(event) else {
            return EventResult::Propagate;
        };
        if window.dispatch_event_with_result(remapped).is_ok() {
            return EventResult::PreventDefault;
        }
        // The original key event goes through instead, so its key-up must too.
        self.forget(code);
        EventResult::Propagate
    }

    /// Updates the state from one winit event and returns the event Slint
    /// should get in its place, with the key it belongs to.
    pub fn remap(&mut self, event: &WindowEvent) -> Option<(KeyCode, SlintEvent)> {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.set_modifiers(modifiers.state());
                None
            }
            WindowEvent::Focused(false) => {
                self.release_all();
                None
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return None;
                };
                self.key(
                    event.physical_key,
                    &event.logical_key,
                    event.state,
                    event.repeat,
                )
                .map(|remapped| (code, remapped))
            }
            _ => None,
        }
    }

    pub fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    /// Forgets held keys: their key-up goes to another window.
    pub fn release_all(&mut self) {
        self.held.clear();
    }

    /// Forgets one held key, so its key-up reaches Slint unchanged.
    pub fn forget(&mut self, code: KeyCode) {
        self.held.retain(|(held, _)| *held != code);
    }

    /// The event Slint should get instead of this key event, or `None` when
    /// the key event should reach Slint unchanged.
    pub fn key(
        &mut self,
        physical: PhysicalKey,
        logical: &Key,
        state: ElementState,
        repeat: bool,
    ) -> Option<SlintEvent> {
        let PhysicalKey::Code(code) = physical else {
            return None;
        };
        if state == ElementState::Released {
            let index = self.held.iter().position(|(held, _)| *held == code)?;
            let (_, letter) = self.held.swap_remove(index);
            return Some(SlintEvent::KeyReleased {
                text: letter.to_string().into(),
            });
        }
        if !repeat {
            // A new key-down: any entry left from a key-up that never came is stale.
            self.forget(code);
        }
        let letter = latin_shortcut_letter(physical, logical, self.modifiers)?;
        let text = letter.to_string().into();
        if repeat {
            // A key-down that went through unchanged keeps its key-up unchanged.
            return Some(SlintEvent::KeyPressRepeated { text });
        }
        self.held.push((code, letter));
        Some(SlintEvent::KeyPressed { text })
    }
}

/// Installs the filter on a window that nothing else filters.
pub fn install(window: &slint::Window) {
    use slint::winit_030::WinitWindowAccessor;
    let mut shortcuts = LayoutShortcuts::default();
    window.on_winit_window_event(move |window, event| shortcuts.filter(window, event));
}

/// The Latin letter Slint should see for a shortcut key, or `None` when the
/// key reports a Latin character, or is not a shortcut.
pub fn latin_shortcut_letter(
    physical: PhysicalKey,
    logical: &Key,
    modifiers: ModifiersState,
) -> Option<char> {
    let shortcut = if cfg!(target_os = "macos") {
        modifiers.super_key()
    } else {
        modifiers.control_key()
    };
    // Ctrl+Alt is AltGr on Windows, which types characters rather than shortcuts.
    if !shortcut || modifiers.alt_key() {
        return None;
    }
    let Key::Character(text) = logical else {
        return None;
    };
    let mut chars = text.chars();
    let (Some(typed), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if !typed.is_alphabetic() || is_latin(typed) {
        return None;
    }
    let PhysicalKey::Code(code) = physical else {
        return None;
    };
    let letter = letter_key(code)?;
    Some(if modifiers.shift_key() {
        letter.to_ascii_uppercase()
    } else {
        letter
    })
}

/// ASCII and the Unicode Latin blocks: Latin-1 Supplement, Latin Extended-A
/// and -B, IPA, Latin Extended Additional, and Latin Extended-C, -D and -E.
fn is_latin(c: char) -> bool {
    matches!(
        c,
        '\0'..='\u{02AF}'
            | '\u{1E00}'..='\u{1EFF}'
            | '\u{2C60}'..='\u{2C7F}'
            | '\u{A720}'..='\u{A7FF}'
            | '\u{AB30}'..='\u{AB6F}'
    )
}

fn letter_key(code: KeyCode) -> Option<char> {
    use KeyCode::*;
    let letters = [
        (KeyA, 'a'),
        (KeyB, 'b'),
        (KeyC, 'c'),
        (KeyD, 'd'),
        (KeyE, 'e'),
        (KeyF, 'f'),
        (KeyG, 'g'),
        (KeyH, 'h'),
        (KeyI, 'i'),
        (KeyJ, 'j'),
        (KeyK, 'k'),
        (KeyL, 'l'),
        (KeyM, 'm'),
        (KeyN, 'n'),
        (KeyO, 'o'),
        (KeyP, 'p'),
        (KeyQ, 'q'),
        (KeyR, 'r'),
        (KeyS, 's'),
        (KeyT, 't'),
        (KeyU, 'u'),
        (KeyV, 'v'),
        (KeyW, 'w'),
        (KeyX, 'x'),
        (KeyY, 'y'),
        (KeyZ, 'z'),
    ];
    letters
        .into_iter()
        .find_map(|(key, letter)| (key == code).then_some(letter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortcut() -> ModifiersState {
        if cfg!(target_os = "macos") {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        }
    }

    fn letter(code: KeyCode, text: &str, modifiers: ModifiersState) -> Option<char> {
        latin_shortcut_letter(
            PhysicalKey::Code(code),
            &Key::Character(text.into()),
            modifiers,
        )
    }

    #[test]
    fn non_latin_shortcuts_use_the_physical_letter() {
        assert_eq!(letter(KeyCode::KeyV, "м", shortcut()), Some('v'));
        assert_eq!(letter(KeyCode::KeyC, "с", shortcut()), Some('c'));
        assert_eq!(letter(KeyCode::KeyX, "ч", shortcut()), Some('x'));
        assert_eq!(letter(KeyCode::KeyA, "ф", shortcut()), Some('a'));
        assert_eq!(letter(KeyCode::KeyZ, "я", shortcut()), Some('z'));
        assert_eq!(
            letter(KeyCode::KeyZ, "Я", shortcut() | ModifiersState::SHIFT),
            Some('Z')
        );
        // Greek, Hebrew and Japanese kana input.
        assert_eq!(letter(KeyCode::KeyV, "ω", shortcut()), Some('v'));
        assert_eq!(letter(KeyCode::KeyV, "ה", shortcut()), Some('v'));
        assert_eq!(letter(KeyCode::KeyV, "ひ", shortcut()), Some('v'));
    }

    #[test]
    fn latin_layouts_and_plain_typing_pass_through() {
        // AZERTY reports "q" on the KeyA position: Slint already handles it.
        assert_eq!(letter(KeyCode::KeyA, "q", shortcut()), None);
        assert_eq!(letter(KeyCode::KeyV, "v", shortcut()), None);
        assert_eq!(letter(KeyCode::KeyV, "м", ModifiersState::empty()), None);
        assert_eq!(letter(KeyCode::KeyV, "м", ModifiersState::SHIFT), None);
    }

    #[test]
    fn accented_latin_letters_stay_their_own_shortcut() {
        // Turkish F: "ö" on the QWERTY X key, "ğ" on E, "ı" on R.
        assert_eq!(letter(KeyCode::KeyX, "ö", shortcut()), None);
        assert_eq!(letter(KeyCode::KeyE, "ğ", shortcut()), None);
        assert_eq!(letter(KeyCode::KeyR, "ı", shortcut()), None);
        // Azerbaijani "ü" on W; Vietnamese letters such as "ư" are Latin too.
        assert_eq!(letter(KeyCode::KeyW, "ü", shortcut()), None);
        assert_eq!(letter(KeyCode::KeyW, "ư", shortcut()), None);
    }

    #[test]
    fn altgr_and_non_letter_keys_pass_through() {
        assert_eq!(
            letter(KeyCode::KeyE, "€", shortcut() | ModifiersState::ALT),
            None
        );
        assert_eq!(letter(KeyCode::Digit1, "№", shortcut()), None);
        assert_eq!(letter(KeyCode::Comma, "б", shortcut()), None);
        // A non-letter symbol on a letter key is not a shortcut letter.
        assert_eq!(letter(KeyCode::KeyQ, "«", shortcut()), None);
    }

    fn press(filter: &mut LayoutShortcuts, code: KeyCode, text: &str) -> Option<SlintEvent> {
        let key = Key::Character(text.into());
        filter.key(PhysicalKey::Code(code), &key, ElementState::Pressed, false)
    }

    fn repeat(filter: &mut LayoutShortcuts, code: KeyCode, text: &str) -> Option<SlintEvent> {
        let key = Key::Character(text.into());
        filter.key(PhysicalKey::Code(code), &key, ElementState::Pressed, true)
    }

    fn release(filter: &mut LayoutShortcuts, code: KeyCode, text: &str) -> Option<SlintEvent> {
        let key = Key::Character(text.into());
        filter.key(PhysicalKey::Code(code), &key, ElementState::Released, false)
    }

    fn pressed(text: &str) -> Option<SlintEvent> {
        Some(SlintEvent::KeyPressed { text: text.into() })
    }

    fn repeated(text: &str) -> Option<SlintEvent> {
        Some(SlintEvent::KeyPressRepeated { text: text.into() })
    }

    fn released(text: &str) -> Option<SlintEvent> {
        Some(SlintEvent::KeyReleased { text: text.into() })
    }

    #[test]
    fn filter_follows_modifier_changes() {
        let mut filter = LayoutShortcuts::default();
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), None);
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);

        filter.set_modifiers(shortcut());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        assert_eq!(repeat(&mut filter, KeyCode::KeyV, "м"), repeated("v"));
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), released("v"));

        filter.set_modifiers(ModifiersState::empty());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), None);
    }

    #[test]
    fn key_up_matches_its_key_down() {
        let mut filter = LayoutShortcuts::default();

        // Shortcut modifier let go before the letter: the key-up stays "v".
        filter.set_modifiers(shortcut());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        filter.set_modifiers(ModifiersState::empty());
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), released("v"));

        // Letter down first, modifier after: the key-up stays "м", while the
        // repeats in between are shortcuts.
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), None);
        filter.set_modifiers(shortcut());
        assert_eq!(repeat(&mut filter, KeyCode::KeyV, "м"), repeated("v"));
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);
    }

    #[test]
    fn several_held_keys_release_separately() {
        let mut filter = LayoutShortcuts::default();
        filter.set_modifiers(shortcut());
        assert_eq!(press(&mut filter, KeyCode::KeyC, "с"), pressed("c"));
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        filter.set_modifiers(ModifiersState::empty());
        assert_eq!(release(&mut filter, KeyCode::KeyC, "с"), released("c"));
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), released("v"));
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);
    }

    #[test]
    fn a_new_key_down_drops_a_lost_key_up() {
        let mut filter = LayoutShortcuts::default();
        filter.set_modifiers(shortcut());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        // The key-up never arrives; the next plain press and its key-up stay "м".
        filter.set_modifiers(ModifiersState::empty());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), None);
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);
    }

    #[test]
    fn a_forgotten_key_releases_unchanged() {
        // `filter` forgets a key whose remapped key-down Slint did not take.
        let mut filter = LayoutShortcuts::default();
        filter.set_modifiers(shortcut());
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        filter.forget(KeyCode::KeyV);
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);
    }

    fn modifiers_changed(state: ModifiersState) -> WindowEvent {
        WindowEvent::ModifiersChanged(state.into())
    }

    #[test]
    fn window_events_drive_modifiers_and_focus() {
        let mut filter = LayoutShortcuts::default();
        assert_eq!(filter.remap(&modifiers_changed(shortcut())), None);
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));

        // Regaining focus keeps the held key; losing it forgets the key.
        assert_eq!(filter.remap(&WindowEvent::Focused(true)), None);
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), released("v"));
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), pressed("v"));
        assert_eq!(filter.remap(&WindowEvent::Focused(false)), None);
        assert_eq!(release(&mut filter, KeyCode::KeyV, "м"), None);

        assert_eq!(
            filter.remap(&modifiers_changed(ModifiersState::empty())),
            None
        );
        assert_eq!(press(&mut filter, KeyCode::KeyV, "м"), None);
    }
}
