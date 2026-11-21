use super::*;
use crate::import_commands::BookEdit;
use nt_application::volume_import::{GenerationId, SeriesId};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

struct Commands {
    reply: Mutex<mpsc::Receiver<Option<PathBuf>>>,
    imports: AtomicUsize,
}

impl VolumeCommands for Commands {
    fn choose_epub(&self) -> Result<Option<PathBuf>, BookError> {
        self.reply
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| BookError::ChooserUnavailable)
    }

    fn import_epub(&self, _: &BookWorkspace, _: &Path) -> Result<BookChange, BookError> {
        self.imports.fetch_add(1, Ordering::Relaxed);
        Ok(BookChange {
            generation: GenerationId::parse("22222222222222222222222222222222").unwrap(),
            volume: volume(2),
        })
    }

    fn catalog(&self, workspace: &BookWorkspace) -> Result<BookCatalog, BookError> {
        Ok(BookCatalog {
            generation: workspace.generation.clone(),
            volumes: vec![BookSummary {
                id: volume(1),
                title: "Fixture 1".into(),
            }],
        })
    }

    fn view(
        &self,
        _: &BookWorkspace,
        _: &VolumeId,
        _: BookPosition,
    ) -> Result<BookView, BookError> {
        Err(BookError::NotSaved)
    }

    fn preview(&self, _: &BookWorkspace, _: &VolumeId, _: &BookEdit) -> Result<String, BookError> {
        Err(BookError::InvalidEdit)
    }

    fn save(&self, _: &BookWorkspace, _: &VolumeId, _: &BookEdit) -> Result<BookChange, BookError> {
        Err(BookError::InvalidEdit)
    }
}

fn volume(id: u8) -> VolumeId {
    VolumeId::parse(format!("{id:032x}")).unwrap()
}

fn workspace(id: u8) -> BookWorkspace {
    BookWorkspace {
        path: PathBuf::from(format!("fixture-{id}")),
        series: SeriesId::parse(format!("{id:032x}")).unwrap(),
        generation: GenerationId::parse("g0").unwrap(),
    }
}

fn settle(screen: &mut ImportScreen) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while screen.change_job.is_some() || screen.read_job.is_some() || screen.queued_read.is_some() {
        screen.poll();
        assert!(Instant::now() < deadline, "fixture worker did not finish");
        thread::yield_now();
    }
}

#[test]
fn chooser_cancel_keeps_origin_and_chooser_result_after_switch_cannot_import() {
    let (send, reply) = mpsc::sync_channel(1);
    let commands = Arc::new(Commands {
        reply: Mutex::new(reply),
        imports: AtomicUsize::new(0),
    });
    let mut screen = ImportScreen::new(Some(commands.clone()));
    let first = workspace(1);
    screen.open_workspace(1, first.clone());
    screen.open_picker();
    send.send(None).unwrap();
    settle(&mut screen);
    assert!(screen.picker_focused);
    assert!(screen.notice.contains("hủy"));

    screen.open_picker();
    let second = workspace(2);
    screen.open_workspace(2, second.clone());
    send.send(Some(PathBuf::from("巻 1.epub"))).unwrap();
    settle(&mut screen);
    assert_eq!(commands.imports.load(Ordering::Relaxed), 0);
    let (_, active) = screen.workspace.as_ref().unwrap();
    assert_eq!(active.series, second.series);
    assert_eq!(active.generation.as_str(), "g0");
    assert_ne!(active.series, first.series);
}

#[test]
fn choosing_another_volume_and_returning_restores_the_source_position() {
    let mut screen = ImportScreen::new(None);
    screen.select_volume(volume(1));
    let remembered = BookPosition {
        row: 3,
        range: 2,
        unit: 198,
    };
    screen.position = remembered;
    screen.select_volume(volume(2));
    assert_eq!(screen.position, BookPosition::default());
    screen.select_volume(volume(1));
    assert_eq!(screen.position, remembered);
}

struct InputCommands {
    previews: AtomicUsize,
    saves: AtomicUsize,
    preview_gate: Mutex<Option<mpsc::Receiver<()>>>,
}

fn input_view() -> BookView {
    use nt_application::volume_import::{
        chapter_map::edit::ChapterBoundary, source_range::SourceBlockId,
    };
    BookView {
        version: Some(crate::import_commands::BookVersion {
            epub_sha256: [1; 32],
            parser_version: "input-fixture".into(),
            mapping_version: [2; 32],
        }),
        mode: ScopeMode::Story,
        position: BookPosition::default(),
        rows: 1,
        ranges: 1,
        caption: "Chương thử 日本".into(),
        review_count: 0,
        chapter: Some(7),
        previous_chapter: None,
        next_chapter: None,
        source: Some(crate::import_commands::BookSource {
            start: ChapterBoundary {
                canonical_path: "chapter.xhtml".into(),
                block_id: SourceBlockId::from_bytes([3; 32]),
                offset: 0,
            },
            end: 12,
            text: "日本語abcdefghij".into(),
            classification: None,
        }),
    }
}

impl VolumeCommands for InputCommands {
    fn choose_epub(&self) -> Result<Option<PathBuf>, BookError> {
        Ok(None)
    }
    fn import_epub(&self, _: &BookWorkspace, _: &Path) -> Result<BookChange, BookError> {
        Err(BookError::InvalidEdit)
    }
    fn catalog(&self, workspace: &BookWorkspace) -> Result<BookCatalog, BookError> {
        Ok(BookCatalog {
            generation: workspace.generation.clone(),
            volumes: vec![BookSummary {
                id: volume(1),
                title: "Tập thử".into(),
            }],
        })
    }
    fn view(
        &self,
        _: &BookWorkspace,
        _: &VolumeId,
        _: BookPosition,
    ) -> Result<BookView, BookError> {
        Ok(input_view())
    }
    fn preview(&self, _: &BookWorkspace, _: &VolumeId, _: &BookEdit) -> Result<String, BookError> {
        self.previews.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = self.preview_gate.lock().unwrap().take() {
            gate.recv_timeout(Duration::from_secs(3)).unwrap();
        }
        Ok("Đối chiếu tên chương trước khi lưu".into())
    }
    fn save(&self, _: &BookWorkspace, _: &VolumeId, _: &BookEdit) -> Result<BookChange, BookError> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        Ok(BookChange {
            generation: GenerationId::parse("g1").unwrap(),
            volume: volume(1),
        })
    }
}

fn input_screen(gate: Option<mpsc::Receiver<()>>) -> (ImportScreen, Arc<InputCommands>) {
    let commands = Arc::new(InputCommands {
        previews: AtomicUsize::new(0),
        saves: AtomicUsize::new(0),
        preview_gate: Mutex::new(gate),
    });
    let mut screen = ImportScreen::new(Some(commands.clone()));
    screen.open_workspace(1, workspace(1));
    screen.set_screen(Screen::VolumesAndChapters);
    settle(&mut screen);
    screen.picker_focused = false;
    screen.start_draft('n');
    assert_eq!(screen.input_form().unwrap().text.as_deref(), Some(""));
    (screen, commands)
}

fn input_key(screen: &mut ImportScreen, code: KeyCode, kind: KeyEventKind) {
    assert!(screen.handle_event(&Event::Key(KeyEvent::new_with_kind(
        code,
        KeyModifiers::NONE,
        kind
    ))));
}

#[test]
fn t15_committed_unicode_repeat_and_backspace_only_change_the_focused_field() {
    let (mut screen, commands) = input_screen(None);
    for character in "cirn 日本 a\u{301}".chars() {
        input_key(&mut screen, KeyCode::Char(character), KeyEventKind::Repeat);
    }
    assert_eq!(
        screen.input_form().unwrap().text.as_deref(),
        Some("cirn 日本 a\u{301}")
    );
    input_key(&mut screen, KeyCode::Backspace, KeyEventKind::Repeat);
    input_key(&mut screen, KeyCode::Backspace, KeyEventKind::Repeat);
    assert_eq!(
        screen.input_form().unwrap().text.as_deref(),
        Some("cirn 日本")
    );
    let focus = screen.input_form().unwrap().focused;
    input_key(&mut screen, KeyCode::Tab, KeyEventKind::Repeat);
    input_key(&mut screen, KeyCode::Enter, KeyEventKind::Repeat);
    assert_eq!(screen.input_form().unwrap().focused, focus);
    assert_eq!(commands.previews.load(Ordering::SeqCst), 0);
    assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
    input_key(&mut screen, KeyCode::Tab, KeyEventKind::Press);
    input_key(&mut screen, KeyCode::Char('z'), KeyEventKind::Press);
    assert_eq!(
        screen.input_form().unwrap().text.as_deref(),
        Some("cirn 日本")
    );
    assert_eq!(screen.screen, Screen::VolumesAndChapters);
}

#[test]
fn t15_keyboard_and_mouse_preview_and_save_use_the_same_single_action() {
    for mouse in [false, true] {
        let (mut screen, commands) = input_screen(None);
        input_key(&mut screen, KeyCode::Char('章'), KeyEventKind::Press);
        if mouse {
            assert!(screen.click_input(InputControl::Primary));
        } else {
            input_key(&mut screen, KeyCode::Enter, KeyEventKind::Press);
        }
        input_key(&mut screen, KeyCode::Enter, KeyEventKind::Repeat);
        settle(&mut screen);
        assert_eq!(commands.previews.load(Ordering::SeqCst), 1);
        assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
        input_key(&mut screen, KeyCode::Enter, KeyEventKind::Repeat);
        assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
        if mouse {
            assert!(screen.click_input(InputControl::Primary));
        } else {
            input_key(&mut screen, KeyCode::Enter, KeyEventKind::Press);
        }
        settle(&mut screen);
        assert_eq!(commands.saves.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn t15_pending_preview_can_return_or_discard_without_a_late_save() {
    for discard in [false, true] {
        let (send, receive) = mpsc::sync_channel(1);
        let (mut screen, commands) = input_screen(Some(receive));
        input_key(&mut screen, KeyCode::Char('名'), KeyEventKind::Press);
        input_key(&mut screen, KeyCode::Enter, KeyEventKind::Press);
        assert!(screen.click_input(if discard {
            InputControl::Discard
        } else {
            InputControl::Back
        }));
        assert!(!screen.editing);
        assert_eq!(screen.drafts.is_empty(), discard);
        send.send(()).unwrap();
        settle(&mut screen);
        assert_eq!(commands.previews.load(Ordering::SeqCst), 1);
        assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
        assert_eq!(
            screen.workspace.as_ref().unwrap().1.generation.as_str(),
            "g0"
        );
    }
}

#[test]
fn t15_shift_tab_and_cancel_keep_the_input_draft_and_visible_focus() {
    let (mut screen, commands) = input_screen(None);
    input_key(&mut screen, KeyCode::Char('名'), KeyEventKind::Press);
    assert!(screen.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::BackTab,
        KeyModifiers::SHIFT
    ))));
    assert_eq!(screen.input_form().unwrap().focused, InputControl::Discard);
    input_key(&mut screen, KeyCode::Tab, KeyEventKind::Press);
    assert_eq!(screen.input_form().unwrap().focused, InputControl::Text);
    assert!(screen.click_input(InputControl::Back));
    assert!(!screen.editing);
    screen.start_draft('n');
    assert_eq!(screen.input_form().unwrap().text.as_deref(), Some("名"));
    assert_eq!(commands.previews.load(Ordering::SeqCst), 0);
    assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
}

#[test]
fn t15_reopening_after_back_and_chooser_cancel_returns_to_typing() {
    let (mut screen, _) = input_screen(None);
    input_key(&mut screen, KeyCode::Char('名'), KeyEventKind::Press);
    assert!(screen.click_input(InputControl::Back));
    screen.start_draft('n');
    assert_eq!(screen.input_form().unwrap().focused, InputControl::Text);
    assert!(screen.click_input(InputControl::Back));
    screen.open_picker();
    settle(&mut screen);
    input_key(&mut screen, KeyCode::Char('e'), KeyEventKind::Press);
    assert_eq!(screen.input_form().unwrap().focused, InputControl::Text);
    input_key(&mut screen, KeyCode::Char('a'), KeyEventKind::Press);
    assert_eq!(screen.input_form().unwrap().text.as_deref(), Some("名a"));
}

#[test]
fn t15_stale_edit_feedback_remains_visible_in_the_input_panel() {
    let (mut screen, commands) = input_screen(None);
    screen.workspace.as_mut().unwrap().1.generation =
        GenerationId::parse("33333333333333333333333333333333").unwrap();
    input_key(&mut screen, KeyCode::Char('x'), KeyEventKind::Press);
    assert!(
        screen
            .input_form()
            .unwrap()
            .message
            .contains(&BookError::Stale.message())
    );
    assert_eq!(screen.input_form().unwrap().text.as_deref(), Some(""));
    assert_eq!(commands.previews.load(Ordering::SeqCst), 0);
    assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
}

#[test]
fn t15_altgr_unicode_is_text_in_the_focused_field_without_running_a_command() {
    let (mut screen, commands) = input_screen(None);
    for modifiers in [
        KeyModifiers::CONTROL | KeyModifiers::ALT,
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    ] {
        let key = KeyEvent::new(KeyCode::Char('©'), modifiers);
        assert!(!crate::input::is_shutdown_request(&Event::Key(key)));
        assert!(screen.handle_event(&Event::Key(key)));
    }
    assert_eq!(screen.input_form().unwrap().text.as_deref(), Some("©©"));
    assert_eq!(commands.previews.load(Ordering::SeqCst), 0);
    assert_eq!(commands.saves.load(Ordering::SeqCst), 0);
}
