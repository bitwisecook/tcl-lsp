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

//! File-extension registrations and external watchers kept in line with the loaded packs.

use std::path::{Path, PathBuf};

use tower_lsp_server::ls_types::{
    DidChangeWatchedFilesRegistrationOptions, FileOperationFilter, FileOperationPattern,
    FileOperationPatternOptions, FileOperationRegistrationOptions, FileSystemWatcher, GlobPattern,
    MessageType, OneOf, Registration, RelativePattern, Unregistration, Uri, WatchKind,
};

use crate::Backend;
use crate::documents::canonical_file_uri;
use crate::protocol::bounded_client_request;

impl Backend {
    /// Bring the registrations and the index into line with the extensions
    /// the freshly-loaded packs claim.
    ///
    /// A pack's `file_extension` row is consulted by
    /// `dialect_from_extension`, which decides the dialect of a document the
    /// server is *already looking at*. If everything that decides which files
    /// the server looks at read the static `TCL_SOURCE_EXTENSIONS` alone, a
    /// pack claiming `.irulex` would give a correct answer for an open
    /// `.irulex` file and nothing at all for a closed one: no workspace index
    /// entry, so no cross-file references, definitions or rename; and no
    /// watcher, so no external edit would ever refresh any of it.
    ///
    /// Three things therefore have to follow the pack set:
    ///
    /// 1. **`is_tcl_source`**, which reads the live snapshot directly and so
    ///    needs nothing here.
    /// 2. **The client-side registrations** — the watched-file glob and the
    ///    `willRename`/`didRename` filters. Those are patterns the *client*
    ///    matches, so they have to be re-registered; the static ones are set
    ///    at `initialize`, long before any pack is read. Dynamic
    ///    re-registration under their own ids, exactly as
    ///    [`Backend::refresh_external_pack_watchers`] does, leaves the static
    ///    registrations untouched.
    /// 3. **The existing files on disk.** A newly-claimed extension does not
    ///    make the last scan's results grow; only a fresh scan indexes the
    ///    `.irulex` files that were already there. That is the expensive step,
    ///    which is why it is gated on the extension set having actually moved
    ///    rather than on any pack reload.
    ///
    /// Returns whether the claimed set changed, so the caller can decide
    /// whether to pay for the re-scan.
    pub(super) async fn refresh_pack_extension_registrations(&self) -> bool {
        const WATCH_ID: &str = "tcl-lsp-pack-extension-files";
        const WATCH_METHOD: &str = "workspace/didChangeWatchedFiles";
        const WILL_RENAME_ID: &str = "tcl-lsp-pack-extension-will-rename";
        const WILL_RENAME_METHOD: &str = "workspace/willRenameFiles";
        const DID_RENAME_ID: &str = "tcl-lsp-pack-extension-did-rename";
        const DID_RENAME_METHOD: &str = "workspace/didRenameFiles";

        let extensions = tcl_registry::dialects::pack_source_extensions();
        let watch_globs =
            tcl_registry::dialects::extension_globs_any_case(extensions.iter().map(String::as_str));
        let had_registrations = {
            let mut current = self.pack_source_extensions.lock().await;
            if *current == extensions {
                return false;
            }
            let had = !current.is_empty();
            current.clone_from(&extensions);
            had
        };

        // Unregister before registering: the ids are stable, and a client
        // still holding the previous set would otherwise match both — which
        // for a *retired* extension means events keep arriving for files the
        // server no longer considers source.
        if had_registrations {
            for (id, method) in [
                (WATCH_ID, WATCH_METHOD),
                (WILL_RENAME_ID, WILL_RENAME_METHOD),
                (DID_RENAME_ID, DID_RENAME_METHOD),
            ] {
                let _ = bounded_client_request(self.client.unregister_capability(vec![
                    Unregistration {
                        id: id.to_owned(),
                        method: method.to_owned(),
                    },
                ]))
                .await;
            }
        }

        let watch_globs = match watch_globs {
            Ok(patterns) => patterns,
            Err(reason) => {
                self.client.log_message(MessageType::WARNING,
                    format!("pack extensions cannot be represented as exact editor globs: {reason:?}")).await;
                // The source-extension identity changed even though client glob
                // registration was refused. Keep the independent index rescan.
                return true;
            }
        };

        if watch_globs.is_empty() {
            // Every pack extension retired; this is distinct from rendering refusal.
            return true;
        }

        let all_kinds = Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete);
        let mut registrations = vec![Registration {
            id: WATCH_ID.to_owned(),
            method: WATCH_METHOD.to_owned(),
            register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                watchers: watch_globs
                    .iter()
                    .map(|watch_glob| FileSystemWatcher {
                        // Case-folded per character for the same reason the static
                        // watcher is: watcher registrations carry no
                        // `ignoreCase` option and a client matches them
                        // case-sensitively on Linux.
                        glob_pattern: GlobPattern::String(watch_glob.clone()),
                        kind: all_kinds,
                    })
                    .collect(),
            })
            .ok(),
        }];

        // Rename filters use the same literal target encoding as watchers;
        // ignoreCase remains the protocol option independently of that encoding.
        let rename_options = FileOperationRegistrationOptions {
            filters: watch_globs
                .iter()
                .map(|glob| FileOperationFilter {
                    scheme: Some("file".to_owned()),
                    pattern: FileOperationPattern {
                        glob: glob.clone(),
                        // `None`, exactly as the static filter leaves it: the glob
                        // ends in an extension, so a directory cannot match it and
                        // naming a kind would only be a second way to say so.
                        matches: None,
                        options: Some(FileOperationPatternOptions {
                            ignore_case: Some(true),
                        }),
                    },
                })
                .collect(),
        };
        for (id, method) in [
            (WILL_RENAME_ID, WILL_RENAME_METHOD),
            (DID_RENAME_ID, DID_RENAME_METHOD),
        ] {
            registrations.push(Registration {
                id: id.to_owned(),
                method: method.to_owned(),
                register_options: serde_json::to_value(&rename_options).ok(),
            });
        }

        if let Err(err) =
            bounded_client_request(self.client.register_capability(registrations)).await
        {
            // Same contract as every other dynamic registration here: a client
            // that cannot honour one declines it, and the server keeps working
            // with whatever the startup scan seeded.
            self.client
                .log_message(
                    MessageType::LOG,
                    format!("pack-extension registration declined by client: {err}"),
                )
                .await;
        }
        true
    }

    /// Re-register the watchers for pack directories outside every workspace
    /// folder, when that set has moved since the last reload.
    ///
    /// The session-wide registration in [`Backend::register_file_watchers`]
    /// uses a workspace-*relative* pattern, and a client matches such a
    /// pattern only within its workspace folders. A pack in the per-user tier
    /// (`<config>/specs`), or named by an absolute `tclLsp.specPacks` entry
    /// pointing outside the workspace, is therefore discovered and loaded at
    /// startup but never watched: editing it produced no
    /// `didChangeWatchedFiles`, so its commands stayed as they were until an
    /// unrelated configuration reload or a server restart. Absolute glob
    /// patterns fix that; LSP allows them, and a client that cannot honour one
    /// declines the registration exactly as it may decline the main one.
    ///
    /// Registered under its own id so it can be replaced when the configured
    /// roots change without disturbing the source-file watcher.
    pub(in super::super) async fn refresh_external_pack_watchers(
        &self,
        options: &tcl_spectcl::DiscoveryOptions,
    ) {
        const REGISTRATION_ID: &str = "tcl-lsp-external-spec-packs";
        const METHOD: &str = "workspace/didChangeWatchedFiles";

        let mut roots: Vec<PathBuf> = Vec::new();
        if !options.skip_user_tier {
            roots.push(
                options
                    .user_dir
                    .clone()
                    .unwrap_or_else(tcl_spectcl::discovery::user_dir),
            );
        }
        // Absolute settings entries only: a relative one resolves inside a
        // workspace folder, which the main registration already covers.
        let configured_absolute = options
            .configured
            .iter()
            .map(|path| (None, path))
            .chain(
                options
                    .folder_configured
                    .iter()
                    .flat_map(|(folder, paths)| paths.iter().map(move |p| (Some(folder), p))),
            )
            .filter(|(_, path)| path.is_absolute())
            .map(|(_, path)| path.clone());
        roots.extend(configured_absolute);

        // Anything inside a workspace folder is already watched; registering it
        // again would only make the client send every event twice.
        let inside_workspace = |path: &Path| {
            options
                .workspace_roots
                .iter()
                .any(|root| path.starts_with(root))
        };
        // Each watch is a base directory plus a pattern under it, because
        // that is the only shape a client can honour for a directory outside
        // the workspace (`RelativePattern`, LSP 3.17). The joined spelling is
        // still what the change-detection below compares.
        let mut watches: Vec<(PathBuf, String)> = roots
            .iter()
            .filter(|path| !inside_workspace(path))
            .filter_map(|path| {
                // A settings entry may name one file or a directory to scan;
                // discovery accepts both, so the watch has to cover both.
                if tcl_spectcl::discovery::is_pack_file(path) {
                    let parent = path.parent()?;
                    let name = path.file_name()?.to_str()?;
                    Some((parent.to_path_buf(), name.to_owned()))
                } else {
                    Some((path.clone(), "**/*.tclspec".to_owned()))
                }
            })
            .collect();
        watches.sort();
        watches.dedup();
        let mut globs: Vec<String> = watches
            .iter()
            .filter_map(|(base, pattern)| {
                Some(format!(
                    "{}/{pattern}",
                    base.to_str()?.trim_end_matches('/')
                ))
            })
            .collect();
        globs.sort();
        globs.dedup();

        {
            let mut current = self.external_pack_watch_globs.lock().await;
            if *current == globs {
                return;
            }
            // Unregister before registering: the id is the same, and a client
            // that already holds it would otherwise be left with both sets.
            if !current.is_empty() {
                let _ = bounded_client_request(self.client.unregister_capability(vec![
                    Unregistration {
                        id: REGISTRATION_ID.to_owned(),
                        method: METHOD.to_owned(),
                    },
                ]))
                .await;
            }
            current.clone_from(&globs);
        }
        if globs.is_empty() {
            return;
        }
        // Without `relativePatternSupport` there is no spelling for "watch
        // this directory outside the workspace": a bare absolute pattern is
        // matched against the workspace instead, which costs such a client
        // one watch descriptor per project directory rather than declining.
        // The packs are still discovered and loaded at startup; only the live
        // reload on an external edit is lost, which is what a client that
        // declines the registration gets anyway.
        if !self
            .client_supports_relative_watch_patterns
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.client
                .log_message(
                    MessageType::LOG,
                    format!(
                        "external spec-pack watchers not registered: client has no \
                         workspace.didChangeWatchedFiles.relativePatternSupport ({} root(s))",
                        globs.len()
                    ),
                )
                .await;
            return;
        }

        let registration = Registration {
            id: REGISTRATION_ID.to_owned(),
            method: METHOD.to_owned(),
            register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                watchers: watches
                    .iter()
                    .filter_map(|(base, pattern)| external_pack_file_watcher(base, pattern))
                    .collect(),
            })
            .ok(),
        };
        if let Err(err) =
            bounded_client_request(self.client.register_capability(vec![registration])).await
        {
            self.client
                .log_message(
                    MessageType::LOG,
                    format!("external spec-pack watcher registration declined by client: {err}"),
                )
                .await;
        }
    }
}

fn external_pack_file_watcher(base: &Path, pattern: &str) -> Option<FileSystemWatcher> {
    Some(relative_pack_file_watcher(
        canonical_file_uri(base)?,
        pattern,
    ))
}

fn relative_pack_file_watcher(base_uri: Uri, pattern: &str) -> FileSystemWatcher {
    FileSystemWatcher {
        glob_pattern: GlobPattern::Relative(RelativePattern {
            base_uri: OneOf::Right(base_uri),
            pattern: pattern.to_owned(),
        }),
        kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
    }
}

#[cfg(test)]
#[test]
fn windows_pack_uri_spelling_matches_did_open_document_key() {
    use crate::uri_norm::{canonical_uri_string, normalise_uris_in_params};
    use tcl_lsp_core::file_uri::vscode_document_spelling;
    let pattern = "**/*.tclspec";

    for (base, editor_uri) in [
        (
            r"C:\workspace\private pack",
            "file:///c%3A/workspace/private%20pack",
        ),
        (
            r"\\?\C:\workspace\private pack",
            "file:///C%3a/workspace/private%20pack",
        ),
        (
            r"\\?\UNC\server\share\private pack",
            "file://server/share/private%20pack",
        ),
    ] {
        let projected = vscode_document_spelling(base, true).unwrap();
        let pack_uri = canonical_uri_string(&projected).parse().unwrap();
        let watcher = relative_pack_file_watcher(pack_uri, pattern);
        let mut opened = serde_json::json!({ "textDocument": { "uri": editor_uri } });
        normalise_uris_in_params(&mut opened);
        let document_uri: Uri = opened["textDocument"]["uri"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            watcher.glob_pattern,
            GlobPattern::Relative(RelativePattern {
                base_uri: OneOf::Right(document_uri),
                pattern: pattern.to_owned(),
            }),
            "{base}",
        );
    }
}

#[cfg(all(test, windows))]
#[test]
fn pack_watch_and_open_document_use_the_same_windows_uri() {
    let base = Path::new(r"C:\workspace\private pack");
    let watcher = external_pack_file_watcher(base, "**/*.tclspec").unwrap();
    let GlobPattern::Relative(RelativePattern {
        base_uri: OneOf::Right(uri),
        ..
    }) = watcher.glob_pattern
    else {
        panic!("expected a rooted relative watch pattern");
    };
    let mut opened = serde_json::json!({ "uri": "file:///c%3A/workspace/private%20pack" });
    crate::uri_norm::normalise_uris_in_params(&mut opened);
    assert_eq!(uri.as_str(), opened["uri"].as_str().unwrap());
}
