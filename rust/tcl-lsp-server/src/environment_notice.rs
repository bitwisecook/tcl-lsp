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

//! The one-time explanation of what a tool environment is.
//!
//! The six EDA shells (Vivado, Quartus, Questa, Libero, Synopsys, Cadence) are
//! environments of kind [`EnvironmentKind::Packages`] shipped in a bundled
//! pack: a stock Tcl release with library packages loaded, not a separate
//! dialect. The first time a document resolves to one, the server tells the
//! user so, once per environment per session, and never again once the user
//! has asked not to be told.
//!
//! The server owns the message because it owns the environment catalogue, and
//! a server-sent message renders in every editor with no client code:
//! `window/showMessageRequest` with two actions when the client advertises
//! `window.showMessage.messageActionItem`, plain `window/showMessage`
//! otherwise. *Learn more* opens the KCS note (`window/showDocument`, or the
//! URL on the log channel when the client cannot show a document); *Don't
//! show again* is recorded in `notices.ini` under the per-user state directory
//! ([`tcl_lsp_core::tcl_install::user_notices_path`]), so the choice follows
//! the user across editors.
//!
//! This is the one scoped exception to the silence `config-precedence.md`
//! keeps about ignored settings: it explains a classification rather than
//! reporting a setting that was overridden. `tclLsp.notifications.environmentKind`
//! (`[notifications] environment_kind` in `config.ini`) turns it off.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tcl_dialect::model::{EnvironmentDefinition, EnvironmentKind, Provenance};
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageActionItem, MessageType, ShowDocumentParams, Uri};

use crate::config_ini::{Section, parse_ini};
use crate::vfs::SourceStore;

/// The KCS note *Learn more* opens.
pub(crate) const LEARN_MORE_URL: &str = "https://github.com/bitwisecook/tcl-lsp/blob/rust/docs/kcs/features/kcs-feature-tool-environments.md";

/// The action that opens [`LEARN_MORE_URL`].
pub(crate) const LEARN_MORE: &str = "Learn more";

/// The action that records the dismissal.
pub(crate) const DONT_SHOW_AGAIN: &str = "Don't show again";

/// The `notices.ini` section that lists what the user has dismissed.
const DISMISSED_SECTION: &str = "dismissed";

/// The key in [`DISMISSED_SECTION`] that lists dismissed environment ids.
const DISMISSED_KEY: &str = "environment-kind";

/// How long a notice waits for the first configuration pull before it decides
/// whether the setting switches it off.
///
/// A document restored at start-up opens concurrently with `initialized`, whose
/// pull carries `notifications.environmentKind`; deciding before it lands would
/// show the notice to a user who has turned it off. The bound only matters for
/// a client that never answers the pull, and after it the notice decides on
/// the built-in default.
const CONFIG_SETTLE_WAIT: Duration = Duration::from_secs(30);

/// Whether a document resolved to `definition` earns the notice: a tool shell
/// (kind `Packages`) that ships in a bundled pack. `tk` is `Packages` but
/// built in, and a user- or workspace-pack environment is the user's own, so
/// neither has anything to be told.
pub(crate) fn qualifies(definition: &EnvironmentDefinition) -> bool {
    definition.kind == EnvironmentKind::Packages && definition.provenance == Provenance::BundledPack
}

/// The notice text for `definition`, built from the environment so the release
/// and the package list are the catalogue's, never copied:
///
/// `Xilinx Vivado (xilinx-eda-tcl) is Tcl 8.5 plus the vivado, sdc and upf
/// packages. Tool support is a set of library packages on a Tcl release, not a
/// separate dialect; your selection keeps working as before.`
///
/// The release and the ambient packages are
/// [`EnvironmentDefinition::core_label`] and
/// [`EnvironmentDefinition::ambient_packages`], the same two the description
/// every picker shows is made of.
pub(crate) fn notice_text(definition: &EnvironmentDefinition) -> String {
    let release = definition.core_label().unwrap_or_else(|| "Tcl".to_owned());
    let packages: Vec<&str> = definition.ambient_packages().collect();
    let plus = match packages.as_slice() {
        [] => String::new(),
        [only] => format!(" plus the {only} package"),
        [init @ .., last] => format!(" plus the {} and {last} packages", init.join(", ")),
    };
    format!(
        "{} ({}) is {release}{plus}. Tool support is a set of library packages on a Tcl \
         release, not a separate dialect; your selection keeps working as before.",
        definition.display_name,
        definition.id.as_str(),
    )
}

/// The environment ids `[dismissed] environment-kind` lists, in file order.
///
/// Ids are separated by commas and/or whitespace. A file with no such section
/// or key dismisses nothing.
fn dismissed_ids(sections: &[Section]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    let listed = sections
        .iter()
        .filter(|section| section.name == DISMISSED_SECTION)
        .flat_map(|section| &section.entries)
        .filter(|(key, _)| key == DISMISSED_KEY)
        .flat_map(|(_, value)| value.split(|c: char| c == ',' || c.is_whitespace()))
        .filter(|id| !id.is_empty());
    for id in listed {
        if !ids.iter().any(|known| known == id) {
            ids.push(id.to_owned());
        }
    }
    ids
}

/// `notices.ini` text that lists `ids` under `[dismissed] environment-kind`,
/// carrying every other section and key of `sections` through unchanged.
///
/// Other keys survive because the file is shared by every editor's server, and
/// a newer server may record notices an older one does not know.
fn render_notices(mut sections: Vec<Section>, ids: &[String]) -> String {
    let list = ids.join(", ");
    match sections
        .iter_mut()
        .find(|section| section.name == DISMISSED_SECTION)
    {
        Some(section) => {
            // Every `environment-kind` entry collapses into the first, which
            // holds the whole list.
            section.entries.retain(|(key, _)| key != DISMISSED_KEY);
            section.entries.insert(0, (DISMISSED_KEY.to_owned(), list));
        }
        None => sections.insert(
            0,
            Section {
                name: DISMISSED_SECTION.to_owned(),
                entries: vec![(DISMISSED_KEY.to_owned(), list)],
            },
        ),
    }
    let mut out = String::from(
        "# Written by tcl-lsp: notices you asked not to be shown again.\n\
         # Delete a line, or the file, to be told again.\n",
    );
    for section in &sections {
        out.push_str("\n[");
        out.push_str(&section.name);
        out.push_str("]\n");
        for (key, value) in &section.entries {
            // A multi-line value continues on indented lines.
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(&value.replace('\n', "\n    "));
            out.push('\n');
        }
    }
    out
}

/// Ids the server has dismissed or shown, guarded by one lock so a claim reads
/// and writes both together.
#[derive(Debug)]
struct Seen {
    /// Dismissed by the user, in the order they were dismissed.
    dismissed: Vec<String>,
    /// Shown this session, whatever the reply.
    shown: Vec<String>,
}

/// The notice's session state: what has been dismissed or shown, the setting
/// that switches it off, and what the connected client can render.
///
/// Held in an `Arc` by the backend so the task that presents the notice owns
/// its state and never borrows the backend across an await on the client.
#[derive(Debug)]
pub(crate) struct EnvironmentNotice {
    /// Where dismissals are persisted; `None` when the platform has no home
    /// for it, in which case a dismissal lasts for the session only.
    path: Option<PathBuf>,
    /// `tclLsp.notifications.environmentKind`.
    enabled: AtomicBool,
    /// The client advertised `window.showMessage.messageActionItem`.
    message_action_items: AtomicBool,
    /// The client advertised `window.showDocument.support`.
    show_document: AtomicBool,
    /// Flips to `true` once the first configuration pull has been applied.
    config_settled: tokio::sync::watch::Sender<bool>,
    /// A problem reading `notices.ini` at start-up, reported once.
    load_warning: Mutex<Option<String>>,
    seen: Mutex<Seen>,
}

impl EnvironmentNotice {
    /// State that persists dismissals at `path`, starting from `dismissed`.
    fn new(path: Option<PathBuf>, dismissed: Vec<String>) -> Self {
        Self {
            path,
            enabled: AtomicBool::new(true),
            message_action_items: AtomicBool::new(false),
            show_document: AtomicBool::new(false),
            config_settled: tokio::sync::watch::channel(false).0,
            load_warning: Mutex::new(None),
            seen: Mutex::new(Seen {
                dismissed,
                shown: Vec::new(),
            }),
        }
    }

    /// State with nowhere to persist: dismissals last for the session.
    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self::new(None, Vec::new())
    }

    /// Read the dismissals from `path` once, at start-up.
    ///
    /// A missing file dismisses nothing. A file that cannot be read (bad
    /// permissions, not UTF-8) also dismisses nothing and leaves one warning
    /// for [`Self::take_load_warning`]; the file is never an error the session
    /// depends on.
    pub(crate) fn load(store: &dyn SourceStore, path: Option<PathBuf>) -> Self {
        let Some(file) = path.clone() else {
            return Self::new(None, Vec::new());
        };
        match store.read_to_string(&file) {
            Ok(content) => Self::new(path, dismissed_ids(&parse_ini(&content))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::new(path, Vec::new())
            }
            Err(error) => {
                let notice = Self::new(path, Vec::new());
                *notice
                    .load_warning
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(format!(
                    "tcl-lsp: could not read {}: {error}; notices you dismissed may be shown again",
                    file.display()
                ));
                notice
            }
        }
    }

    /// The warning [`Self::load`] left, handed out once.
    pub(crate) fn take_load_warning(&self) -> Option<String> {
        self.load_warning
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Record what the client can render, from its `initialize` capabilities.
    pub(crate) fn set_client_capabilities(&self, message_action_items: bool, show_document: bool) {
        self.message_action_items
            .store(message_action_items, Ordering::Relaxed);
        self.show_document.store(show_document, Ordering::Relaxed);
    }

    /// Apply `tclLsp.notifications.environmentKind`.
    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Whether `tclLsp.notifications.environmentKind` allows the notice.
    pub(crate) fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Release notices that were waiting on the first configuration pull.
    pub(crate) fn mark_config_settled(&self) {
        self.config_settled.send_replace(true);
    }

    /// Whether a document resolved to `definition` still needs the notice
    /// considered: it qualifies, and neither the user nor this session has
    /// dealt with it. The cheap check the open path makes before spawning.
    pub(crate) fn wants(&self, definition: &EnvironmentDefinition) -> bool {
        if !qualifies(definition) {
            return false;
        }
        let id = definition.id.as_str();
        let seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        !seen.dismissed.iter().any(|known| known == id)
            && !seen.shown.iter().any(|known| known == id)
    }

    /// Take the one showing this session is allowed for `id`. `false` when the
    /// user dismissed it, or another document already claimed it.
    fn claim(&self, id: &str) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.dismissed.iter().any(|known| known == id)
            || seen.shown.iter().any(|known| known == id)
        {
            return false;
        }
        seen.shown.push(id.to_owned());
        true
    }

    /// Wait for the first configuration pull, bounded by [`CONFIG_SETTLE_WAIT`].
    async fn wait_for_config(&self) {
        let mut settled = self.config_settled.subscribe();
        let _ = crate::rt::timeout(CONFIG_SETTLE_WAIT, settled.wait_for(|done| *done)).await;
    }

    /// Show the notice for `definition` if the setting allows it and this
    /// session has not shown it yet. Runs as its own task: it awaits the
    /// client, and nothing it holds is a document or analyser lock.
    pub(crate) async fn offer(
        self: Arc<Self>,
        client: Client,
        definition: Arc<EnvironmentDefinition>,
    ) {
        self.wait_for_config().await;
        let id = definition.id.as_str();
        if !self.enabled() || !self.claim(id) {
            return;
        }
        let text = notice_text(&definition);
        if !self.message_action_items.load(Ordering::Relaxed) {
            client.show_message(MessageType::INFO, text).await;
            return;
        }
        let actions = [LEARN_MORE, DONT_SHOW_AGAIN]
            .into_iter()
            .map(|title| MessageActionItem {
                title: title.to_owned(),
                properties: std::collections::HashMap::new(),
            })
            .collect();
        let reply = client
            .show_message_request(MessageType::INFO, text, Some(actions))
            .await;
        match reply {
            Ok(Some(chosen)) if chosen.title == LEARN_MORE => self.learn_more(&client).await,
            Ok(Some(chosen)) if chosen.title == DONT_SHOW_AGAIN => self.dismiss(&client, id).await,
            _ => {}
        }
    }

    /// Open the KCS note: an external `window/showDocument` when the client can
    /// show one, otherwise (or when it declines) the URL on the log channel.
    async fn learn_more(&self, client: &Client) {
        if self.show_document.load(Ordering::Relaxed)
            && let Ok(uri) = LEARN_MORE_URL.parse::<Uri>()
        {
            let shown = crate::bounded_client_request(client.show_document(ShowDocumentParams {
                uri,
                external: Some(true),
                take_focus: None,
                selection: None,
            }))
            .await;
            if matches!(shown, Ok(true)) {
                return;
            }
        }
        client
            .log_message(
                MessageType::INFO,
                format!("tcl-lsp: about tool environments: {LEARN_MORE_URL}"),
            )
            .await;
    }

    /// Record the dismissal in memory at once and on disk off the async
    /// thread; a failed write leaves one warning and the dismissal in force
    /// for the rest of the session.
    async fn dismiss(self: &Arc<Self>, client: &Client, id: &str) {
        {
            let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
            if !seen.dismissed.iter().any(|known| known == id) {
                seen.dismissed.push(id.to_owned());
            }
        }
        let notice = Arc::clone(self);
        let written = crate::rt::spawn_blocking(move || notice.persist()).await;
        let failure = match written {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        client
            .log_message(
                MessageType::WARNING,
                format!("tcl-lsp: could not save your choice: {failure}"),
            )
            .await;
    }

    /// Rewrite `notices.ini` with this session's dismissals merged into what is
    /// on disk at that moment, so two editors dismissing different environments
    /// both stick. Written to a sibling file and renamed into place, so a reader
    /// never sees half a file.
    ///
    /// This writes with `std::fs` where the start-up read goes through the
    /// [`SourceStore`]: the store only reads, and holds the files a host
    /// supplies rather than the user's state directory. The path is `None`
    /// wherever the host has no such directory (off-native), so a write only
    /// ever reaches a real file system.
    fn persist(&self) -> std::io::Result<()> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        let session = self
            .seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .dismissed
            .clone();
        persist_ids(path, &session, || {})
    }
}

/// How many times [`persist_ids`] writes the merged list.
const PERSIST_ATTEMPTS: usize = 2;

/// Merge `session` into the file at `path` and check that it stuck.
///
/// Reading, merging and renaming is not one atomic step: two servers that read
/// the file before either renames each write a list that lacks the other's id,
/// and the later rename drops the earlier one's. So after each rename the file
/// is read back, and if an id of `session` is missing the merge runs once more
/// against what the other writer left, up to [`PERSIST_ATTEMPTS`] writes.
/// `after_rename` runs after each rename, before the read back.
fn persist_ids(
    path: &Path,
    session: &[String],
    mut after_rename: impl FnMut(),
) -> std::io::Result<()> {
    for _ in 0..PERSIST_ATTEMPTS {
        let on_disk = std::fs::read_to_string(path).unwrap_or_default();
        let sections = parse_ini(&on_disk);
        let mut merged = dismissed_ids(&sections);
        for id in session {
            if !merged.contains(id) {
                merged.push(id.clone());
            }
        }
        write_atomically(path, &render_notices(sections, &merged))?;
        after_rename();
        let stored = std::fs::read_to_string(path).unwrap_or_default();
        let stored = dismissed_ids(&parse_ini(&stored));
        if session.iter().all(|id| stored.contains(id)) {
            break;
        }
    }
    Ok(())
}

/// Write `content` to `path` through a temporary sibling, creating the
/// directory first.
fn write_atomically(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let scratch = scratch_path(path);
    std::fs::write(&scratch, content)?;
    std::fs::rename(&scratch, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&scratch);
    })
}

/// The temporary sibling [`write_atomically`] writes through, named for the
/// process and the moment of writing so two servers writing at once never share
/// a scratch file.
fn scratch_path(path: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let mut scratch = path.as_os_str().to_owned();
    scratch.push(format!(".{}.{stamp}.tmp", std::process::id()));
    PathBuf::from(scratch)
}

#[cfg(test)]
mod tests;
