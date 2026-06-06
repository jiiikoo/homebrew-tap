//! A file-browser modal for picking a key file anywhere on disk (so users don't have to type
//! a path). Type to fuzzy-filter the listing; ↑/↓ move; Enter opens a directory or selects a
//! file; ← goes up; Backspace edits the filter (or goes up when empty); Esc clears the filter
//! (or cancels when empty).

use std::path::PathBuf;

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use super::{accent, centered, highlight};
use crate::search;

pub enum BrowseOutcome {
    Continue,
    Cancel,
    Pick(PathBuf),
}

struct Entry {
    label: String,
    path: PathBuf,
    is_dir: bool,
}

pub struct FileBrowser {
    cwd: PathBuf,
    entries: Vec<Entry>,
    /// Fuzzy filter over entry labels.
    query: String,
    /// Selection index into the *visible* (filtered) entries.
    selected: usize,
}

impl FileBrowser {
    pub fn new(start: PathBuf) -> Self {
        let mut b = FileBrowser {
            cwd: start,
            entries: Vec::new(),
            query: String::new(),
            selected: 0,
        };
        b.reload();
        b
    }

    fn reload(&mut self) {
        let mut entries = Vec::new();
        if let Some(parent) = self.cwd.parent() {
            entries.push(Entry {
                label: "../".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            });
        }
        if let Ok(rd) = std::fs::read_dir(&self.cwd) {
            let mut items: Vec<Entry> = rd
                .flatten()
                .map(|e| {
                    let path = e.path();
                    let is_dir = path.is_dir();
                    let name = e.file_name().to_string_lossy().into_owned();
                    let label = if is_dir { format!("{name}/") } else { name };
                    Entry {
                        label,
                        path,
                        is_dir,
                    }
                })
                .collect();
            // Directories first, then files; each alphabetical (case-insensitive).
            items.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            });
            entries.extend(items);
        }
        self.entries = entries;
        self.query.clear();
        self.selected = 0;
    }

    /// Indices into `entries` that match the current filter, best-first.
    fn visible(&self) -> Vec<usize> {
        let labels: Vec<String> = self.entries.iter().map(|e| e.label.clone()).collect();
        search::fuzzy_filter(&labels, &self.query)
    }

    fn move_sel(&mut self, delta: isize, visible_len: usize) {
        if visible_len == 0 {
            self.selected = 0;
            return;
        }
        let n = visible_len as isize;
        self.selected = (self.selected as isize + delta).clamp(0, n - 1) as usize;
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.reload();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> BrowseOutcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let visible = self.visible();
        match (key.code, ctrl) {
            (KeyCode::Esc, _) => {
                if self.query.is_empty() {
                    return BrowseOutcome::Cancel;
                }
                self.query.clear();
                self.selected = 0;
            }
            (KeyCode::Down, false) | (KeyCode::Char('n'), true) => self.move_sel(1, visible.len()),
            (KeyCode::Up, false) | (KeyCode::Char('p'), true) => self.move_sel(-1, visible.len()),
            (KeyCode::Enter, _) | (KeyCode::Right, false) => {
                if let Some(&ei) = visible.get(self.selected) {
                    let entry = &self.entries[ei];
                    if entry.is_dir {
                        self.cwd = entry.path.clone();
                        self.reload();
                    } else {
                        return BrowseOutcome::Pick(entry.path.clone());
                    }
                }
            }
            (KeyCode::Left, false) => self.go_up(),
            (KeyCode::Backspace, _) => {
                if self.query.is_empty() {
                    self.go_up();
                } else {
                    self.query.pop();
                    self.selected = 0;
                }
            }
            (KeyCode::Char(c), false) => {
                self.query.push(c);
                self.selected = 0;
            }
            _ => {}
        }
        BrowseOutcome::Continue
    }
}

pub fn render(frame: &mut Frame, b: &FileBrowser) {
    let fa = frame.area();
    let width = fa.width.saturating_sub(6).clamp(50, 90);
    let height = fa.height.saturating_sub(4).clamp(8, 24);
    let area = centered(fa, width, height);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" pick a key file ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1), // cwd
        Constraint::Length(1), // filter
        Constraint::Min(0),    // list
        Constraint::Length(1), // hint
    ])
    .split(inner);

    // Current directory (truncated from the left so the tail stays visible).
    let cwd = b.cwd.to_string_lossy();
    let w = rows[0].width as usize;
    let shown = if cwd.chars().count() > w {
        let tail: String = cwd.chars().rev().take(w.saturating_sub(1)).collect();
        format!("…{}", tail.chars().rev().collect::<String>())
    } else {
        cwd.into_owned()
    };
    frame.render_widget(
        Paragraph::new(shown).style(Style::default().fg(accent())),
        rows[0],
    );

    // Filter line.
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(accent())),
            Span::raw(b.query.as_str()),
        ])),
        rows[1],
    );

    let visible = b.visible();
    let sel = b.selected.min(visible.len().saturating_sub(1));
    let mut matcher = search::matcher();
    let hl = Style::default().fg(accent()).add_modifier(Modifier::BOLD);
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&ei| {
            let e = &b.entries[ei];
            let base = if e.is_dir {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let idx = search::match_indices(&e.label, &b.query, &mut matcher);
            ListItem::new(Line::from(highlight(&e.label, &idx, base, hl)))
        })
        .collect();
    let list = List::new(items)
        .highlight_symbol("▸ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(sel));
    }
    frame.render_stateful_widget(list, rows[2], &mut state);

    frame.render_widget(
        Paragraph::new("type filter · ↑↓ move · ↵ open/select · ← up · esc clear/cancel")
            .style(Style::default().fg(Color::DarkGray)),
        rows[3],
    );

    // Cursor after the filter text.
    let cx = rows[1].x + 2 + b.query.chars().count() as u16;
    frame.set_cursor_position((
        cx.min(rows[1].x + rows[1].width.saturating_sub(1)),
        rows[1].y,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn scratch() -> PathBuf {
        let root = std::env::temp_dir().join(format!("sshelf-browse-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("key.pem"), b"-----BEGIN RSA PRIVATE KEY-----\n").unwrap();
        std::fs::write(root.join("notes.txt"), b"hi").unwrap();
        root
    }

    #[test]
    fn lists_dirs_first_then_files_with_parent() {
        let b = FileBrowser::new(scratch());
        assert_eq!(b.entries[0].label, "../");
        let sub = b.entries.iter().position(|e| e.label == "sub/").unwrap();
        let key = b.entries.iter().position(|e| e.label == "key.pem").unwrap();
        assert!(sub < key);
    }

    #[test]
    fn typing_filters_entries() {
        let mut b = FileBrowser::new(scratch());
        for c in "pem".chars() {
            b.handle_key(k(KeyCode::Char(c)));
        }
        let visible = b.visible();
        let labels: Vec<&str> = visible
            .iter()
            .map(|&i| b.entries[i].label.as_str())
            .collect();
        assert!(labels.contains(&"key.pem"));
        assert!(!labels.contains(&"notes.txt"));
    }

    #[test]
    fn enter_on_filtered_file_picks_it() {
        let root = scratch();
        let mut b = FileBrowser::new(root.clone());
        for c in "pem".chars() {
            b.handle_key(k(KeyCode::Char(c)));
        }
        match b.handle_key(k(KeyCode::Enter)) {
            BrowseOutcome::Pick(p) => assert_eq!(p, root.join("key.pem")),
            _ => panic!("expected pick"),
        }
    }

    #[test]
    fn backspace_edits_filter_then_goes_up() {
        let root = scratch();
        let mut b = FileBrowser::new(root.clone());
        b.handle_key(k(KeyCode::Char('x')));
        assert_eq!(b.query, "x");
        b.handle_key(k(KeyCode::Backspace)); // clears the 'x'
        assert_eq!(b.query, "");
        b.handle_key(k(KeyCode::Backspace)); // now goes up
        assert_eq!(b.cwd, root.parent().unwrap());
    }

    #[test]
    fn esc_clears_filter_then_cancels() {
        let mut b = FileBrowser::new(scratch());
        b.handle_key(k(KeyCode::Char('a')));
        assert!(matches!(
            b.handle_key(k(KeyCode::Esc)),
            BrowseOutcome::Continue
        ));
        assert_eq!(b.query, "");
        assert!(matches!(
            b.handle_key(k(KeyCode::Esc)),
            BrowseOutcome::Cancel
        ));
    }

    #[test]
    fn enter_on_dir_descends_clears_filter_and_left_goes_up() {
        let root = scratch();
        let mut b = FileBrowser::new(root.clone());
        let idx = b
            .visible()
            .iter()
            .position(|&i| b.entries[i].label == "sub/")
            .unwrap();
        b.selected = idx;
        b.handle_key(k(KeyCode::Enter));
        assert_eq!(b.cwd, root.join("sub"));
        assert_eq!(b.query, "");
        b.handle_key(k(KeyCode::Left));
        assert_eq!(b.cwd, root);
    }

    #[test]
    fn renders_and_writes_snapshot() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let b = FileBrowser::new(scratch());
        let mut term = Terminal::new(TestBackend::new(70, 16)).unwrap();
        term.draw(|f| render(f, &b)).unwrap();
        let buf = term.backend().buffer();
        let width = buf.area.width as usize;
        let snapshot: String = buf
            .content()
            .chunks(width)
            .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(snapshot.contains("pick a key file"));
        assert!(snapshot.contains("key.pem"));

        if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let p = std::path::Path::new(&dir).join("target/browse-snapshot.txt");
            let _ = std::fs::write(p, &snapshot);
        }
    }
}
