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

//! The checked URI-to-path receipt a document's source is read under, and the canonical file URI.

use super::*;

/// Checked original URI-to-path data for one source-publication operation.
/// Non-file documents deliberately have no filesystem path; refused file URIs
/// never acquire that state. This receipt supplies no source freshness or edit grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum DocumentPathAdmission {
    NoFilesystemPath {
        uri: Uri,
    },
    Native {
        uri: Uri,
        selected: vfs::SourceUriPath,
        source_origin: Option<vfs::SourcePathOrigin>,
        analyser_spelling: String,
    },
    Virtual {
        uri: Uri,
        selected: vfs::SourceUriPath,
        source_origin: Option<vfs::SourcePathOrigin>,
        analyser_spelling: String,
    },
    WasiGuest {
        uri: Uri,
        selected: vfs::SourceUriPath,
        source_origin: Option<vfs::SourcePathOrigin>,
        analyser_spelling: String,
    },
}

/// Saved document bytes, each with the path admission that read them.
pub(in super::super) type SavedSourceBytes =
    Vec<(std::sync::Arc<DocumentPathAdmission>, std::sync::Arc<[u8]>)>;

impl DocumentPathAdmission {
    pub(in super::super) fn capture(
        uri: &Uri,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<Self> {
        let selected = match store.document_uri_path(uri.as_str()) {
            Ok(selected) => selected,
            Err(vfs::SourceUriPathUnavailable::Uri(
                uri_norm::DocumentFilePathUnavailable::NotFile,
            )) => return Ok(Self::NoFilesystemPath { uri: uri.clone() }),
            Err(vfs::SourceUriPathUnavailable::Uri(cause)) => {
                return Err(DocumentSourceUnavailable::Path {
                    uri: uri.clone(),
                    cause,
                });
            }
            Err(cause) => {
                return Err(DocumentSourceUnavailable::Store {
                    uri: uri.clone(),
                    cause,
                });
            }
        };
        if selected.original_uri() != uri.as_str() {
            return Err(DocumentSourceUnavailable::Store {
                uri: uri.clone(),
                cause: vfs::SourceUriPathUnavailable::ForeignProvider,
            });
        }
        store
            .validate_document_uri_path(&selected)
            .map_err(|cause| DocumentSourceUnavailable::Store {
                uri: uri.clone(),
                cause,
            })?;
        let analyser_spelling = selected
            .path()
            .to_str()
            .ok_or_else(|| DocumentSourceUnavailable::Path {
                uri: uri.clone(),
                cause: uri_norm::DocumentFilePathUnavailable::InvalidUnicode,
            })?
            .to_owned();
        Ok(match selected.kind() {
            vfs::SourceUriPathKind::Native => Self::Native {
                uri: uri.clone(),
                selected,
                source_origin: None,
                analyser_spelling,
            },
            vfs::SourceUriPathKind::Virtual => Self::Virtual {
                uri: uri.clone(),
                selected,
                source_origin: None,
                analyser_spelling,
            },
            vfs::SourceUriPathKind::WasiGuest => Self::WasiGuest {
                uri: uri.clone(),
                selected,
                source_origin: None,
                analyser_spelling,
            },
        })
    }

    pub(in super::super) fn from_selected_path(
        uri: &Uri,
        path: &Path,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<Self> {
        // The only caller supplies this URI directly from the existing forward
        // canonical_file_uri producer on this actual scan Path. Do not decode
        // that URI to recover or compare the original Path spelling: Windows
        // drive/verbatim product spelling deliberately differs.
        let origin =
            store
                .source_path_origin(path)
                .map_err(|cause| DocumentSourceUnavailable::Store {
                    uri: uri.clone(),
                    cause,
                })?;
        store
            .validate_source_path_origin(&origin)
            .map_err(|cause| DocumentSourceUnavailable::Store {
                uri: uri.clone(),
                cause,
            })?;
        let mut admission = Self::capture(uri, store)?;
        match &mut admission {
            Self::NoFilesystemPath { .. } => {
                return Err(DocumentSourceUnavailable::Path {
                    uri: uri.clone(),
                    cause: uri_norm::DocumentFilePathUnavailable::NotFile,
                });
            }
            Self::Native {
                selected,
                source_origin,
                analyser_spelling,
                ..
            }
            | Self::Virtual {
                selected,
                source_origin,
                analyser_spelling,
                ..
            }
            | Self::WasiGuest {
                selected,
                source_origin,
                analyser_spelling,
                ..
            } => {
                if selected.kind() != origin.kind() {
                    return Err(DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause: vfs::SourceUriPathUnavailable::ForeignProvider,
                    });
                }
                // An actual registered virtual URI is stronger than a scan's
                // forward label and must still select this exact produced path.
                if selected.kind() == vfs::SourceUriPathKind::Virtual
                    && selected.path().as_os_str() != origin.path().as_os_str()
                {
                    return Err(DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause: vfs::SourceUriPathUnavailable::PathOriginMismatch,
                    });
                }
                origin
                    .path()
                    .to_str()
                    .ok_or_else(|| DocumentSourceUnavailable::Path {
                        uri: uri.clone(),
                        cause: uri_norm::DocumentFilePathUnavailable::InvalidUnicode,
                    })?
                    .clone_into(analyser_spelling);
                *source_origin = Some(origin);
            }
        }
        admission.check_for(uri, store)?;
        Ok(admission)
    }

    pub(in super::super) fn uri(&self) -> &Uri {
        match self {
            Self::NoFilesystemPath { uri }
            | Self::Native { uri, .. }
            | Self::Virtual { uri, .. }
            | Self::WasiGuest { uri, .. } => uri,
        }
    }

    pub(in super::super) fn check_uri(&self, uri: &Uri) -> DocumentSourceResult<()> {
        if self.uri() == uri {
            Ok(())
        } else {
            Err(DocumentSourceUnavailable::ForeignReceipt {
                expected: uri.clone(),
                actual: Box::new(self.uri().clone()),
            })
        }
    }

    pub(in super::super) fn check_current(
        &self,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<()> {
        match self {
            Self::NoFilesystemPath { .. } => Ok(()),
            Self::Native {
                uri,
                selected,
                source_origin,
                ..
            }
            | Self::Virtual {
                uri,
                selected,
                source_origin,
                ..
            }
            | Self::WasiGuest {
                uri,
                selected,
                source_origin,
                ..
            } => {
                if let Some(origin) = source_origin {
                    store.validate_source_path_origin(origin).map_err(|cause| {
                        DocumentSourceUnavailable::Store {
                            uri: uri.clone(),
                            cause,
                        }
                    })?;
                }
                store.validate_document_uri_path(selected).map_err(|cause| {
                    DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause,
                    }
                })
            }
        }
    }

    pub(in super::super) fn check_for(
        &self,
        uri: &Uri,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<()> {
        self.check_uri(uri)?;
        self.check_current(store)
    }

    pub(in super::super) fn with_current<T>(
        admissions: &[&Self],
        store: &dyn vfs::SourceStore,
        mut publish: impl FnMut() -> T,
    ) -> DocumentSourceResult<T> {
        for admission in admissions {
            if let Self::Native {
                uri,
                source_origin: Some(origin),
                ..
            }
            | Self::Virtual {
                uri,
                source_origin: Some(origin),
                ..
            }
            | Self::WasiGuest {
                uri,
                source_origin: Some(origin),
                ..
            } = admission
            {
                store.validate_source_path_origin(origin).map_err(|cause| {
                    DocumentSourceUnavailable::Store {
                        uri: uri.clone(),
                        cause,
                    }
                })?;
            }
        }
        let selected = admissions
            .iter()
            .filter_map(|admission| match admission {
                Self::NoFilesystemPath { .. } => None,
                Self::Native { selected, .. }
                | Self::Virtual { selected, .. }
                | Self::WasiGuest { selected, .. } => Some(selected),
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Ok(publish());
        }
        let mut result = None;
        store
            .with_current_document_paths(&selected, &mut || result = Some(publish()))
            .map_err(|cause| DocumentSourceUnavailable::Store {
                uri: admissions[0].uri().clone(),
                cause,
            })?;
        result.ok_or_else(|| DocumentSourceUnavailable::Store {
            uri: admissions[0].uri().clone(),
            cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
        })
    }

    pub(in super::super) fn analyser_path(&self) -> Option<&str> {
        match self {
            Self::NoFilesystemPath { .. } => None,
            Self::Native {
                analyser_spelling, ..
            }
            | Self::Virtual {
                analyser_spelling, ..
            }
            | Self::WasiGuest {
                analyser_spelling, ..
            } => Some(analyser_spelling),
        }
    }

    pub(in super::super) fn path(&self) -> Option<&Path> {
        match self {
            Self::NoFilesystemPath { .. } => None,
            Self::Native {
                selected,
                source_origin,
                ..
            }
            | Self::Virtual {
                selected,
                source_origin,
                ..
            }
            | Self::WasiGuest {
                selected,
                source_origin,
                ..
            } => Some(
                source_origin
                    .as_ref()
                    .map_or_else(|| selected.path(), vfs::SourcePathOrigin::path),
            ),
        }
    }
}

impl Backend {
    pub(in super::super) async fn published_document_path_admission(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Option<Arc<DocumentPathAdmission>>> {
        // Writers update `db_files` and `source_path_admissions` together under
        // `db` and `db_files` (see `debug_assert_published_paths_in_step`), so
        // `db_files` alone yields a consistent pair without waiting on `db`.
        let selected = {
            let files = self.db_files.lock().await;
            let Some(file) = files.get(uri) else {
                return Ok(None);
            };
            let retained = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(uri)
                .cloned()
                .ok_or_else(|| DocumentSourceUnavailable::PublishedPathUnavailable {
                    uri: uri.clone(),
                })?;
            if retained.file != *file {
                return Err(DocumentSourceUnavailable::PublishedPathUnavailable {
                    uri: uri.clone(),
                });
            }
            retained.admission
        };
        selected.check_for(uri, self.store.as_ref())?;
        Ok(Some(selected))
    }

    /// Reborrow the URI's retained checked path, validating synthetic inputs
    /// through the same protocol before any source read or cache access.
    pub(in super::super) async fn document_path_admission(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Arc<DocumentPathAdmission>> {
        let checked = DocumentPathAdmission::capture(uri, self.store.as_ref())?;
        let retained = self
            .documents
            .lock("source_path_admission")
            .await
            .get(uri)
            .and_then(|state| state.path_admission.clone());
        if let Some(retained) = retained {
            retained.check_for(uri, self.store.as_ref())?;
            return Ok(retained);
        }
        if let Some(published) = self.published_document_path_admission(uri).await? {
            return Ok(published);
        }
        checked.check_for(uri, self.store.as_ref())?;
        Ok(Arc::new(checked))
    }
}

/// The URI for a file path, in the **one canonical form** both sides of the
/// protocol use.
///
/// Every URI the server constructs for itself goes through here — the workspace
/// scan, the `source` / autoload cross-file resolver, the entry-point resolver.
/// Client-sent URIs are put into the same form at the transport boundary
/// ([`uri_norm::normalise_uris_in_params`]), so a file the server found on disk
/// and the same file open in the editor are the same string.  Without that,
/// they are two documents to everything keyed by URI: find-references,
/// workspace symbols, and rename each see one file twice.
///
/// Relative paths retain the provider's filesystem canonicalisation requirement;
/// absolute paths need not exist and keep their supplied spelling. The selected
/// actual path must be losslessly representable as Unicode: opaque native units
/// return `None`, while a literal Unicode replacement character remains data.
///
/// URI construction uses the shared Core actual-Path owner with an explicit
/// VS Code product spelling; its compatibility alias cannot recover the actual
/// Path. The scan retains a separate forward carrier through provider admission.
#[must_use]
pub fn canonical_file_uri<P: AsRef<Path>>(path: P) -> Option<Uri> {
    constructed_file_uri(path.as_ref()).map(|constructed| constructed.uri)
}

/// Forward-only geometry: the resolved actual Path and its product alias stay
/// together. This supplies no provider, filesystem identity or source currency.
pub(in super::super) struct ConstructedFileUri {
    pub(in super::super) path: PathBuf,
    pub(in super::super) uri: Uri,
}

impl ConstructedFileUri {
    pub(in super::super) fn admission(
        &self,
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<DocumentPathAdmission> {
        DocumentPathAdmission::from_selected_path(&self.uri, &self.path, store)
    }
}

pub(in super::super) fn constructed_file_uri(path: &Path) -> Option<ConstructedFileUri> {
    // A virtual-workspace document keeps the scheme it was opened under.
    if let Some(original) = tcl_lsp_core::file_uri::from_virtual_uri_path(path) {
        return Some(ConstructedFileUri {
            path: path.to_path_buf(),
            uri: Uri::from_str(&original).ok()?,
        });
    }
    // Preserve the original absolute/no-existence and relative/canonicalise policy.
    // A browser worker (wasm32-unknown-unknown) has neither `Path::is_absolute`
    // for a rooted path nor a filesystem to canonicalise against; its store's
    // virtual paths are rooted, so a rooted path is the absolute spelling there.
    let rooted_without_os =
        cfg!(all(target_family = "wasm", target_os = "unknown")) && path.has_root();
    let selected = if path.is_absolute() || rooted_without_os {
        std::borrow::Cow::Borrowed(path)
    } else {
        std::borrow::Cow::Owned(std::fs::canonicalize(path).ok()?)
    };
    let raw = tcl_lsp_core::file_uri::from_path_for_vscode_document(selected.as_ref())?;
    let canonical = uri_norm::canonical_uri_string(&raw);
    Some(ConstructedFileUri {
        path: selected.into_owned(),
        uri: Uri::from_str(&canonical).ok()?,
    })
}
