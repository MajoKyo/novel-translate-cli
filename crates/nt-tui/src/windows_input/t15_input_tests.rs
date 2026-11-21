use super::*;

fn enter(down: bool, scan: u16, enhanced: bool) -> NativeKeyEvent {
    NativeKeyEvent {
        key_down: down,
        repeat_count: 1,
        virtual_key: VK_RETURN,
        scan_code: scan,
        unicode: 13,
        modifiers: KeyModifiers::NONE,
        enhanced,
    }
}

#[test]
fn enter_press_and_release_with_mixed_scan_codes_do_not_leave_a_held_key() {
    for enhanced in [false, true] {
        for (press, release) in [(0, 0x1c), (0x1c, 0), (0, 0), (0x1c, 0x1c)] {
            let mut state = WindowsKeyState::default();
            assert_eq!(
                state
                    .process_key(enter(true, press, enhanced), |_| true)
                    .unwrap()
                    .kind,
                KeyEventKind::Press
            );
            assert_eq!(
                state
                    .process_key(enter(false, release, enhanced), |_| false)
                    .unwrap()
                    .kind,
                KeyEventKind::Release
            );
            assert!(state.physical_down.is_empty());
            assert_eq!(
                state
                    .process_key(enter(true, press, enhanced), |_| true)
                    .unwrap()
                    .kind,
                KeyEventKind::Press
            );
        }
    }
}

#[test]
fn canonical_enter_keeps_held_and_keypad_guards() {
    let mut state = WindowsKeyState::default();
    assert_eq!(
        state
            .process_key(enter(true, 0, false), |_| true)
            .unwrap()
            .kind,
        KeyEventKind::Press
    );
    assert_eq!(
        state
            .process_key(enter(true, 0x1c, false), |_| true)
            .unwrap()
            .kind,
        KeyEventKind::Repeat
    );
    assert_eq!(
        state
            .process_key(enter(true, 0, true), |_| true)
            .unwrap()
            .kind,
        KeyEventKind::Repeat
    );
    state.process_key(enter(false, 0x1c, true), |_| true);
    assert_eq!(
        state
            .process_key(enter(true, 0, false), |_| true)
            .unwrap()
            .kind,
        KeyEventKind::Repeat
    );
    state.process_key(enter(false, 0x1c, false), |_| false);
    assert_eq!(
        state
            .process_key(enter(true, 0, true), |_| true)
            .unwrap()
            .kind,
        KeyEventKind::Press
    );
}

#[test]
fn unknown_scan_releases_clear_text_and_control_keys_in_both_directions() {
    for (virtual_key, unicode, scan) in [
        (VK_RETURN, 13, 0x1c),
        (0x43, 99, 0x2e),
        (0x58, 120, 0x2d),
        (VK_BACK, 8, 0x0e),
        (VK_SPACE, 32, 0x39),
        (VK_TAB, 9, 0x0f),
        (0x4e, 110, 0x31),
    ] {
        for (press, release) in [(0, scan), (scan, 0), (scan, scan)] {
            let mut state = WindowsKeyState::default();
            let mut native = enter(true, press, false);
            native.virtual_key = virtual_key;
            native.unicode = unicode;
            assert_eq!(
                state.process_key(native, |_| true).unwrap().kind,
                KeyEventKind::Press
            );
            native.key_down = false;
            native.scan_code = release;
            assert_eq!(
                state.process_key(native, |_| false).unwrap().kind,
                KeyEventKind::Release
            );
            assert!(
                state.physical_down.is_empty(),
                "Key {virtual_key} remained held after release scan {release}"
            );
            native.key_down = true;
            native.scan_code = press;
            assert_eq!(
                state.process_key(native, |_| true).unwrap().kind,
                KeyEventKind::Press
            );
        }
    }
}

#[test]
fn ctrl_c_after_committed_text_is_new_but_the_same_held_key_stays_repeat() {
    for release_text in [false, true] {
        let mut state = WindowsKeyState::default();
        let mut key = enter(true, 0, false);
        key.virtual_key = 0x43;
        key.unicode = 99;
        state.process_key(key, |_| true);
        if release_text {
            key.key_down = false;
            key.scan_code = 0x2e;
            state.process_key(key, |_| false);
        }
        key.key_down = true;
        key.scan_code = 0;
        key.unicode = 3;
        key.modifiers = KeyModifiers::CONTROL;
        let event = state.process_key(key, |_| true).unwrap();
        assert_eq!(
            event.kind,
            if release_text {
                KeyEventKind::Press
            } else {
                KeyEventKind::Repeat
            }
        );
        assert_eq!(
            crate::input::is_shutdown_request(&Event::Key(event)),
            release_text
        );
    }
}

#[test]
fn a_known_scan_release_does_not_clear_another_known_physical_key() {
    let mut state = WindowsKeyState::default();
    state.process_key(enter(true, 0x1c, false), |_| true);
    state.process_key(enter(true, 0x4d, false), |_| true);
    state.process_key(enter(false, 0x1c, false), |_| true);
    assert!(state.physical_down.iter().any(|key| key.scan_code == 0x4d));
    assert!(!state.physical_down.iter().any(|key| key.scan_code == 0x1c));
}
