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

//! Where the server's *closed* files come from.
//!
//! Open buffers retain their text in the server's document store, while this
//! module selects their actual native, virtual or guest document URI path.
//! Its raw I/O methods supply the other half of the picture: the file a `source` edge points at, the sibling the workspace scan
//! indexed, the `config.ini` a session layers under the editor's settings, the
//! `pkgIndex.tcl` the package database parsed, the `.tclspec` a pack was loaded
//! from. Natively those are all `std::fs` calls. In a browser worker there is
//! no `std::fs` to call — the host has the bytes and has to hand them over.
//!
//! [`SourceStore`] is that seam, and `tcl_lsp_server::Backend` holds exactly
//! one of them.
//!
//! * [`NativeStore`] is the native implementation and is a literal delegation
//!   to `std::fs` — the same calls, in the same order, returning the same
//!   `io::Error`s a direct call site would. A native server
//!   retains the raw file-I/O behavior of those direct calls: no caching,
//!   path rewriting or normalization is inserted into them. Its separate
//!   document-URI capability selects the checked native path namespace, or
//!   the actual WASI guest namespace on WASI, before a consumer uses raw I/O.
//! * [`MemoryStore`] is the browser implementation: a `PathBuf`-keyed byte map
//!   the host fills in over `postMessage`, plus a URI alias table so a file
//!   registered under the client's spelling of a URI is still found when the
//!   server re-derives a path from its own canonical spelling. A path the host
//!   never supplied reads as [`io::ErrorKind::NotFound`], which every call
//!   site here already treats as "that file contributes nothing" — the same
//!   answer a native server gives for a file that genuinely is not there.
//!
//! # Why the trait lives here
//!
//! It was born in `tcl-lsp-server`, where every call site was. Finishing the
//! seam moved the whole-workspace paths onto it, and two of those live below
//! the server: the package database ([`crate::package_resolver`]) and
//! `.tclspec` discovery (`tcl_spectcl::discovery`). `tcl-lsp-core` is the
//! lowest crate that can hold the trait *and both implementations* undivided —
//! [`SourceStore::read_source`] needs [`crate::source_decode`], so a home
//! below this crate would split the decode default off the trait.
//! `tcl-core-types`, the other leaf both dependants share, is `#![no_std]` and
//! cannot name `std::fs` at all; `tcl-platform` bans syscalls by charter and
//! already owns a host-filesystem trait, so a second one there would put two
//! owners of one axis in one crate. `tcl-spectcl` therefore depends on this
//! crate for the trait, and `tcl_lsp_server::vfs` re-exports it so the server's
//! own call sites did not move.
//!
//! # Scope
//!
//! Every path the server reads a closed file on now goes through the store:
//! the single-document chokepoints (the closed-file read behind
//! `read_document`, the decode-report comparison, `scan_disk_file`, the
//! `config.ini` layers, the `exportConfig` write, the notice line-range read,
//! the recovery-widening package read) **and** the whole-workspace ones — the
//! workspace folder scan, the package database
//! ([`scan_path_in`](crate::package_resolver::PackageResolver::scan_path_in) /
//! [`scan_tree_in`](crate::package_resolver::PackageResolver::scan_tree_in)),
//! the W120 / W123 package scans, and spec-pack discovery. A host that fills a
//! [`MemoryStore`] gets a real workspace session, not an empty one.
//!
//! The store-free spellings (`scan_path`, `scan_tree`,
//! `tcl_spectcl::discover`, `bundled::load_discovered`) survive as
//! [`NativeStore`] wrappers, the same shape `Backend::new` and
//! `Backend::with_store` use — native callers and their tests did not move, so
//! the store-taking form cannot change native behaviour by accident.
//!
//! What is deliberately still `std::fs`, because each is a capability a byte
//! map genuinely does not have: the compiled-pack disk cache
//! (`tcl_spectcl::cache`, a native-only speed-up that already no-ops when its
//! directory is unwritable), `std::env::current_exe`-relative bundled-pack
//! discovery and its `canonicalize` (see
//! `tcl_spectcl::discovery::VIRTUAL_PACK_MOUNT` for the store path that
//! replaces the former), and Tcl-installation discovery
//! ([`crate::tcl_install`], which probes for a native interpreter a browser has
//! no equivalent of).
//!
//! `docs/design/contracts/lsp-source-store.md` is the full contract.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// Whether a source store is backed by native filesystem enumeration or a
/// host-issued browser census. This distinction never treats missing host data
/// as filesystem completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSourceInventoryProvider {
    /// The native store delegates to the actual filesystem enumerator.
    Filesystem,
    /// The browser memory store relies on the host's explicit census.
    HostReported,
    /// A provider has not established either inventory authority.
    Untracked,
}

/// Whether a host's selected workspace-file roster is complete for one root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSourceInventoryCompleteness {
    /// The host enumerated every selected file and admitted each source byte sequence.
    Complete,
    /// Search, byte admission, read, or budget refusal left the roster partial.
    Incomplete,
}

/// One host-issued closed-file inventory receipt. Its session, root, selection, and
/// source-store revision are compared at the final edit decision; it is not a
/// freestanding claim that a project has no additional files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSourceInventoryReceipt {
    /// Identity of the source-store session that issued this receipt.
    pub session_id: u64,
    /// Admitted provider root whose selected inventory was enumerated.
    pub root: PathBuf,
    /// Store revision captured when this root inventory was issued.
    pub source_revision: u64,
    /// Opaque host selection identifier retained without interpretation.
    pub selection: String,
    /// Whether all selected source files were admitted by this host census.
    pub completeness: WorkspaceSourceInventoryCompleteness,
}

/// Current view of the host-issued source inventory for an admitted root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSourceInventoryObservation {
    /// This provider has no host-census protocol; its filesystem owner remains separate.
    NotTracked,
    /// The tracked provider has not issued a receipt for this root.
    Missing {
        /// Current source-store session lacking a receipt for this root.
        session_id: u64,
        /// Admitted provider root for which no receipt is available.
        root: PathBuf,
        /// Current store revision at the failed receipt lookup.
        source_revision: u64,
    },
    /// The receipt predates a source-store mutation.
    Stale {
        /// Earlier receipt that no longer matches the store revision.
        receipt: WorkspaceSourceInventoryReceipt,
        /// Store revision against which the earlier receipt was checked.
        current_revision: u64,
    },
    /// The receipt matches the current session and source-store revision.
    Current(WorkspaceSourceInventoryReceipt),
}

impl WorkspaceSourceInventoryObservation {
    /// Whether the provider receipt is current and complete. Independent index
    /// scan coverage and final edit-plan currentness are still required.
    #[must_use]
    pub fn admits_complete_inventory(&self) -> bool {
        matches!(self, Self::Current(receipt) if receipt.completeness == WorkspaceSourceInventoryCompleteness::Complete)
    }
}

static NEXT_MEMORY_STORE_SESSION: AtomicU64 = AtomicU64::new(1);

/// What a directory listing reports about one entry.
///
/// Deliberately smaller than `std::fs::DirEntry`: the callers only ever ask
/// for the path and the entry's kind, and a browser store has no honest answer
/// for anything more.
///
/// `is_dir` and `is_file` are the two questions `std::fs::FileType` answers,
/// and they are **both false** for a symlink — the walks that consult them (the
/// workspace scan and `.tclspec` discovery) ask `file_type().is_dir()` /
/// `.is_file()`, so a symlink is neither descended into nor indexed.
/// Collapsing them to one flag would silently start indexing symlinked
/// sources.
///
/// The package-index scan is the counter-example, and it is why these are not
/// the only kind test in the trait: it asked `path.is_dir()` on the entry, a
/// `stat` that *does* follow a symlink, because C Tcl's `auto_path` walk
/// descends a symlinked package directory. That caller uses
/// [`SourceStore::is_dir`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's full path.
    pub path: PathBuf,
    /// Whether the entry is a directory, not following a symlink.
    pub is_dir: bool,
    /// Whether the entry is a regular file, not following a symlink.
    pub is_file: bool,
}

/// The subset of `std::fs::Metadata` this crate actually consults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// Whether the path names a directory.
    pub is_dir: bool,
    /// The file's length in bytes (zero for a directory).
    pub len: u64,
}

/// The actual resource namespace that issued a document path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceUriPathKind {
    /// A path interpreted by the selected native host filesystem provider.
    Native,
    /// A path interpreted in the selected host-filled virtual source store.
    Virtual,
    /// A rooted path interpreted by actual WASI filesystem calls and mounts.
    WasiGuest,
}

/// A source provider could not admit or retain the selected URI path.
/// This is distinct from an admitted path whose file is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceUriPathUnavailable {
    /// The strict URI/path protocol refused the original input.
    Uri(crate::file_uri::FileUriPathUnavailable),
    /// The actual selected provider or retained issuer is unavailable.
    ProviderUnavailable,
    /// The receipt was issued by a different source provider or namespace.
    ForeignProvider,
    /// The selected alias was removed, replaced, or superseded by registration.
    WithdrawnMapping,
    /// A stronger URI mapping selects a different path from the actual source producer.
    PathOriginMismatch,
    /// The exact selected saved image is missing, changed or unreadable.
    /// This supplies no file-absence or physical-identity verdict.
    SourceImageUnavailable,
}

impl std::fmt::Display for SourceUriPathUnavailable {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uri(cause) => write!(out, "source URI path is unavailable: {cause:?}"),
            Self::ProviderUnavailable => {
                out.write_str("the selected source provider has no URI path capability")
            }
            Self::ForeignProvider => {
                out.write_str("the path receipt belongs to another source provider")
            }
            Self::WithdrawnMapping => {
                out.write_str("the selected URI mapping is no longer current")
            }
            Self::SourceImageUnavailable => {
                out.write_str("the selected saved source image is unavailable")
            }
            Self::PathOriginMismatch => {
                out.write_str("the selected URI mapping does not name the produced source path")
            }
        }
    }
}
impl std::error::Error for SourceUriPathUnavailable {}
impl From<crate::file_uri::FileUriPathUnavailable> for SourceUriPathUnavailable {
    fn from(cause: crate::file_uri::FileUriPathUnavailable) -> Self {
        Self::Uri(cause)
    }
}

#[derive(Debug)]
struct UriAliasBinding {
    path: PathBuf,
    aliases: Vec<String>,
}
type UriAliases = Mutex<HashMap<String, Arc<UriAliasBinding>>>;

#[derive(Debug, Clone)]
enum SourceUriSelection {
    Filesystem,
    Memory {
        owner: Weak<UriAliases>,
        binding: Option<Arc<UriAliasBinding>>,
    },
}

#[derive(Debug, Clone)]
enum SourcePathIssuer {
    Filesystem,
    Memory { owner: Weak<UriAliases> },
}

/// The original typed Path selected by an actual source provider's path caller.
/// This keeps native drive/verbatim spelling separate from a forward URI's
/// product spelling and from strict incoming URI decoding. It grants no file
/// identity, existence, freshness, native file action or execution authority.
#[derive(Debug, Clone)]
pub struct SourcePathOrigin {
    path: PathBuf,
    kind: SourceUriPathKind,
    issuer: SourcePathIssuer,
}
impl PartialEq for SourcePathOrigin {
    fn eq(&self, other: &Self) -> bool {
        self.path.as_os_str() == other.path.as_os_str()
            && self.kind == other.kind
            && match (&self.issuer, &other.issuer) {
                (SourcePathIssuer::Filesystem, SourcePathIssuer::Filesystem) => true,
                (SourcePathIssuer::Memory { owner: a }, SourcePathIssuer::Memory { owner: b }) => {
                    Weak::ptr_eq(a, b)
                }
                _ => false,
            }
    }
}
impl Eq for SourcePathOrigin {}
impl SourcePathOrigin {
    /// The original selected Path spelling, without a URI inverse roundtrip.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// The actual source provider's native, virtual or guest namespace.
    #[must_use]
    pub fn kind(&self) -> SourceUriPathKind {
        self.kind
    }
}

/// Immutable original URI/path data selected by an actual SourceStore.
/// Native, virtual and guest paths remain distinct. Retention grants no file
/// identity, existence, source freshness or native host file-action authority.
#[derive(Debug, Clone)]
pub struct SourceUriPath {
    original_uri: Arc<str>,
    path: PathBuf,
    kind: SourceUriPathKind,
    selection: SourceUriSelection,
}

impl PartialEq for SourceUriPath {
    fn eq(&self, other: &Self) -> bool {
        self.original_uri == other.original_uri
            && self.path.as_os_str() == other.path.as_os_str()
            && self.kind == other.kind
            && match (&self.selection, &other.selection) {
                (SourceUriSelection::Filesystem, SourceUriSelection::Filesystem) => true,
                (
                    SourceUriSelection::Memory {
                        owner: a,
                        binding: x,
                    },
                    SourceUriSelection::Memory {
                        owner: b,
                        binding: y,
                    },
                ) => {
                    Weak::ptr_eq(a, b)
                        && match (x, y) {
                            (None, None) => true,
                            (Some(x), Some(y)) => Arc::ptr_eq(x, y),
                            _ => false,
                        }
                }
                _ => false,
            }
    }
}
impl Eq for SourceUriPath {}

impl SourceUriPath {
    #[must_use]
    /// The complete original URI admitted by the actual selected provider.
    pub fn original_uri(&self) -> &str {
        &self.original_uri
    }
    #[must_use]
    /// The selected path in this receipt's declared resource namespace.
    pub fn path(&self) -> &Path {
        &self.path
    }
    #[must_use]
    /// The actual provider's path namespace, without a file-action grant.
    pub fn kind(&self) -> SourceUriPathKind {
        self.kind
    }

    /// Recheck the retained issuer/selection, including registration withdrawal
    /// and replacement. Callers also ask the actual selected store to validate
    /// provider correspondence before consuming or publishing this path.
    ///
    /// # Errors
    /// Returns `ProviderUnavailable` if the retained issuer has ended, or
    /// `WithdrawnMapping` if its actual URI registration is no longer selected.
    pub fn check_current(&self) -> Result<(), SourceUriPathUnavailable> {
        match &self.selection {
            SourceUriSelection::Filesystem => Ok(()),
            SourceUriSelection::Memory { owner, binding } => {
                let owner = owner
                    .upgrade()
                    .ok_or(SourceUriPathUnavailable::ProviderUnavailable)?;
                let aliases = MemoryStore::lock(&owner);
                match (aliases.get(self.original_uri()), binding) {
                    (None, None) => Ok(()),
                    (Some(current), Some(selected)) if Arc::ptr_eq(current, selected) => Ok(()),
                    _ => Err(SourceUriPathUnavailable::WithdrawnMapping),
                }
            }
        }
    }
}

/// The source of every file the server reads that the editor has not opened.
///
/// Implementations must be cheap to share (`Backend` holds an `Arc<dyn
/// SourceStore>` and clones it into blocking tasks) and must report a missing
/// path as [`io::ErrorKind::NotFound`] rather than panicking.
pub trait SourceStore: std::fmt::Debug + Send + Sync {
    /// Select the actual document URI path for this provider. The default is
    /// explicitly unavailable; missing capability is never file absence.
    ///
    /// # Errors
    /// Returns the strict URI refusal or unavailable provider/selection cause.
    fn document_uri_path(
        &self,
        _original_uri: &str,
    ) -> Result<SourceUriPath, SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Retain an original typed Path from this provider's actual source caller.
    /// This producer does not parse a URI or canonicalize the path. A forward
    /// URI producer and raw incoming URI admission remain separate operations.
    ///
    /// # Errors
    /// Returns unavailable when the provider cannot issue this path namespace
    /// or the path has unsupported source geometry.
    fn source_path_origin(
        &self,
        _path: &Path,
    ) -> Result<SourcePathOrigin, SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Authenticate the issuer/namespace of an original source Path. This
    /// supplies no correspondence to an independently chosen URI.
    ///
    /// # Errors
    /// Refuses an unavailable or foreign source provider/path namespace.
    fn validate_source_path_origin(
        &self,
        _origin: &SourcePathOrigin,
    ) -> Result<(), SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Authenticate this retained path against the actual provider and its
    /// current selected registration/policy. No default grants correspondence.
    ///
    /// # Errors
    /// Refuses a missing capability, foreign provider, or withdrawn mapping.
    fn validate_document_uri_path(
        &self,
        _selected: &SourceUriPath,
    ) -> Result<(), SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Validate the complete selected mapping set and invoke one bounded,
    /// synchronous publication while its provider mapping remains selected.
    /// The continuation must not await, perform I/O, or reenter this store's
    /// URI mapping methods. Implementations invoke it exactly once after
    /// authenticating the complete set. The default supplies no capability.
    ///
    /// # Errors
    /// Refuses before invoking the continuation if any selected provider or
    /// mapping is unavailable, foreign, or withdrawn.
    fn with_current_document_paths(
        &self,
        _selected: &[&SourceUriPath],
        _publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Reborrow the actual selected saved images before one bounded synchronous
    /// publication. MemoryStore holds mappings and bytes throughout; Native/WASI
    /// reobserves actual saved bytes immediately before the callback, without
    /// excluding external writers or granting a physical snapshot/Native role.
    /// The callback must not await, perform I/O or reenter this store.
    /// Missing capability is unavailable, never successful empty standing.
    ///
    /// # Errors
    /// Refuses an unavailable/foreign/withdrawn selection or changed/unreadable
    /// expected saved image before invoking the callback.
    fn with_current_document_sources(
        &self,
        _selected: &[(&SourceUriPath, &[u8])],
        _publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    }

    /// Identify the source-census authority for this provider.
    fn workspace_source_inventory_provider(&self) -> WorkspaceSourceInventoryProvider {
        WorkspaceSourceInventoryProvider::Untracked
    }

    /// Observe the provider-issued root census against its current session and
    /// source-store revision. The default makes no completion claim.
    fn workspace_source_inventory(&self, _root: &Path) -> WorkspaceSourceInventoryObservation {
        WorkspaceSourceInventoryObservation::NotTracked
    }

    /// Read a file's bytes.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error; a path the store does not hold is
    /// [`io::ErrorKind::NotFound`].
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Read a file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// As [`Self::read`], plus [`io::ErrorKind::InvalidData`] for bytes that
    /// are not valid UTF-8 — matching `std::fs::read_to_string`. Callers that
    /// must tolerate an ill-formed byte use [`Self::read_source`] instead.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// List a directory's immediate entries, in unspecified order.
    ///
    /// An entry that cannot itself be read is **skipped**, not reported: every
    /// caller in the tree wrote `std::fs::read_dir(dir)?.flatten()`, so a
    /// mid-listing error costs that one entry and never the whole directory.
    ///
    /// `is_dir` is the entry's own type and does not follow a symlink —
    /// `std::fs::DirEntry::file_type`, not a `stat` of the target. A caller
    /// that wants the target's kind asks [`Self::is_dir`] with the entry path.
    ///
    /// # Errors
    ///
    /// Propagates the error of *opening* the directory.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;

    /// Report what a path is.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error; a path the store does not hold is
    /// [`io::ErrorKind::NotFound`].
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;

    /// Replace a file's contents, creating it if needed.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error.
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Create a directory and every missing parent.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Whether the path names a readable file.
    fn is_file(&self, path: &Path) -> bool {
        self.metadata(path).is_ok_and(|m| !m.is_dir)
    }

    /// Whether the path names a readable directory.
    fn is_dir(&self, path: &Path) -> bool {
        self.metadata(path).is_ok_and(|m| m.is_dir)
    }

    /// Whether anything exists at the path.
    fn exists(&self, path: &Path) -> bool {
        self.metadata(path).is_ok()
    }

    /// Read a file through the shared lossy decoder.
    ///
    /// The counterpart of [`crate::source_decode::read_source_file`], and the
    /// reason it exists: a closed file with one ill-formed byte
    /// must still contribute its procs, its `source` edges, and its symbols,
    /// so decoding is lossy and only the *read* can fail.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error. Decoding itself never fails.
    fn read_source(&self, path: &Path) -> io::Result<(String, crate::source_decode::DecodeReport)> {
        Ok(crate::source_decode::decode_source(&self.read(path)?))
    }
}

/// The real filesystem — a direct delegation to `std::fs`.
///
/// Every method is the plain `std::fs` call. Nothing is cached, rewritten, or
/// normalised: the native server's behaviour is defined to be `std::fs`'s.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeStore;

impl SourceStore for NativeStore {
    fn workspace_source_inventory_provider(&self) -> WorkspaceSourceInventoryProvider {
        WorkspaceSourceInventoryProvider::Filesystem
    }

    fn document_uri_path(
        &self,
        original_uri: &str,
    ) -> Result<SourceUriPath, SourceUriPathUnavailable> {
        let (path, kind) = if cfg!(target_os = "wasi") {
            (
                crate::file_uri::to_source_namespace_path(
                    original_uri,
                    crate::file_uri::SourceFileUriNamespace::WasiGuest,
                )?,
                SourceUriPathKind::WasiGuest,
            )
        } else {
            (
                crate::file_uri::to_path(
                    original_uri,
                    crate::file_uri::FileUriPathPurpose::VsCodeDocument,
                )?,
                SourceUriPathKind::Native,
            )
        };
        Ok(SourceUriPath {
            original_uri: Arc::from(original_uri),
            path,
            kind,
            selection: SourceUriSelection::Filesystem,
        })
    }

    fn source_path_origin(
        &self,
        path: &Path,
    ) -> Result<SourcePathOrigin, SourceUriPathUnavailable> {
        let kind = if cfg!(target_os = "wasi") {
            SourceUriPathKind::WasiGuest
        } else if cfg!(any(unix, windows)) {
            SourceUriPathKind::Native
        } else {
            return Err(SourceUriPathUnavailable::ProviderUnavailable);
        };
        let rooted = if kind == SourceUriPathKind::WasiGuest {
            path.has_root()
        } else {
            path.is_absolute()
        };
        if !rooted {
            return Err(crate::file_uri::FileUriPathUnavailable::UnsupportedNativeGeometry.into());
        }
        Ok(SourcePathOrigin {
            path: path.to_path_buf(),
            kind,
            issuer: SourcePathIssuer::Filesystem,
        })
    }

    fn validate_source_path_origin(
        &self,
        origin: &SourcePathOrigin,
    ) -> Result<(), SourceUriPathUnavailable> {
        let kind = if cfg!(target_os = "wasi") {
            SourceUriPathKind::WasiGuest
        } else if cfg!(any(unix, windows)) {
            SourceUriPathKind::Native
        } else {
            return Err(SourceUriPathUnavailable::ProviderUnavailable);
        };
        if origin.kind != kind || !matches!(origin.issuer, SourcePathIssuer::Filesystem) {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        }
        Ok(())
    }

    fn validate_document_uri_path(
        &self,
        selected: &SourceUriPath,
    ) -> Result<(), SourceUriPathUnavailable> {
        let kind = if cfg!(target_os = "wasi") {
            SourceUriPathKind::WasiGuest
        } else if cfg!(any(unix, windows)) {
            SourceUriPathKind::Native
        } else {
            return Err(SourceUriPathUnavailable::ProviderUnavailable);
        };
        if selected.kind != kind || !matches!(selected.selection, SourceUriSelection::Filesystem) {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        }
        selected.check_current()
    }

    fn with_current_document_paths(
        &self,
        selected: &[&SourceUriPath],
        publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        for path in selected {
            self.validate_document_uri_path(path)?;
        }
        publish();
        Ok(())
    }

    fn with_current_document_sources(
        &self,
        selected: &[(&SourceUriPath, &[u8])],
        publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        for (path, expected) in selected {
            self.validate_document_uri_path(path)?;
            if self.read(path.path()).ok().as_deref() != Some(*expected) {
                return Err(SourceUriPathUnavailable::SourceImageUnavailable);
            }
        }
        publish();
        Ok(())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(path)?.flatten() {
            let kind = entry.file_type();
            out.push(DirEntry {
                path: entry.path(),
                is_dir: kind.as_ref().is_ok_and(std::fs::FileType::is_dir),
                is_file: kind.as_ref().is_ok_and(std::fs::FileType::is_file),
            });
        }
        Ok(out)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let meta = std::fs::metadata(path)?;
        Ok(Metadata {
            is_dir: meta.is_dir(),
            len: meta.len(),
        })
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

/// An in-memory filesystem the host fills in.
///
/// Two tables. `files` maps a path to its bytes and is what every
/// [`SourceStore`] method consults. `aliases` maps a URI string to the path it
/// was registered under, because the two spellings of one file — the client's
/// URI and the path the server re-derives from its own canonical URI — are not
/// guaranteed to round-trip through `Uri::to_file_path` on every platform. A
/// host that registers content by URI can therefore always find it again.
///
/// Directories are implicit: a path is a directory exactly when some stored
/// file is under it, which is enough for [`SourceStore::read_dir`] and
/// [`SourceStore::is_dir`] and avoids the host having to declare a tree.
#[derive(Debug)]
pub struct MemoryStore {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    aliases: Arc<UriAliases>,
    session_id: u64,
    source_revision: AtomicU64,
    source_inventories: Mutex<HashMap<PathBuf, WorkspaceSourceInventoryReceipt>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
            aliases: Arc::new(UriAliases::default()),
            session_id: NEXT_MEMORY_STORE_SESSION.fetch_add(1, Ordering::Relaxed),
            source_revision: AtomicU64::new(0),
            source_inventories: Mutex::new(HashMap::new()),
        }
    }
}

impl MemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a file's bytes.
    pub fn upsert(&self, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) {
        let path = path.into();
        let mut files = Self::lock(&self.files);
        files.insert(path, contents.into());
        self.source_revision.fetch_add(1, Ordering::Release);
    }

    /// Add or replace a file's bytes, and record `uri` as another name for it.
    pub fn upsert_uri(&self, uri: &str, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) {
        self.upsert_uri_aliases(&[uri], path, contents);
    }

    /// Register the actual original and canonical client aliases as one
    /// immutable path binding. Replacing/withdrawing it retires all its aliases;
    /// a bytes-only update of the same complete binding preserves its receipt.
    pub fn upsert_uri_aliases(
        &self,
        original_aliases: &[&str],
        path: impl Into<PathBuf>,
        contents: impl Into<Vec<u8>>,
    ) {
        let path = path.into();
        let mut names: Vec<String> = original_aliases
            .iter()
            .map(|uri| (*uri).to_owned())
            .collect();
        names.sort();
        names.dedup();
        if !names.is_empty() {
            let mut aliases = Self::lock(&self.aliases);
            let retained = aliases
                .get(&names[0])
                .filter(|binding| {
                    binding.path.as_os_str() == path.as_os_str()
                        && binding.aliases == names
                        && names.iter().all(|name| {
                            aliases
                                .get(name)
                                .is_some_and(|current| Arc::ptr_eq(current, binding))
                        })
                })
                .cloned();
            let binding = retained.unwrap_or_else(|| {
                let previous: Vec<_> = names
                    .iter()
                    .filter_map(|name| aliases.get(name).cloned())
                    .collect();
                aliases.retain(|_, row| !previous.iter().any(|old| Arc::ptr_eq(old, row)));
                Arc::new(UriAliasBinding {
                    path: path.clone(),
                    aliases: names.clone(),
                })
            });
            for name in names {
                aliases.insert(name, Arc::clone(&binding));
            }
        }
        let mut files = Self::lock(&self.files);
        files.insert(path, contents.into());
        self.source_revision.fetch_add(1, Ordering::Release);
    }

    /// Forget a file. Returns whether anything was stored under `path`.
    pub fn remove(&self, path: &Path) -> bool {
        let mut files = Self::lock(&self.files);
        let removed = files.remove(path).is_some();
        if removed {
            self.source_revision.fetch_add(1, Ordering::Release);
        }
        removed
    }

    /// Record the host's census for one admitted root after all preceding
    /// byte upserts/deletes. The receipt captures the current session revision.
    pub fn set_workspace_source_inventory(
        &self,
        root: impl Into<PathBuf>,
        selection: impl Into<String>,
        completeness: WorkspaceSourceInventoryCompleteness,
    ) {
        let files = Self::lock(&self.files);
        let revision = self.source_revision.load(Ordering::Acquire);
        let root = root.into();
        let receipt = WorkspaceSourceInventoryReceipt {
            session_id: self.session_id,
            root: root.clone(),
            source_revision: revision,
            selection: selection.into(),
            completeness,
        };
        Self::lock(&self.source_inventories).insert(root, receipt);
        drop(files);
    }

    /// Drop a root receipt when its workspace folder leaves the session.
    pub fn clear_workspace_source_inventory(&self, root: &Path) {
        Self::lock(&self.source_inventories).remove(root);
    }

    /// Forget the file registered under `uri`, whatever path it was stored at.
    /// Returns whether anything was removed.
    pub fn remove_uri(&self, uri: &str) -> bool {
        let binding = {
            let mut aliases = Self::lock(&self.aliases);
            let binding = aliases.remove(uri);
            if let Some(binding) = &binding {
                aliases.retain(|_, row| !Arc::ptr_eq(row, binding));
                // URI-to-path membership is part of the host's scanned source
                // view even when the byte map no longer has that file.
                self.source_revision.fetch_add(1, Ordering::Release);
            }
            binding
        };
        binding.is_some_and(|binding| self.remove(&binding.path))
    }

    fn registered_uri_binding(&self, uri: &str) -> Option<Arc<UriAliasBinding>> {
        Self::lock(&self.aliases).get(uri).cloned()
    }

    /// The exact path the host registered, without deriving URI/path identity.
    /// The typed SourceStore operation also retains its issuer and current binding.
    #[must_use]
    pub fn path_for_uri(&self, uri: &str) -> Option<PathBuf> {
        self.registered_uri_binding(uri)
            .map(|binding| binding.path.clone())
    }

    /// How many files the store holds.
    #[must_use]
    pub fn len(&self) -> usize {
        Self::lock(&self.files).len()
    }

    /// Whether the store holds no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Take a lock, recovering from a poisoned one.
    ///
    /// A poisoned lock here means a panic while the map was borrowed. The map
    /// is a plain `HashMap` with no cross-entry invariant, so the worst a
    /// poisoning can have left behind is a half-finished insert — nothing that
    /// makes the remaining entries wrong. Refusing to serve any file at all
    /// after one panic would be the larger failure.
    fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn not_found(path: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not in the in-memory source store", path.display()),
        )
    }
}

impl MemoryStore {
    fn validate_locked_document_paths(
        &self,
        aliases: &HashMap<String, Arc<UriAliasBinding>>,
        selected: &[&SourceUriPath],
    ) -> Result<(), SourceUriPathUnavailable> {
        for path in selected {
            let SourceUriSelection::Memory { owner, binding } = &path.selection else {
                return Err(SourceUriPathUnavailable::ForeignProvider);
            };
            if path.kind != SourceUriPathKind::Virtual
                || !Weak::ptr_eq(owner, &Arc::downgrade(&self.aliases))
            {
                return Err(SourceUriPathUnavailable::ForeignProvider);
            }
            match (aliases.get(path.original_uri()), binding) {
                (None, None) => {}
                (Some(current), Some(retained)) if Arc::ptr_eq(current, retained) => {}
                _ => return Err(SourceUriPathUnavailable::WithdrawnMapping),
            }
        }
        Ok(())
    }
}

impl SourceStore for MemoryStore {
    fn workspace_source_inventory_provider(&self) -> WorkspaceSourceInventoryProvider {
        WorkspaceSourceInventoryProvider::HostReported
    }

    fn workspace_source_inventory(&self, root: &Path) -> WorkspaceSourceInventoryObservation {
        let files = Self::lock(&self.files);
        let current_revision = self.source_revision.load(Ordering::Acquire);
        let receipt = Self::lock(&self.source_inventories).get(root).cloned();
        drop(files);
        let Some(receipt) = receipt else {
            return WorkspaceSourceInventoryObservation::Missing {
                session_id: self.session_id,
                root: root.to_path_buf(),
                source_revision: current_revision,
            };
        };
        if receipt.session_id != self.session_id || receipt.root != root {
            return WorkspaceSourceInventoryObservation::Stale {
                receipt,
                current_revision,
            };
        }
        if receipt.source_revision != current_revision {
            WorkspaceSourceInventoryObservation::Stale {
                receipt,
                current_revision,
            }
        } else {
            WorkspaceSourceInventoryObservation::Current(receipt)
        }
    }
    fn document_uri_path(
        &self,
        original_uri: &str,
    ) -> Result<SourceUriPath, SourceUriPathUnavailable> {
        // Strictly validate the entire URI even when an exact registered alias
        // selects a different virtual path. Raw URI data never receives lossy decoding.
        let derived = crate::file_uri::to_source_namespace_path(
            original_uri,
            crate::file_uri::SourceFileUriNamespace::Virtual,
        )?;
        let binding = self.registered_uri_binding(original_uri);
        let path = binding
            .as_ref()
            .map_or(derived, |binding| binding.path.clone());
        Ok(SourceUriPath {
            original_uri: Arc::from(original_uri),
            path,
            kind: SourceUriPathKind::Virtual,
            selection: SourceUriSelection::Memory {
                owner: Arc::downgrade(&self.aliases),
                binding,
            },
        })
    }

    fn source_path_origin(
        &self,
        path: &Path,
    ) -> Result<SourcePathOrigin, SourceUriPathUnavailable> {
        Ok(SourcePathOrigin {
            path: path.to_path_buf(),
            kind: SourceUriPathKind::Virtual,
            issuer: SourcePathIssuer::Memory {
                owner: Arc::downgrade(&self.aliases),
            },
        })
    }

    fn validate_source_path_origin(
        &self,
        origin: &SourcePathOrigin,
    ) -> Result<(), SourceUriPathUnavailable> {
        let SourcePathIssuer::Memory { owner } = &origin.issuer else {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        };
        if origin.kind != SourceUriPathKind::Virtual
            || !Weak::ptr_eq(owner, &Arc::downgrade(&self.aliases))
        {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        }
        owner
            .upgrade()
            .ok_or(SourceUriPathUnavailable::ProviderUnavailable)?;
        Ok(())
    }

    fn validate_document_uri_path(
        &self,
        selected: &SourceUriPath,
    ) -> Result<(), SourceUriPathUnavailable> {
        let SourceUriSelection::Memory { owner, .. } = &selected.selection else {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        };
        if selected.kind != SourceUriPathKind::Virtual
            || !Weak::ptr_eq(owner, &Arc::downgrade(&self.aliases))
        {
            return Err(SourceUriPathUnavailable::ForeignProvider);
        }
        selected.check_current()
    }

    fn with_current_document_paths(
        &self,
        selected: &[&SourceUriPath],
        publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        let aliases = Self::lock(&self.aliases);
        self.validate_locked_document_paths(&aliases, selected)?;
        publish();
        Ok(())
    }

    fn with_current_document_sources(
        &self,
        selected: &[(&SourceUriPath, &[u8])],
        publish: &mut dyn FnMut(),
    ) -> Result<(), SourceUriPathUnavailable> {
        let aliases = Self::lock(&self.aliases);
        let paths = selected.iter().map(|(path, _)| *path).collect::<Vec<_>>();
        self.validate_locked_document_paths(&aliases, &paths)?;
        let files = Self::lock(&self.files);
        for (path, expected) in selected {
            if files.get(path.path()).map(Vec::as_slice) != Some(*expected) {
                return Err(SourceUriPathUnavailable::SourceImageUnavailable);
            }
        }
        publish();
        Ok(())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        Self::lock(&self.files)
            .get(path)
            .cloned()
            .ok_or_else(|| Self::not_found(path))
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let files = Self::lock(&self.files);
        let mut seen: HashMap<PathBuf, bool> = HashMap::new();
        for stored in files.keys() {
            let Ok(rest) = stored.strip_prefix(path) else {
                continue;
            };
            let mut parts = rest.components();
            let Some(first) = parts.next() else { continue };
            let child = path.join(first.as_os_str());
            let is_dir = parts.next().is_some();
            seen.entry(child)
                .and_modify(|d| *d = *d || is_dir)
                .or_insert(is_dir);
        }
        if seen.is_empty() && !files.keys().any(|stored| stored.starts_with(path)) {
            return Err(Self::not_found(path));
        }
        Ok(seen
            .into_iter()
            .map(|(path, is_dir)| DirEntry {
                path,
                is_dir,
                is_file: !is_dir,
            })
            .collect())
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let files = Self::lock(&self.files);
        if let Some(bytes) = files.get(path) {
            return Ok(Metadata {
                is_dir: false,
                len: bytes.len() as u64,
            });
        }
        if files.keys().any(|stored| stored.starts_with(path)) {
            return Ok(Metadata {
                is_dir: true,
                len: 0,
            });
        }
        Err(Self::not_found(path))
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        self.upsert(path, contents.to_vec());
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        // Directories are implied by the files under them, so there is nothing
        // to create and nothing that can fail.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MemoryStore, NativeStore, SourceStore as _, WorkspaceSourceInventoryCompleteness,
        WorkspaceSourceInventoryObservation,
    };
    use std::path::Path;

    #[test]
    fn a_stored_file_reads_back() {
        let store = MemoryStore::new();
        store.upsert("/w/a.tcl", b"puts hi\n".to_vec());
        assert_eq!(
            store.read_to_string(Path::new("/w/a.tcl")).expect("stored"),
            "puts hi\n"
        );
        assert!(store.is_file(Path::new("/w/a.tcl")));
        assert!(!store.is_dir(Path::new("/w/a.tcl")));
    }

    #[test]
    fn a_host_inventory_receipt_is_bound_to_the_current_store_revision() {
        let store = MemoryStore::new();
        let root = Path::new("/workspace");
        store.upsert("/workspace/a.tcl", b"puts a\n".to_vec());
        store.set_workspace_source_inventory(
            root,
            r#"{"globs":["**/*.tcl"],"maxFiles":8}"#,
            WorkspaceSourceInventoryCompleteness::Complete,
        );
        let first = store.workspace_source_inventory(root);
        assert!(first.admits_complete_inventory());

        store.upsert("/workspace/b.tcl", b"puts b\n".to_vec());
        assert!(matches!(
            store.workspace_source_inventory(root),
            WorkspaceSourceInventoryObservation::Stale { .. }
        ));
        store.set_workspace_source_inventory(
            root,
            r#"{"globs":["**/*.tcl"],"maxFiles":8}"#,
            WorkspaceSourceInventoryCompleteness::Incomplete,
        );
        assert!(
            !store
                .workspace_source_inventory(root)
                .admits_complete_inventory()
        );
    }

    #[test]
    fn a_missing_file_is_not_found_rather_than_a_panic() {
        let store = MemoryStore::new();
        let err = store.read(Path::new("/w/gone.tcl")).expect_err("missing");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn an_ill_formed_byte_still_decodes_through_read_source() {
        let store = MemoryStore::new();
        store.upsert("/w/latin.tcl", vec![b'p', 0xFF, b'\n']);
        let (text, report) = store
            .read_source(Path::new("/w/latin.tcl"))
            .expect("stored");
        assert!(text.contains('p'));
        assert!(!report.is_faithful());
    }

    #[test]
    fn a_uri_alias_finds_the_file_again() {
        let store = MemoryStore::new();
        store.upsert_uri("file:///w/a.tcl", "/w/a.tcl", b"set x 1\n".to_vec());
        assert_eq!(
            store.path_for_uri("file:///w/a.tcl").expect("alias"),
            Path::new("/w/a.tcl")
        );
        assert!(store.remove_uri("file:///w/a.tcl"));
        assert!(!store.exists(Path::new("/w/a.tcl")));
    }

    #[test]
    fn directories_are_implied_by_the_files_under_them() {
        let store = MemoryStore::new();
        store.upsert("/w/lib/a.tcl", b"".to_vec());
        store.upsert("/w/b.tcl", b"".to_vec());
        assert!(store.is_dir(Path::new("/w")));
        let mut names: Vec<_> = store
            .read_dir(Path::new("/w"))
            .expect("listing")
            .into_iter()
            .map(|e| (e.path, e.is_dir))
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                (Path::new("/w/b.tcl").to_path_buf(), false),
                (Path::new("/w/lib").to_path_buf(), true),
            ]
        );
    }

    #[test]
    fn a_write_is_readable_again() {
        let store = MemoryStore::new();
        store
            .write(Path::new("/cfg/config.ini"), b"[features]\n")
            .expect("write");
        assert_eq!(
            store
                .read_to_string(Path::new("/cfg/config.ini"))
                .expect("written"),
            "[features]\n"
        );
    }

    #[test]
    fn the_native_store_is_std_fs() {
        let dir = std::env::temp_dir().join("tcl-lsp-vfs-native-store");
        let store = NativeStore;
        store.create_dir_all(&dir).expect("mkdir");
        let file = dir.join("probe.tcl");
        store.write(&file, b"proc p {} {}\n").expect("write");
        assert_eq!(store.read_to_string(&file).expect("read"), "proc p {} {}\n");
        assert!(store.is_dir(&dir));
        assert!(
            store
                .read_dir(&dir)
                .expect("listing")
                .iter()
                .any(|e| e.path == file)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod document_uri_path_tests {
    use super::*;

    #[test]
    fn virtual_issuer_keeps_authority_counted_data_and_refuses_invalid_uri() {
        let store = MemoryStore::new();
        let selected = store
            .document_uri_path("file://Srv/share/%C3%A9%252F.tcl")
            .unwrap();
        assert_eq!(selected.kind(), SourceUriPathKind::Virtual);
        assert_eq!(selected.path(), Path::new("//Srv/share/é%2F.tcl"));
        store.validate_document_uri_path(&selected).unwrap();
        for raw in [
            "file:///a%FF",
            "file:///a%00",
            "file:///a%2Fb",
            "file://user@host/a",
            "file:///a?query",
        ] {
            store.upsert_uri(raw, "/registered/does-not-forgive.tcl", Vec::new());
            assert!(store.document_uri_path(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn registration_is_stronger_and_old_receipt_cannot_revive_or_cross_provider() {
        let store = MemoryStore::new();
        let uri = "file:///browser/a.tcl";
        let derived = store.document_uri_path(uri).unwrap();
        store.upsert_uri(uri, derived.path(), b"one".to_vec());
        assert_eq!(
            store.validate_document_uri_path(&derived),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
        let first = store.document_uri_path(uri).unwrap();
        store.upsert_uri(uri, first.path(), b"two".to_vec());
        store.validate_document_uri_path(&first).unwrap();
        store.upsert_uri(uri, "/different/a.tcl", b"three".to_vec());
        store.upsert_uri(uri, first.path(), b"four".to_vec());
        assert_eq!(
            store.validate_document_uri_path(&first),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
        let foreign = MemoryStore::new();
        foreign.upsert_uri(uri, first.path(), b"four".to_vec());
        assert_eq!(
            foreign.validate_document_uri_path(&first),
            Err(SourceUriPathUnavailable::ForeignProvider)
        );
        let current = store.document_uri_path(uri).unwrap();
        drop(store);
        assert_eq!(
            current.check_current(),
            Err(SourceUriPathUnavailable::ProviderUnavailable)
        );
    }

    #[test]
    fn original_and_service_aliases_share_actual_row_and_withdraw_together() {
        let store = MemoryStore::new();
        let raw = "file:///browser/%61.tcl";
        let normalized = "file:///browser/a.tcl";
        store.upsert_uri_aliases(&[raw, normalized], "/actual/mounted.tcl", b"body".to_vec());
        let first = store.document_uri_path(raw).unwrap();
        let second = store.document_uri_path(normalized).unwrap();
        assert_eq!(first.path(), second.path());
        let mut calls = 0;
        store
            .with_current_document_paths(&[&first, &second], &mut || calls += 1)
            .unwrap();
        assert_eq!(calls, 1);
        store.upsert_uri_aliases(&[raw, normalized], first.path(), b"edited".to_vec());
        store.validate_document_uri_path(&first).unwrap();
        assert!(store.remove_uri(normalized));
        assert_eq!(
            store.validate_document_uri_path(&first),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
        assert!(store.path_for_uri(raw).is_none());
        assert_eq!(
            store.with_current_document_paths(&[&first], &mut || calls += 1),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn publication_holds_the_actual_mapping_until_continuation_returns() {
        let store = Arc::new(MemoryStore::new());
        let uri = "file:///browser/leased.tcl";
        store.upsert_uri(uri, "/actual/first.tcl", Vec::new());
        let selected = store.document_uri_path(uri).unwrap();
        let (attempt_tx, attempt_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let other = Arc::clone(&store);
        let mut worker = None;
        store
            .with_current_document_paths(&[&selected], &mut || {
                let other = Arc::clone(&other);
                let attempt_tx = attempt_tx.clone();
                let done_tx = done_tx.clone();
                worker = Some(std::thread::spawn(move || {
                    attempt_tx.send(()).unwrap();
                    other.upsert_uri(uri, "/actual/second.tcl", Vec::new());
                    done_tx.send(()).unwrap();
                }));
                attempt_rx.recv().unwrap();
                assert!(matches!(
                    done_rx.try_recv(),
                    Err(std::sync::mpsc::TryRecvError::Empty)
                ));
            })
            .unwrap();
        worker.take().unwrap().join().unwrap();
        done_rx.recv().unwrap();
        assert_eq!(
            store.validate_document_uri_path(&selected),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
    }

    #[cfg(unix)]
    #[test]
    fn different_counted_path_spelling_is_a_new_registration() {
        let store = MemoryStore::new();
        let uri = "file:///browser/counted.tcl";
        store.upsert_uri(uri, "/actual//counted.tcl", b"one".to_vec());
        let original = store.document_uri_path(uri).unwrap();
        assert_eq!(
            original.path().as_os_str(),
            std::ffi::OsStr::new("/actual//counted.tcl")
        );
        store.upsert_uri(uri, "/actual/counted.tcl", b"two".to_vec());
        assert_eq!(
            store.validate_document_uri_path(&original),
            Err(SourceUriPathUnavailable::WithdrawnMapping)
        );
        let current = store.document_uri_path(uri).unwrap();
        assert_eq!(
            current.path().as_os_str(),
            std::ffi::OsStr::new("/actual/counted.tcl")
        );
    }

    #[cfg(target_os = "wasi")]
    #[test]
    fn actual_wasi_store_issues_guest_path_without_native_host_claim() {
        let selected = NativeStore
            .document_uri_path("file:///workspace/a.tcl")
            .unwrap();
        assert_eq!(selected.kind(), SourceUriPathKind::WasiGuest);
        assert_eq!(selected.path(), Path::new("/workspace/a.tcl"));
        NativeStore.validate_document_uri_path(&selected).unwrap();
    }
}

#[cfg(test)]
mod actual_source_path_origin_tests {
    use super::*;

    #[test]
    fn virtual_original_path_is_provider_issued_and_never_a_native_path_grant() {
        let actual = MemoryStore::new();
        let path = Path::new("/virtual//MiXeD/source.tcl");
        let origin = actual.source_path_origin(path).unwrap();
        assert_eq!(origin.path().as_os_str(), path.as_os_str());
        assert_eq!(origin.kind(), SourceUriPathKind::Virtual);
        actual.validate_source_path_origin(&origin).unwrap();
        assert_eq!(
            MemoryStore::new().validate_source_path_origin(&origin),
            Err(SourceUriPathUnavailable::ForeignProvider)
        );
        assert!(NativeStore.validate_source_path_origin(&origin).is_err());
    }

    #[cfg(all(not(target_os = "wasi"), any(unix, windows)))]
    #[test]
    fn actual_native_source_path_keeps_original_spelling_without_uri_reconstruction() {
        let path = std::env::temp_dir().join("MiXeD-source-path.tcl");
        let origin = NativeStore.source_path_origin(&path).unwrap();
        assert_eq!(origin.path().as_os_str(), path.as_os_str());
        assert_eq!(origin.kind(), SourceUriPathKind::Native);
        NativeStore.validate_source_path_origin(&origin).unwrap();
        assert_eq!(
            MemoryStore::new().validate_source_path_origin(&origin),
            Err(SourceUriPathUnavailable::ForeignProvider)
        );
        assert!(
            NativeStore
                .source_path_origin(Path::new("relative-source.tcl"))
                .is_err()
        );
    }

    #[cfg(target_os = "wasi")]
    #[test]
    fn actual_wasi_source_path_is_a_guest_path_origin() {
        let path = Path::new("/workspace/MiXeD-source.tcl");
        let origin = NativeStore.source_path_origin(path).unwrap();
        assert_eq!(origin.kind(), SourceUriPathKind::WasiGuest);
        assert_eq!(origin.path().as_os_str(), path.as_os_str());
        NativeStore.validate_source_path_origin(&origin).unwrap();
    }
}

#[cfg(test)]
mod saved_source_publication_tests {
    use super::*;

    #[test]
    fn selected_memory_source_holds_mapping_and_bytes_and_refuses_in_place_change() {
        let store = MemoryStore::new();
        let uri = "file:///studio/active.tclspec";
        store.upsert_uri(uri, "/studio/active.tclspec", b"original".to_vec());
        let selected = store.document_uri_path(uri).unwrap();
        let mut published = 0;
        store
            .with_current_document_sources(&[(&selected, b"original")], &mut || {
                assert!(store.aliases.try_lock().is_err());
                assert!(store.files.try_lock().is_err());
                published += 1;
            })
            .unwrap();
        assert_eq!(published, 1);
        store.upsert("/studio/active.tclspec", b"changed".to_vec());
        store.validate_document_uri_path(&selected).unwrap();
        assert_eq!(
            store
                .with_current_document_sources(&[(&selected, b"original")], &mut || published += 1),
            Err(SourceUriPathUnavailable::SourceImageUnavailable)
        );
        assert_eq!(published, 1);
        store
            .with_current_document_sources(&[(&selected, b"changed")], &mut || published += 1)
            .unwrap();
        assert_eq!(published, 2);
    }
}
