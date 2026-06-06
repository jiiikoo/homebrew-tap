//! The settings screen (F2): configure sshelf itself. Starts with the hosts-file location;
//! designed to grow (more fields can be added to the form). The config-file path is shown
//! read-only because it's chosen *before* the config is read (via `--config` / `$SSHELF_CONFIG`).

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::widgets::TextField;
use super::{accent, centered};

const VALUE_COL: u16 = 15;
const LABEL_W: usize = 12;

pub enum SettingsOutcome {
    Continue,
    Cancel,
    /// Save preferences; `hosts_file` is `None` to use the default location.
    Save {
        hosts_file: Option<String>,
    },
}

pub struct Settings {
    /// Active config-file path (display only).
    config_path: String,
    /// Default hosts path, shown as a placeholder when the field is blank.
    default_hosts: String,
    hosts_file: TextField,
}

impl Settings {
    pub fn new(config_path: String, hosts_file: Option<String>, default_hosts: String) -> Self {
        Settings {
            config_path,
            default_hosts,
            hosts_file: TextField::with(hosts_file.unwrap_or_default()),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SettingsOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s')) {
            return self.save();
        }
        match key.code {
            KeyCode::Esc => SettingsOutcome::Cancel,
            KeyCode::Enter => self.save(),
            code => {
                self.hosts_file.handle(code);
                SettingsOutcome::Continue
            }
        }
    }

    fn save(&self) -> SettingsOutcome {
        let v = self.hosts_file.value.trim();
        SettingsOutcome::Save {
            hosts_file: (!v.is_empty()).then(|| v.to_string()),
        }
    }
}

/// Truncate `s` from the left to fit `width`, prefixing `…` when shortened.
fn fit_left(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let tail: String = s.chars().rev().take(width.saturating_sub(1)).collect();
    format!("…{}", tail.chars().rev().collect::<String>())
}

pub fn render(frame: &mut Frame, s: &Settings) {
    let width = frame.area().width.saturating_sub(6).clamp(56, 100);
    let area = centered(frame.area(), width, 11);
    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title(" settings ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let acc = Style::default().fg(accent()).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let row = |i: u16| Rect {
        x: inner.x,
        y: inner.y + i,
        width: inner.width,
        height: 1,
    };
    let val_w = inner.width.saturating_sub(VALUE_COL) as usize;

    // Config file (read-only info).
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::raw(format!("{:<LABEL_W$} ", "Config file")),
            Span::styled(fit_left(&s.config_path, val_w), dim),
        ])),
        row(0),
    );
    frame.render_widget(
        Paragraph::new("    (read-only — set via --config or $SSHELF_CONFIG)").style(dim),
        row(1),
    );

    // Hosts file (the editable field).
    let (hosts_val, hosts_style) = if s.hosts_file.value.is_empty() {
        (
            format!(
                "default · {}",
                fit_left(&s.default_hosts, val_w.saturating_sub(10))
            ),
            dim,
        )
    } else {
        (s.hosts_file.value.clone(), Style::default())
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("▸ ", acc),
            Span::styled(format!("{:<LABEL_W$} ", "Hosts file"), acc),
            Span::styled(hosts_val, hosts_style),
        ])),
        row(3),
    );

    frame.render_widget(
        Paragraph::new("↵ or ^s save · esc cancel").style(dim),
        row(inner.height.saturating_sub(2)),
    );
    frame.render_widget(
        Paragraph::new("more settings coming soon").style(dim),
        row(inner.height.saturating_sub(1)),
    );

    // Cursor on the Hosts file field.
    let cx = inner.x + VALUE_COL + s.hosts_file.cursor as u16;
    frame.set_cursor_position((cx.min(inner.x + inner.width.saturating_sub(1)), inner.y + 3));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn new_settings() -> Settings {
        Settings::new(
            "/home/u/.config/sshelf/config.toml".into(),
            None,
            "/home/u/.config/sshelf/hosts.toml".into(),
        )
    }

    #[test]
    fn empty_saves_none() {
        let mut s = new_settings();
        match s.handle_key(k(KeyCode::Enter)) {
            SettingsOutcome::Save { hosts_file } => assert!(hosts_file.is_none()),
            _ => panic!("expected save"),
        }
    }

    #[test]
    fn typed_path_saves_some() {
        let mut s = new_settings();
        for c in "/data/hosts.toml".chars() {
            s.handle_key(k(KeyCode::Char(c)));
        }
        match s.handle_key(ctrl(KeyCode::Char('s'))) {
            SettingsOutcome::Save { hosts_file } => {
                assert_eq!(hosts_file.as_deref(), Some("/data/hosts.toml"));
            }
            _ => panic!("expected save"),
        }
    }

    #[test]
    fn esc_cancels() {
        let mut s = new_settings();
        assert!(matches!(
            s.handle_key(k(KeyCode::Esc)),
            SettingsOutcome::Cancel
        ));
    }

    #[test]
    fn renders_and_writes_snapshot() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let s = new_settings();
        let mut term = Terminal::new(TestBackend::new(72, 14)).unwrap();
        term.draw(|f| render(f, &s)).unwrap();
        let buf = term.backend().buffer();
        let width = buf.area.width as usize;
        let snapshot: String = buf
            .content()
            .chunks(width)
            .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(snapshot.contains("settings"));
        assert!(snapshot.contains("Hosts file"));
        assert!(snapshot.contains("Config file"));
        if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let p = std::path::Path::new(&dir).join("target/settings-snapshot.txt");
            let _ = std::fs::write(p, &snapshot);
        }
    }
}
