// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Interactive `tcl explore --tui` shell (feature `tui`).
//!
//! A thin presentation layer over the shared pipeline + serialiser + the
//! `tcl-explorer` `view_tree` model: a sidebar of views, an expandable
//! tree of the selected view, and a detail panel for the highlighted node
//! — the same `ViewNode` forest the GUI renders. Keys: ↑/↓ move the tree
//! cursor, ←/→ (or Tab) switch view, Enter/Space expand/collapse, q quit.
//!
//! The navigation/flatten logic ([`App`]) is pure and unit-tested; only
//! [`run`] touches the terminal.

use std::collections::HashSet;
use std::io::Write as _;

use ratatui::Terminal;
use ratatui::backend::TerminaBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::termina::escape::csi::{self, Csi};
use ratatui::termina::event::{KeyCode, KeyEventKind};
use ratatui::termina::{Event, EventReader, PlatformTerminal, Terminal as _};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde_json::Value;

/// The concrete terminal type: a Ratatui terminal over the Termina backend.
type AppTerminal = Terminal<TerminaBackend<PlatformTerminal>>;

/// Build a private-mode CSI for the named [`csi::DecPrivateModeCode`].
macro_rules! dec_set {
    ($mode:ident) => {
        Csi::Mode(csi::Mode::SetDecPrivateMode(csi::DecPrivateMode::Code(
            csi::DecPrivateModeCode::$mode,
        )))
    };
}
macro_rules! dec_reset {
    ($mode:ident) => {
        Csi::Mode(csi::Mode::ResetDecPrivateMode(csi::DecPrivateMode::Code(
            csi::DecPrivateModeCode::$mode,
        )))
    };
}

use tcl_explorer::view_tree::TREE_VIEWS;
use tcl_explorer::{ViewNode, build_view};

/// One visible (un-collapsed) tree row.
struct VisibleRow {
    path: Vec<usize>,
    depth: usize,
    label: String,
    has_children: bool,
}

/// Explorer TUI state — pure; no terminal I/O.
struct App {
    data: Value,
    view_idx: usize,
    roots: Vec<ViewNode>,
    expanded: HashSet<Vec<usize>>,
    cursor: usize,
}

impl App {
    fn new(data: Value) -> Self {
        // Open on `ir` — the IR is the most useful entry point into the views.
        let view_idx = TREE_VIEWS.iter().position(|v| *v == "ir").unwrap_or(0);
        let roots = build_view(TREE_VIEWS[view_idx], &data);
        Self {
            data,
            view_idx,
            roots,
            expanded: HashSet::new(),
            cursor: 0,
        }
    }

    fn current_view(&self) -> &'static str {
        TREE_VIEWS[self.view_idx]
    }

    /// Switch to view `idx` (wrapping), rebuilding the forest and resetting
    /// the cursor/expansion.
    fn set_view(&mut self, idx: usize) {
        self.view_idx = idx % TREE_VIEWS.len();
        self.roots = build_view(self.current_view(), &self.data);
        self.expanded.clear();
        self.cursor = 0;
    }

    fn next_view(&mut self) {
        self.set_view(self.view_idx + 1);
    }

    fn prev_view(&mut self) {
        self.set_view((self.view_idx + TREE_VIEWS.len() - 1) % TREE_VIEWS.len());
    }

    /// The currently visible rows, honouring the expanded set.
    fn visible(&self) -> Vec<VisibleRow> {
        let mut out = Vec::new();
        for (i, node) in self.roots.iter().enumerate() {
            self.walk(node, &[i], 0, &mut out);
        }
        out
    }

    fn walk(&self, node: &ViewNode, path: &[usize], depth: usize, out: &mut Vec<VisibleRow>) {
        let has_children = !node.children.is_empty();
        out.push(VisibleRow {
            path: path.to_vec(),
            depth,
            label: node.label.clone(),
            has_children,
        });
        if has_children && self.expanded.contains(path) {
            for (j, child) in node.children.iter().enumerate() {
                let mut cp = path.to_vec();
                cp.push(j);
                self.walk(child, &cp, depth + 1, out);
            }
        }
    }

    fn move_cursor(&mut self, delta: i64) {
        let n = self.visible().len();
        if n == 0 {
            self.cursor = 0;
            return;
        }
        let max = i64::try_from(n - 1).unwrap_or(0);
        let cur = i64::try_from(self.cursor.min(n - 1)).unwrap_or(0);
        self.cursor = usize::try_from((cur + delta).clamp(0, max)).unwrap_or(0);
    }

    /// Toggle expand/collapse on the cursor row (no-op for leaves).
    fn toggle(&mut self) {
        let rows = self.visible();
        let Some(row) = rows.get(self.cursor) else {
            return;
        };
        if !row.has_children {
            return;
        }
        if !self.expanded.remove(&row.path) {
            self.expanded.insert(row.path.clone());
        }
    }

    /// Find a node by its path.
    fn node_at<'a>(roots: &'a [ViewNode], path: &[usize]) -> Option<&'a ViewNode> {
        let (&first, rest) = path.split_first()?;
        let mut node = roots.get(first)?;
        for &idx in rest {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    /// The detail rows of the highlighted node.
    fn current_detail(&self) -> Vec<(String, String)> {
        let rows = self.visible();
        rows.get(self.cursor)
            .and_then(|r| Self::node_at(&self.roots, &r.path))
            .map(|n| n.detail.clone())
            .unwrap_or_default()
    }
}

/// Run the interactive explorer over `source`/`dialect`.
pub fn run(source: &str, dialect: &str) -> anyhow::Result<()> {
    let data = tcl_explorer::serialise_result(&tcl_explorer::run_pipeline(source, dialect));
    let mut app = App::new(data);

    let (mut terminal, events) = init_terminal()?;
    let result = event_loop(&mut terminal, &events, &mut app);
    // Restore the terminal even if the loop errored; the raw-mode reset happens
    // when the backend's `PlatformTerminal` drops.
    let restored = restore_terminal(&mut terminal);
    result.and(restored)
}

/// Enter raw mode + the alternate screen and build the Ratatui/Termina
/// terminal, returning it alongside the event reader.
fn init_terminal() -> anyhow::Result<(AppTerminal, EventReader)> {
    let mut output = PlatformTerminal::new()?;
    output.enter_raw_mode()?;
    // Alternate screen, hidden cursor — a full-screen navigation UI.
    write!(
        output,
        "{}{}",
        dec_set!(ClearAndEnableAlternateScreen),
        dec_reset!(ShowCursor)
    )?;
    output.flush()?;

    let events = output.event_reader();
    let terminal = Terminal::new(TerminaBackend::new(output))?;
    Ok((terminal, events))
}

/// Leave the alternate screen and restore the cursor (raw mode is reset on drop).
fn restore_terminal(terminal: &mut AppTerminal) -> anyhow::Result<()> {
    let backend = terminal.backend_mut();
    write!(
        backend,
        "{}{}",
        dec_reset!(ClearAndEnableAlternateScreen),
        dec_set!(ShowCursor)
    )?;
    backend.flush()?;
    Ok(())
}

fn event_loop(
    terminal: &mut AppTerminal,
    events: &EventReader,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        // Block until a key press (so shortcuts don't double-fire on release)
        // or a resize (which the next draw picks up).
        let event = events.read(|e| {
            matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press)
                || matches!(e, Event::WindowResized(_))
        })?;
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') | KeyCode::Escape => return Ok(()),
                KeyCode::Up => app.move_cursor(-1),
                KeyCode::Down => app.move_cursor(1),
                KeyCode::Left => app.prev_view(),
                KeyCode::Right | KeyCode::Tab => app.next_view(),
                KeyCode::Enter | KeyCode::Char(' ') => app.toggle(),
                _ => {}
            }
        }
    }
}

fn draw(frame: &mut ratatui::Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(20)])
        .split(outer[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(cols[1]);

    // Sidebar: view list.
    let views: Vec<ListItem> = TREE_VIEWS.iter().map(|v| ListItem::new(*v)).collect();
    let mut vstate = ListState::default();
    vstate.select(Some(app.view_idx));
    frame.render_stateful_widget(
        List::new(views)
            .block(Block::default().borders(Borders::ALL).title("views"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        cols[0],
        &mut vstate,
    );

    // Tree: visible rows, indented, with expand markers.
    let rows = app.visible();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            let marker = if r.has_children {
                if app.expanded.contains(&r.path) {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };
            let indent = "  ".repeat(r.depth);
            ListItem::new(Line::from(vec![Span::raw(format!(
                "{indent}{marker}{}",
                r.label
            ))]))
        })
        .collect();
    let mut tstate = ListState::default();
    tstate.select(Some(app.cursor.min(rows.len().saturating_sub(1))));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{} ", app.current_view())),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        right[0],
        &mut tstate,
    );

    // Detail panel.
    let detail = app.current_detail();
    let lines: Vec<Line> = detail
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(
                    format!("{k:<16}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(v.clone()),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("detail")),
        right[1],
    );

    frame.render_widget(
        Paragraph::new("↑/↓ move · ←/→ view · Enter/Space expand · q quit"),
        outer[1],
    );
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn app(src: &str) -> App {
        let data = tcl_explorer::serialise_result(&tcl_explorer::run_pipeline(src, "tcl8.6"));
        App::new(data)
    }

    /// `draw` is backend-agnostic, so render it into a [`TestBackend`] buffer
    /// (no terminal needed) and assert the chrome is present — a smoke test that
    /// the Ratatui/Termina migration didn't break the layout.
    #[test]
    fn draw_renders_chrome_into_test_backend() {
        let mut app = app("proc add {a b} { expr {$a + $b} }\nputs [add 1 2]");
        app.toggle(); // expand the IR root so the tree shows statements
        let mut terminal = Terminal::new(TestBackend::new(78, 20)).expect("test terminal");
        terminal.draw(|frame| draw(frame, &app)).expect("draw");
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("views"), "sidebar title: {text}");
        assert!(text.contains("detail"), "detail panel: {text}");
        assert!(text.contains("expand"), "footer hint: {text}");
    }

    #[test]
    fn opens_on_ir_with_rows() {
        let a = app("set x 1");
        assert_eq!(a.current_view(), "ir");
        assert!(!a.visible().is_empty());
    }

    #[test]
    fn switching_views_wraps_and_rebuilds() {
        // Start from view 0 so the wrap-around arithmetic is unambiguous
        // regardless of which view `App::new` opens on.
        let mut a = app("set x 1");
        a.set_view(0);
        a.prev_view();
        assert_eq!(a.view_idx, TREE_VIEWS.len() - 1);
        a.next_view();
        assert_eq!(a.view_idx, 0);
    }

    #[test]
    fn expand_reveals_children() {
        // The IR view's top-level node has children (the statements).
        let mut a = app("set x 1\nset y 2");
        a.set_view(TREE_VIEWS.iter().position(|v| *v == "ir").unwrap());
        let before = a.visible().len();
        a.cursor = 0;
        a.toggle();
        assert!(
            a.visible().len() > before,
            "expanding should reveal children"
        );
        a.toggle();
        assert_eq!(
            a.visible().len(),
            before,
            "collapsing restores the row count"
        );
    }

    #[test]
    fn cursor_clamps_to_visible_rows() {
        let mut a = app("set x 1");
        a.move_cursor(1000);
        assert!(a.cursor < a.visible().len());
        a.move_cursor(-1000);
        assert_eq!(a.cursor, 0);
    }

    #[test]
    fn detail_reflects_cursor_node() {
        // In the IR view, expand the top-level node and move to a statement
        // child, whose detail table carries a "summary" row.
        let mut a = app("set x 1\nset y 2");
        a.cursor = 0;
        a.toggle();
        a.move_cursor(1);
        let detail = a.current_detail();
        assert!(detail.iter().any(|(k, _)| k == "summary"));
    }
}
