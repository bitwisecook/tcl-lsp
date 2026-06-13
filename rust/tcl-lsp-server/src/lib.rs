//! Native LSP server backend for Tcl.
//!
//! Exposes a [`Backend`] that implements [`tower_lsp::LanguageServer`]
//! and is wrapped in an `LspService` by the binary. This crate is the
//! second consumer of [`tcl_lsp_core`] (the first is `tcl-lsp-rust`),
//! so the pure-Rust crate boundary now has both production drivers
//! exercising it.
//!
//! ARCH8 ships the bootstrap with the folding-range provider wired
//! end-to-end; the `S-document-symbols` follow-up adds the
//! document-symbol provider on top.  Every other LSP method returns
//! [`tower_lsp::jsonrpc::ErrorCode::MethodNotFound`] until later
//! chunks extend the provider set.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_lsp_core::bigip as core_bigip;
use tcl_lsp_core::call_hierarchy as core_call_hierarchy;
use tcl_lsp_core::code_actions as core_code_actions;
use tcl_lsp_core::code_lens as core_code_lens;
use tcl_lsp_core::completion::{
    self as core_completion, CompletionItem as CoreCompletionItem,
    CompletionKind as CoreCompletionKind,
};
use tcl_lsp_core::declaration as core_declaration;
use tcl_lsp_core::definition::{self as core_definition, LspRange as CoreLspRange};
use tcl_lsp_core::document_links as core_document_links;
use tcl_lsp_core::document_symbols::{self as core_symbols, SymbolKind as CoreSymbolKind};
use tcl_lsp_core::file_ops as core_file_ops;
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::formatting as core_formatting;
use tcl_lsp_core::hover::{self as core_hover, Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::implementation as core_implementation;
use tcl_lsp_core::inlay_hints as core_inlay_hints;
use tcl_lsp_core::linked_editing_range as core_linked_editing_range;
use tcl_lsp_core::minify as core_minify;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::selection_range as core_selection_range;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help::{
    self as core_sig, ParameterInformation as CoreParameterInformation,
    SignatureHelp as CoreSignatureHelp, SignatureInformation as CoreSignatureInformation,
};
use tcl_lsp_core::type_definition as core_type_definition;
use tcl_lsp_core::workspace_index as core_workspace_index;
// type_hierarchy core provider lands when tower-lsp's
// LanguageServer trait exposes the type-hierarchy methods.
// Module is registered in `tcl_lsp_core` for downstream
// callers; intentionally not imported here yet.
// use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
use tcl_lsp_core::workspace_symbols::{
    self as core_workspace_symbols, WorkspaceSymbolKind as CoreWorkspaceSymbolKind,
};
use tcl_registry::dialects::DialectSet;
use tcl_registry::CommandRegistry;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    request::{
        GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
        GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
    },
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CallHierarchyServerCapability, CodeAction, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeLens, CodeLensOptions, CodeLensParams, CompletionItem,
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse, ConfigurationItem,
    DeclarationCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentChanges, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, DocumentFormattingParams, DocumentHighlight,
    DocumentHighlightKind, DocumentHighlightParams, DocumentLink, DocumentLinkOptions,
    DocumentLinkParams, DocumentRangeFormattingParams, DocumentSymbol, DocumentSymbolParams,
    DocumentSymbolResponse, Documentation, ExecuteCommandOptions, ExecuteCommandParams,
    FileOperationFilter, FileOperationPattern, FileOperationRegistrationOptions, FoldingRange,
    FoldingRangeKind, FoldingRangeParams, FoldingRangeProviderCapability,
    FullDocumentDiagnosticReport, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, ImplementationProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, InlayHint, InlayHintKind,
    InlayHintLabel, InlayHintParams, LinkedEditingRangeParams, LinkedEditingRanges, Location,
    MarkupContent, MarkupKind, MessageType, OneOf, OptionalVersionedTextDocumentIdentifier,
    ParameterInformation, ParameterLabel, Position, PrepareRenameResponse, Range, ReferenceParams,
    RelatedFullDocumentDiagnosticReport, RenameFilesParams, RenameOptions, RenameParams,
    SelectionRange, SelectionRangeParams, SelectionRangeProviderCapability,
    SemanticTokens as LspSemanticTokens, SemanticTokensDelta, SemanticTokensDeltaParams,
    SemanticTokensEdit, SemanticTokensFullDeltaResult, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams, SemanticTokensRangeParams,
    SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    ServerCapabilities, ServerInfo, SignatureHelp, SignatureHelpOptions, SignatureHelpParams,
    SignatureInformation, SymbolInformation, SymbolKind, TextDocumentEdit,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
    TypeDefinitionProviderCapability, Url, WorkDoneProgressOptions, WorkspaceEdit,
    WorkspaceFileOperationsServerCapabilities, WorkspaceServerCapabilities, WorkspaceSymbolParams,
};
use tower_lsp::{Client, LanguageServer};

/// Document store value: source text + dialect string.
#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    dialect: String,
    /// Monotonic local revision.  Incremented before each
    /// asynchronous analyser run so stale workers cannot publish
    /// analysis/cache state for an older edit.
    revision: u64,
    /// Last LSP document version reported by the client.  Forwarded
    /// with published diagnostics so clients can discard obsolete
    /// diagnostics if messages arrive out of order.
    version: Option<i32>,
}

impl DocumentState {
    fn new(text: String, dialect: String) -> Self {
        Self {
            text,
            dialect,
            revision: 0,
            version: None,
        }
    }

    fn with_version(text: String, dialect: String, version: i32) -> Self {
        Self {
            text,
            dialect,
            revision: 0,
            version: Some(version),
        }
    }

    fn bump_revision(&mut self, version: i32) {
        self.revision = self.revision.saturating_add(1);
        self.version = Some(version);
    }
}

/// LSP server backend.
///
/// Holds the LSP `Client` for outbound notifications, a document
/// store keyed on URL, and a per-dialect [`CommandRegistry`]
/// cache. Constructed once per LSP session by
/// [`tower_lsp::LspService::new`].
///
/// Each dialect gets its own cached registry built lazily on the
/// first request that names it — feature providers in
/// `tcl-lsp-core` take a `&CommandRegistry` so no per-request
/// rebuild is needed. The cache also owns the plain-Tcl base
/// registry under the empty-string key, so registries are dropped
/// with the `Backend` rather than leaked for the process lifetime.
pub struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, DocumentState>>,
    /// Serialises the critical section that couples the live document
    /// revision to the derived analysis cache, workspace index, hover
    /// cache, semantic-token cache, and diagnostics publication.
    ///
    /// The expensive analyser work still runs outside this lock.  The
    /// gate only covers revision checks and state publication, so an
    /// older worker cannot finish late and overwrite state for newer
    /// text.
    document_analysis_gate: Mutex<()>,
    /// Fallback dialect string used when ``did_open`` cannot derive
    /// one from the ``languageId`` and no per-session
    /// ``workspace/didChangeConfiguration`` has been received yet.
    /// Updated by ``did_change_configuration`` so editor reconfigures
    /// take effect for subsequently-opened documents.
    default_dialect: Mutex<String>,
    dialect_registries: Mutex<HashMap<String, Arc<CommandRegistry>>>,
    /// Workspace folder roots received from `initialize` /
    /// `workspace/didChangeWorkspaceFolders`.  Stored as
    /// `Url` (typically `file://...` directories).
    /// `S-workspace-init` minimal port — the folder list
    /// supports cross-document features (workspace symbols
    /// already walks every cached document; future
    /// `S-workspace-symbols-rich` extends this by scanning
    /// folder contents on disk).
    workspace_folders: Mutex<Vec<Url>>,
    /// Optional per-folder dialect override map keyed on the
    /// folder URL prefix (typically `file://...`).  Populated
    /// from the `initializationOptions.folderDialects` JSON
    /// object when present.  `S-workspace-init` / `SYNC-MAY19-
    /// dialect-contextvar`: enables multi-folder workspaces
    /// with mixed dialects to parse correctly by selecting the
    /// longest-prefix folder's dialect when a document is
    /// opened.
    folder_dialects: Mutex<Vec<(Url, String)>>,
    /// W108 non-ASCII detection mode (`tclLsp.style.nonAscii`).
    /// [`NonAsciiMode::Default`] until an editor configures it via
    /// `initializationOptions` or `workspace/didChangeConfiguration`.
    /// Threaded into every `Analyser` the diagnostics path builds.
    non_ascii_mode: Mutex<NonAsciiMode>,
    /// Diagnostic codes the user has disabled (`tclLsp.diagnostics.<CODE>
    /// = false`). Threaded into every analyser build so the disabled
    /// codes are filtered, and consulted by the source-style pass.
    disabled_diagnostics: Mutex<HashSet<String>>,
    /// Cached `AnalysisResult` per document, populated by
    /// `did_open` / `did_change` and consumed by request
    /// handlers that previously re-analysed on every call.
    /// `S-async-diagnostics` cached-analysis surface — once
    /// this entry exists, request handlers consult it before
    /// falling back to a fresh `analyser.analyse(...)`.
    analyses: Mutex<HashMap<Url, tcl_compiler::analyser::AnalysisResult>>,
    /// `S-hover-sync11`: LRU(256) cache of hover responses
    /// keyed on `(uri, line, character)`.  Invalidated per
    /// URI on `did_change` / `did_close` so stale answers
    /// don't outlive the document version that produced them.
    /// `None` entries record "no hover here" so repeated
    /// requests for the same empty position are also fast.
    hover_cache: Mutex<HoverCache>,
    /// Per-URI semantic-tokens delta cache.  Records the last
    /// `result_id` and packed token stream we returned for
    /// each document so `semanticTokens/full/delta` can short-
    /// circuit when nothing changed.  Invalidated on
    /// `did_change` / `did_close`.
    semantic_tokens_cache: Mutex<HashMap<Url, SemanticTokensEntry>>,
    /// Cross-document proc / class definition index, maintained
    /// incrementally as documents open / change / close.  Lets
    /// completion enumerate procs from sibling files and
    /// (later) cross-document go-to-definition resolve symbols
    /// defined elsewhere.
    workspace_index: Mutex<core_workspace_index::WorkspaceIndex>,
    /// Per-feature provider toggles (`tclLsp.features.*`).  Absent
    /// keys default to enabled, so a config that names only some
    /// features leaves the rest on.  Consulted by each provider
    /// entry point and surfaced by `getEffectiveConfig`.  Mirrors
    /// the Python server's `_FEATURE_TOGGLE_KEYS` resolution.
    feature_toggles: Mutex<FeatureToggles>,
    /// Optimiser master switch (`tclLsp.optimiser.enabled`).  When
    /// `false`, the `tcl-lsp.optimiseDocument` command yields no
    /// rewrites.  Default on.
    optimiser_enabled: Mutex<bool>,
    /// Optimisation profile (`tclLsp.optimiser.profile`) controlling which
    /// O-code categories surface as diagnostics. Default
    /// [`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]
    /// (`readability`).
    optimiser_profile: Mutex<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    /// Per-code optimiser overrides (`tclLsp.optimiser.<CODE>` = bool): a code
    /// mapped to `true` is force-*enabled* (removed from the profile's disabled
    /// set), `false` is force-*disabled* (added). Layered on top of the
    /// profile-derived set, mirroring Python's per-code `disabled_optimisations`
    /// adjustment in `settings.py`.
    optimiser_code_overrides: Mutex<HashMap<String, bool>>,
    /// Resolved formatter line length (`tclLsp.formatting.lineLength`).
    /// Surfaced by `getEffectiveConfig`; default 80.
    line_length: Mutex<u32>,
}

/// Resolved `tclLsp.features.*` toggle state.
///
/// Stores only the keys an editor has explicitly set; every other
/// feature resolves to enabled.  This matches the Python server's
/// "absent → default-on" semantics and the config-pull restore
/// contract (a pulled config only *sets* the keys it carries).
#[derive(Debug, Default, Clone)]
struct FeatureToggles {
    set: HashMap<String, bool>,
}

impl FeatureToggles {
    /// camelCase feature keys reported by `getEffectiveConfig`,
    /// mirroring Python's `_FEATURE_TOGGLE_KEYS`.
    const KEYS: &'static [&'static str] = &[
        "hover",
        "completion",
        "diagnostics",
        "semanticTokens",
        "codeActions",
        "definition",
        "references",
        "documentSymbols",
        "folding",
        "rename",
        "signatureHelp",
        "workspaceSymbols",
        "inlayHints",
        "callHierarchy",
        "documentLinks",
        "selectionRange",
        "documentHighlight",
        "codeLens",
        "implementation",
        "typeDefinition",
        "declaration",
        "linkedEditingRange",
    ];

    /// Whether `feature` is enabled — explicitly-set value, else
    /// the default-on fallback.
    fn is_enabled(&self, feature: &str) -> bool {
        self.set.get(feature).copied().unwrap_or(true)
    }

    /// Merge an editor-supplied `features` object, setting only the
    /// keys it carries (absent keys keep their last-applied value).
    fn apply(&mut self, features: &serde_json::Map<String, serde_json::Value>) {
        for (key, value) in features {
            if let Some(flag) = value.as_bool() {
                self.set.insert(key.clone(), flag);
            }
        }
    }

    /// The full resolved `{feature: bool}` map for `getEffectiveConfig`.
    fn resolved_map(&self) -> serde_json::Map<String, serde_json::Value> {
        Self::KEYS
            .iter()
            .map(|&k| (k.to_owned(), serde_json::Value::Bool(self.is_enabled(k))))
            .collect()
    }
}

/// Cached semantic-tokens result keyed on the URI.
#[derive(Debug, Clone)]
struct SemanticTokensEntry {
    /// `result_id` returned to the client; bumped on every
    /// recompute so clients can request deltas against the
    /// stored snapshot.
    result_id: String,
    /// Packed `(deltaLine, deltaCol, length, type, modifiers)`
    /// stream — same shape `tcl_lsp_core::semantic_tokens`
    /// emits.
    data: Vec<u32>,
}

/// Result of a `HoverCache::get` lookup.
#[derive(Debug, Clone)]
enum HoverLookup {
    /// Cache miss — the caller must compute the hover.
    Miss,
    /// Cached "no hover here" answer (the provider returned
    /// `None` for this position last time).
    HitEmpty,
    /// Cached hover response.
    Hit(tcl_lsp_core::hover::Hover),
}

/// LRU cache of hover responses bounded at 256 entries.
/// Keys are `(uri, line, character)`.  Stored values
/// distinguish between "no entry" and "entry with empty
/// hover" so repeated requests on positions that have no
/// hover are also fast.  On capacity overflow the oldest
/// entry is evicted.
#[derive(Debug, Default)]
struct HoverCache {
    /// FIFO queue of cache keys in insertion order.  Doubles
    /// as the eviction order — bounded LRU; reads don't
    /// promote.  256 is the SYNC11-mandated cap.
    order: std::collections::VecDeque<(Url, u32, u32)>,
    entries: HashMap<(Url, u32, u32), Option<tcl_lsp_core::hover::Hover>>,
}

impl HoverCache {
    const CAP: usize = 256;

    fn get(&self, key: &(Url, u32, u32)) -> HoverLookup {
        match self.entries.get(key) {
            None => HoverLookup::Miss,
            Some(None) => HoverLookup::HitEmpty,
            Some(Some(h)) => HoverLookup::Hit(h.clone()),
        }
    }

    fn put(&mut self, key: (Url, u32, u32), value: Option<tcl_lsp_core::hover::Hover>) {
        use std::collections::hash_map::Entry;
        match self.entries.entry(key.clone()) {
            Entry::Occupied(mut e) => {
                e.insert(value);
            }
            Entry::Vacant(slot) => {
                if self.order.len() >= Self::CAP {
                    if let Some(old) = self.order.pop_front() {
                        // The slot we're about to fill matches
                        // `key`; the popped key is different, so
                        // remove it via a direct lookup.
                        if old != key {
                            // The Vacant binding holds the entry,
                            // so we can't remove via the same map
                            // without giving it up.  Stash the key,
                            // drop the entry, then evict, then
                            // re-insert.
                            drop(slot);
                            self.entries.remove(&old);
                            self.order.push_back(key.clone());
                            self.entries.insert(key, value);
                            return;
                        }
                    }
                }
                slot.insert(value);
                self.order.push_back(key);
            }
        }
    }

    fn invalidate_uri(&mut self, uri: &Url) {
        self.order.retain(|(u, _, _)| u != uri);
        self.entries.retain(|(u, _, _), _| u != uri);
    }
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

impl Backend {
    /// Build a fresh backend wrapping the given client.
    ///
    /// Dialect-specific registries (including the plain-Tcl base)
    /// are loaded lazily from [`Backend::registry_for_dialect`] on
    /// the first request that names them. Pre-loading every
    /// dialect into one shared registry would let dialect-specific
    /// command overrides (e.g. iRules's `proc` shape) shadow the
    /// Tcl spec for requests on unrelated dialects.
    ///
    /// `default_dialect` is the dialect string forwarded to
    /// providers when the document store has no override.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            document_analysis_gate: Mutex::new(()),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            non_ascii_mode: Mutex::new(NonAsciiMode::Default),
            disabled_diagnostics: Mutex::new(HashSet::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
            workspace_index: Mutex::new(core_workspace_index::WorkspaceIndex::new()),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
        }
    }

    /// Read the cached `AnalysisResult` for `uri` if one
    /// exists.  Returns a clone so the caller can run on a
    /// `spawn_blocking` worker without holding the mutex.
    /// Falls back to `None` when no cache entry exists; the
    /// caller is expected to compute a fresh analysis in
    /// that case.
    async fn cached_analysis(&self, uri: &Url) -> Option<tcl_compiler::analyser::AnalysisResult> {
        self.analyses.lock().await.get(uri).cloned()
    }

    /// Resolve an analysis for the document.  Consults the
    /// cache first; computes a fresh analysis when no entry
    /// exists.  Returns owned data the caller can move into
    /// a `spawn_blocking` worker.
    async fn analysis_for(
        &self,
        uri: &Url,
        text: String,
        dialect: String,
    ) -> tcl_compiler::analyser::AnalysisResult {
        if let Some(cached) = self.cached_analysis(uri).await {
            return cached;
        }
        let (disabled, na_mode) = self.analyser_config().await;
        tokio::task::spawn_blocking(move || {
            let mut analyser = Self::configured_analyser(disabled, na_mode);
            analyser.analyse(&text, &dialect).clone()
        })
        .await
        .unwrap_or_default()
    }

    /// Return a snapshot of the current workspace folder
    /// URLs.  Used by cross-document features (workspace
    /// symbols, cross-doc references / rename / call-
    /// hierarchy).
    pub async fn workspace_folder_urls(&self) -> Vec<Url> {
        self.workspace_folders.lock().await.clone()
    }

    /// `S-workspace-init`: copy the workspace folders the
    /// editor sent into `self.workspace_folders` so cross-
    /// document features can resolve relative paths and (in
    /// the future) scan folder contents for a workspace index.
    /// Both the newer `workspace_folders` field and the
    /// single-root `root_uri` fallback are supported.
    async fn apply_workspace_folders(&self, params: &InitializeParams) {
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Url> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
    }

    /// `SYNC-MAY19-dialect-contextvar`: read per-folder dialect
    /// overrides from `params.initialization_options`.
    ///
    /// Expected shape:
    /// `{ "folderDialects": { "file:///path": "f5-irules",
    ///                        "file:///other": "tcl9.0" } }`.
    ///
    /// Unknown dialect names and unparseable folder URLs are
    /// dropped silently rather than failing the entire
    /// initialise — the server keeps starting up with whatever
    /// valid entries it could pull from the editor.
    async fn apply_initialization_options(&self, params: &InitializeParams) {
        let Some(opts) = &params.initialization_options else {
            return;
        };
        // `folderDialects` map (per-folder dialect overrides).
        if let Some(entries) = opts
            .as_object()
            .and_then(|m| m.get("folderDialects"))
            .and_then(serde_json::Value::as_object)
        {
            let mut parsed: Vec<(Url, String)> = Vec::new();
            for (folder_url, dialect_val) in entries {
                let Ok(url) = Url::parse(folder_url) else {
                    continue;
                };
                let Some(dialect) = dialect_val.as_str() else {
                    continue;
                };
                if DialectSet::parse(dialect).is_none() {
                    continue;
                }
                parsed.push((url, dialect.to_owned()));
            }
            *self.folder_dialects.lock().await = parsed;
        }
        // Diagnostic settings (`tclLsp.style.nonAscii` /
        // `tclLsp.diagnostics.<CODE>`), accepted in either the nested or
        // flat-dotted shape.
        if let Some(mode) = settings_non_ascii_mode(opts) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(opts) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
    }

    /// Resolve the dialect string a freshly opened document should
    /// be tagged with.
    ///
    /// Resolution order (`SYNC-MAY19-dialect-contextvar`):
    ///
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl9.0"``
    ///    / etc.), use it directly.
    /// 2. The per-folder override map (`folder_dialects`) — when
    ///    the document URI sits under one of the configured folder
    ///    URLs, use the deepest-matching folder's dialect.
    /// 3. The session-wide ``default_dialect`` fallback.
    async fn dialect_for_open(&self, uri: &Url, language_id: &str) -> String {
        let lang_dialect = Self::dialect_from_language_id(language_id);
        // A canonical BIG-IP basename (``bigip.conf``, ``bigip_base.conf``,
        // …) routes to ``f5-bigip`` ahead of a *generic* Tcl ``languageId``,
        // mirroring Python ``infer_document_dialect``: only an explicit
        // non-Tcl dialect id (``f5-irules`` / ``f5-iapps`` / ``expect`` /
        // EDA / ``tk``) wins over the basename. This is what lets the test
        // harness open ``bigip.conf`` with ``languageId: "tcl"`` and still
        // get the BIG-IP path (parse / outline + general-Tcl-diagnostic
        // suppression).
        let explicit_non_tcl = matches!(lang_dialect, Some(d) if !d.starts_with("tcl"));
        if !explicit_non_tcl && core_bigip::is_bigip_conf_name(uri.as_str()) {
            return "f5-bigip".to_owned();
        }
        if let Some(d) = lang_dialect {
            return d.to_owned();
        }
        if let Some(d) = self.resolve_folder_dialect(uri).await {
            return d;
        }
        self.default_dialect.lock().await.clone()
    }

    /// Whether `dialect` denotes an F5 BIG-IP config document — the
    /// signal every BIG-IP-specific branch (analysis suppression,
    /// config outline) keys on.
    fn is_bigip_dialect(dialect: &str) -> bool {
        dialect == "f5-bigip"
    }

    /// Look up the per-folder dialect override for `uri`,
    /// preferring the deepest (longest-prefix) match so nested
    /// folders shadow their parents.
    async fn resolve_folder_dialect(&self, uri: &Url) -> Option<String> {
        let folders = self.folder_dialects.lock().await;
        folder_dialect_for(uri, &folders)
    }

    /// Map an LSP ``languageId`` string to a dialect name accepted
    /// by ``DialectSet::parse`` / the providers' ``dialect`` arg.
    ///
    /// Recognises the editor-extension language ids
    /// (``tcl-irule`` → ``f5-irules``, etc.) plus the canonical
    /// dialect names used by the MCP bridge / direct callers
    /// (``f5-irules``, ``tcl9.0``, …).  Returns ``None`` when the
    /// language id does not name a known dialect — the caller falls
    /// back to the session default.
    fn dialect_from_language_id(language_id: &str) -> Option<&'static str> {
        // Group editor language ids alongside the canonical dialect
        // name they map to so callers in either world land on the
        // string the registry / provider trait expects.
        let mapped = match language_id {
            "tcl" | "tcl8.6" => "tcl8.6",
            "tcl8.4" => "tcl8.4",
            "tcl8.5" => "tcl8.5",
            "tcl9.0" => "tcl9.0",
            "tcl-irule" | "f5-irules" => "f5-irules",
            "tcl-iapp" | "f5-iapps" => "f5-iapps",
            "tcl-expect" | "expect" => "expect",
            "tcl-synopsys" | "synopsys-eda-tcl" => "synopsys-eda-tcl",
            "tcl-cadence" | "cadence-eda-tcl" => "cadence-eda-tcl",
            "tcl-xilinx" | "xilinx-eda-tcl" => "xilinx-eda-tcl",
            "tcl-quartus" | "intel-quartus-eda-tcl" => "intel-quartus-eda-tcl",
            "tcl-mentor" | "mentor-eda-tcl" => "mentor-eda-tcl",
            "tk" => "tk",
            _ => return None,
        };
        // Sanity-check the mapped string is something the registry
        // accepts; guards against typos in the table.
        debug_assert!(DialectSet::parse(mapped).is_some());
        Some(mapped)
    }

    async fn read_document(&self, url: &Url) -> Option<DocumentState> {
        if let Some(doc) = self.documents.lock().await.get(url).cloned() {
            return Some(doc);
        }
        // On-disk fallback: files the folder scan indexed but the
        // editor hasn't opened aren't in the open-document map.
        // Read them from disk so cross-document span→range
        // resolution (references / rename / call-hierarchy) can
        // reach their sources.  Non-`file://` URLs and unreadable
        // paths fall through to `None`.
        let path = url.to_file_path().ok()?;
        let text = tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
            .await
            .ok()?
            .ok()?;
        let dialect = match self.resolve_folder_dialect(url).await {
            Some(d) => d,
            None => self.default_dialect.lock().await.clone(),
        };
        Some(DocumentState::new(text, dialect))
    }

    /// Re-index a (now-closed) document from its on-disk contents so
    /// its definitions / calls remain visible to cross-document
    /// features.  `scan_workspace_folders` only runs at
    /// `initialized`, so on `did_close` we must refresh the index
    /// from disk rather than dropping the entry — otherwise opening
    /// then closing a file would erase it from cross-document
    /// definition / references / rename / call-hierarchy until the
    /// server restarts.  Falls back to removing the entry when the
    /// URI isn't a readable file (untitled buffer, deleted file).
    ///
    /// Disk IO and analysis deliberately run outside
    /// `document_analysis_gate`. The final index update rechecks that
    /// the URI is still closed, so a slow disk read cannot overwrite a
    /// newly reopened unsaved buffer.
    async fn reindex_index_from_disk(&self, uri: &Url) {
        let analysed: Option<AnalysisResult> = if let Ok(path) = uri.to_file_path() {
            let dialect = match self.resolve_folder_dialect(uri).await {
                Some(d) => d,
                None => self.default_dialect.lock().await.clone(),
            };
            tokio::task::spawn_blocking(move || {
                let text = std::fs::read_to_string(path).ok()?;
                Some(Analyser::new().analyse(&text, &dialect).clone())
            })
            .await
            .ok()
            .flatten()
        } else {
            None
        };
        let _gate = self.document_analysis_gate.lock().await;
        if self.documents.lock().await.contains_key(uri) {
            return;
        }
        let mut index = self.workspace_index.lock().await;
        index.remove_document(uri.as_str());
        if let Some(analysis) = analysed {
            index.add_document(uri.as_str(), &analysis);
        }
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    async fn compute_definition(&self, uri: &Url, pos: Position) -> jsonrpc::Result<Vec<Location>> {
        let Some(doc) = self.read_document(uri).await else {
            return Ok(Vec::new());
        };
        let analysis = self
            .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // The cross-document fallback should only fire when the cursor
        // sits on a command head (a call site), matching what the
        // in-document provider resolves.  Compute that here, while
        // `analysis` is still in scope (it is moved into the worker
        // below).  Without this gate, an argument word that happens to
        // collide with a sibling proc/class name produces a
        // false-positive go-to-definition jump.
        let on_command_head = position_is_command_head(&doc.text, pos, &analysis);
        let text = doc.text.clone();
        let in_doc = tokio::task::spawn_blocking(move || {
            core_definition::definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("definition worker panicked: {err}").into(),
            data: None,
        })?;
        if !in_doc.is_empty() {
            // Resolved within the current document.
            return Ok(in_doc
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect());
        }
        if !on_command_head {
            return Ok(Vec::new());
        }
        // Cross-document fallback: resolve a proc / class
        // defined in a sibling document via the workspace index.
        self.cross_document_definition(uri, &doc.text, pos).await
    }

    /// Resolve the symbol at `pos` against the workspace index
    /// when the current document has no local definition.  Only
    /// fires on bare command words (not `$var` references), and
    /// returns `Location`s pointing into the *defining*
    /// documents.
    async fn cross_document_definition(
        &self,
        uri: &Url,
        source: &str,
        pos: Position,
    ) -> jsonrpc::Result<Vec<Location>> {
        // A `$var` reference can't resolve to a cross-document
        // proc / class.
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return Ok(Vec::new());
        }
        let Some((word, _, _)) =
            core_hover::find_word_span_at_position(source, pos.line, pos.character)
        else {
            return Ok(Vec::new());
        };
        // Collect (uri, name_span) targets from the index.
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            index
                .proc_definitions(&word, uri.as_str())
                .into_iter()
                .map(|p| (p.uri.clone(), p.name_span))
                .chain(
                    index
                        .class_definitions(&word, uri.as_str())
                        .into_iter()
                        .map(|c| (c.uri.clone(), c.name_span)),
                )
                .collect()
        };
        Ok(self.resolve_target_locations(targets).await)
    }

    /// Resolve a list of `(target-uri, byte-span)` pairs to LSP
    /// `Location`s, converting each span against the *target*
    /// document's source.  Targets whose document isn't in the
    /// store, or whose URI doesn't parse, are dropped.
    async fn resolve_target_locations(
        &self,
        targets: Vec<(String, tcl_lexer::Span)>,
    ) -> Vec<Location> {
        let mut locations = Vec::new();
        for (target_uri, span) in targets {
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at_utf16(span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(span.end(), &target_doc.text);
            locations.push(Location {
                uri: parsed,
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
            });
        }
        locations
    }

    /// Resolve the proc / class symbol at `pos` (a bare command
    /// word, not a `$var`) to its `(simple_name,
    /// qualified_name)`, consulting the current document's
    /// analysis first and the workspace index second.  Used by
    /// cross-document references / rename to identify which
    /// symbol's call sites to gather.
    async fn resolve_workspace_symbol(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<(String, String)> {
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return None;
        }
        let (word, _, _) = core_hover::find_word_span_at_position(source, pos.line, pos.character)?;
        // Current document: proc, then class.
        for (qname, proc_def) in &analysis.all_procs {
            if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
                return Some((proc_def.name.clone(), proc_def.qualified_name.clone()));
            }
        }
        for class_def in analysis.all_classes.values() {
            if class_def.name == word
                || class_def.qualified_name == word
                || class_def.qualified_name == format!("::{word}")
            {
                return Some((class_def.name.clone(), class_def.qualified_name.clone()));
            }
        }
        // Otherwise the symbol may be defined in a sibling
        // document — resolve its qualified name from the index.
        // Gate this on the cursor actually sitting on a command
        // invocation head in *this* document: without it, a
        // coincidental bareword argument (`puts hello`) would resolve
        // against any sibling document that happens to define a proc /
        // class of the same name, producing spurious cross-document
        // references / rename edits.
        if !position_is_command_head(source, pos, analysis) {
            return None;
        }
        let index = self.workspace_index.lock().await;
        if let Some(p) = index.proc_definitions(&word, uri.as_str()).first() {
            return Some((p.name.clone(), p.qualified_name.clone()));
        }
        if let Some(c) = index.class_definitions(&word, uri.as_str()).first() {
            return Some((c.name.clone(), c.qualified_name.clone()));
        }
        None
    }

    /// Cross-document references for the proc / class at `pos`:
    /// every invocation site in *other* documents, plus the
    /// definition sites in other documents when
    /// `include_declaration`.  Returns `Location`s resolved
    /// against their defining documents.
    async fn cross_document_references(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> Vec<Location> {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return Vec::new();
        };
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            let mut t: Vec<(String, tcl_lexer::Span)> = index
                .invocations_of(&simple, &qualified, uri.as_str())
                .into_iter()
                .map(|i| (i.uri.clone(), i.range))
                .collect();
            if include_declaration {
                for p in index.proc_definitions(&simple, uri.as_str()) {
                    t.push((p.uri.clone(), p.name_span));
                }
                for c in index.class_definitions(&simple, uri.as_str()) {
                    t.push((c.uri.clone(), c.name_span));
                }
            }
            t
        };
        self.resolve_target_locations(targets).await
    }

    /// Add cross-document rename edits for the proc / class at
    /// `pos` into `changes`.  Resolves the symbol, asks the core
    /// rename provider for the namespace-aware sibling-document
    /// edit intents (call sites + definition sites), converts
    /// each byte span to a range against its target document,
    /// and merges into the per-URI edit map (deduped).
    async fn add_cross_document_rename_edits(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>,
    ) {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return;
        };
        let intents = {
            let index = self.workspace_index.lock().await;
            core_rename::cross_document_symbol_edits(
                &simple,
                &qualified,
                new_name,
                &index,
                uri.as_str(),
            )
        };
        for intent in intents {
            let Ok(parsed) = Url::parse(&intent.uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at_utf16(intent.span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(intent.span.end(), &target_doc.text);
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
                new_text: intent.new_text,
            };
            let bucket = changes.entry(parsed).or_default();
            if !bucket.iter().any(|e| e.range == edit.range) {
                bucket.push(edit);
            }
        }
    }

    /// Cross-document incoming calls to the proc whose
    /// qualified name is `qualified`: run the externally-
    /// targeted incoming-calls pass over every analysed
    /// document *other than* `current_uri`, tagging each result
    /// with its document's URI.  Documents that don't define
    /// the proc still contribute their call sites (passing
    /// `None` for the declaration span so nothing is skipped).
    async fn cross_document_incoming_calls(
        &self,
        current_uri: &Url,
        qualified: &str,
    ) -> Vec<(Url, core_call_hierarchy::IncomingCall)> {
        let simple = qualified.rsplit("::").next().unwrap_or(qualified);
        // Collect (uri, source, dialect) for every document *other
        // than* the current one: first the open buffers, then the
        // indexed-but-unopened files the folder scan discovered
        // (read from disk).  Without the latter, callers living in
        // files the editor never opened would be missing from the
        // incoming-call results even though they are indexed for
        // every other cross-document feature.
        let mut open_uris: HashSet<Url> = HashSet::new();
        let mut docs: Vec<(Url, String, String)> = {
            let store = self.documents.lock().await;
            store
                .iter()
                .filter(|(u, _)| *u != current_uri)
                .map(|(u, d)| {
                    open_uris.insert(u.clone());
                    (u.clone(), d.text.clone(), d.dialect.clone())
                })
                .collect()
        };
        let indexed = self.workspace_index.lock().await.document_uris();
        for uri_str in indexed {
            let Ok(uri) = Url::parse(&uri_str) else {
                continue;
            };
            if uri == *current_uri || open_uris.contains(&uri) {
                continue;
            }
            if let Some(doc) = self.read_document(&uri).await {
                docs.push((uri, doc.text, doc.dialect));
            }
        }
        let mut out = Vec::new();
        for (doc_uri, source, dialect) in docs {
            let analysis = self
                .cached_analysis(&doc_uri)
                .await
                .unwrap_or_else(|| Analyser::new().analyse(&source, &dialect).clone());
            let calls = core_call_hierarchy::incoming_calls_for_target(
                &source, &analysis, simple, qualified, None,
            );
            for c in calls {
                out.push((doc_uri.clone(), c));
            }
        }
        out
    }

    /// Cross-document outgoing calls from the queried proc /
    /// method: the call sites inside its body whose callee is
    /// defined in a *sibling* document.  The local
    /// [`core_call_hierarchy::outgoing_calls`] pass only resolves
    /// callees in the current document; this resolves the
    /// remaining heads against the workspace index, builds a `to`
    /// item pointing at the sibling definition (URI + name span →
    /// range against that document's source), and keeps the call
    /// ranges from the *current* document.
    async fn cross_document_outgoing_calls(
        &self,
        current_uri: &Url,
        source: &str,
        dialect: &str,
        item: &core_call_hierarchy::CallHierarchyItem,
        analysis: &AnalysisResult,
    ) -> Vec<CallHierarchyOutgoingCall> {
        let unresolved = {
            let source = source.to_owned();
            let dialect = dialect.to_owned();
            let item = item.clone();
            let analysis = analysis.clone();
            tokio::task::spawn_blocking(move || {
                core_call_hierarchy::unresolved_outgoing_calls(&source, &dialect, &item, &analysis)
            })
            .await
            .unwrap_or_default()
        };
        let mut out = Vec::new();
        for call in unresolved {
            // Resolve the callee head against the index, preferring
            // the analyser's resolved qualified name when present.
            let Some((target_uri, qualified, name_span, detail)) = ({
                let index = self.workspace_index.lock().await;
                let mut found = None;
                for key in [
                    call.resolved_qualified_name.as_deref(),
                    Some(call.name.as_str()),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(p) = index.proc_definitions(key, current_uri.as_str()).first() {
                        found = Some((
                            p.uri.clone(),
                            p.qualified_name.clone(),
                            p.name_span,
                            Some(format!("({} params)", p.param_count)),
                        ));
                        break;
                    }
                    if let Some(c) = index.class_definitions(key, current_uri.as_str()).first() {
                        found = Some((c.uri.clone(), c.qualified_name.clone(), c.name_span, None));
                        break;
                    }
                }
                found
            }) else {
                continue;
            };
            // Convert the sibling definition's name span to a range
            // against that document's source.
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at_utf16(name_span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(name_span.end(), &target_doc.text);
            let name_range = Range {
                start: Position {
                    line: start.line,
                    character: start.character,
                },
                end: Position {
                    line: end.line,
                    character: end.character,
                },
            };
            out.push(CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: qualified,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail,
                    uri: parsed,
                    range: name_range,
                    selection_range: name_range,
                    data: None,
                },
                from_ranges: call.from_ranges.into_iter().map(lift_lsp_range).collect(),
            });
        }
        out
    }

    /// Handle the `tcl-lsp.minifyDocument` workspace command.
    ///
    /// Arguments (positional, matching the editor extensions):
    /// `[uri: string, compact: bool, aggressive: bool, isolated: bool]`.
    /// Returns a JSON object with `source`, `originalLength`,
    /// `minifiedLength`, and — for the compact / aggressive tiers —
    /// `symbolMap` (and `optimisationsApplied` for aggressive),
    /// mirroring `lsp/commands.py::on_minify_document`.
    async fn minify_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let compact = args
            .get(1)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let aggressive = args
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let isolated = args
            .get(3)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let registry = self.registry_for_dialect(&doc.dialect).await;

        // Pure-CPU work; run off the LSP event loop.
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let value = tokio::task::spawn_blocking(move || {
            if aggressive {
                let res = core_minify::minify_tcl_aggressive(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": res.source,
                    "originalLength": res.original_length,
                    "minifiedLength": res.minified_length(),
                    "symbolMap": res.symbol_map.format(),
                    "optimisationsApplied": res.optimisations_applied,
                })
            } else if compact {
                let (minified, symbol_map) =
                    core_minify::minify_tcl_compact(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                    "symbolMap": symbol_map.format(),
                })
            } else {
                let minified = core_minify::minify_tcl(&text, &dialect, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                })
            }
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("minify worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle the `tcl-lsp.optimiseDocument` workspace command.
    ///
    /// Arguments: `[uri, profile?]`.  Runs the optimiser over the document
    /// (multi-pass for the `"full"` profile, single-pass otherwise),
    /// applies the rewrites, and returns `{source, optimisations}` — the
    /// optimised text plus the list of applied optimisation suggestions.
    /// Mirrors the Python `tcl-lsp.optimiseDocument` command.
    async fn optimise_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // The `"full"` profile iterates to a fixpoint; anything else is a
        // single pass.  Default to `"full"`.
        let profile = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("full")
            .to_owned();
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let value = tokio::task::spawn_blocking(move || {
            let dialect_opt = Some(dialect.as_str());
            let (source, opts) = if profile == "full" {
                tcl_compiler::optimiser::optimise_source_multipass(&text, &registry, dialect_opt, 5)
            } else {
                let opts =
                    tcl_compiler::optimiser::optimise_with_dialect(&text, &registry, dialect_opt);
                let applied = tcl_compiler::optimiser::apply_optimisations(&text, &opts);
                (applied, opts)
            };
            let line_index = tcl_lexer::LineIndex::new(&text);
            let items: Vec<serde_json::Value> = opts
                .iter()
                .map(|o| {
                    let start = line_index.position_at_utf16(o.span.start(), &text);
                    let end = line_index.position_at_utf16(o.span.end(), &text);
                    serde_json::json!({
                        "code": o.code,
                        "message": o.message,
                        "startLine": start.line,
                        "startCharacter": start.character,
                        "endLine": end.line,
                        "endCharacter": end.character,
                        "replacement": o.replacement,
                        "group": o.group,
                        "hintOnly": o.hint_only,
                    })
                })
                .collect();
            serde_json::json!({
                "source": source,
                "optimisations": items,
            })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("optimise worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle the `tcl-lsp.unminifyError` workspace command.
    ///
    /// Arguments: `[errorMessage, symbolMapText, minifiedSource?,
    /// originalSource?]`.  Translates compacted names in the error
    /// message back to originals via the symbol map and returns
    /// `{originalError, translatedError, changed}`.  Source-
    /// correlated line remapping (the `minified` / `original`
    /// arguments) is a follow-up.
    fn unminify_error_command(args: &[serde_json::Value]) -> Option<serde_json::Value> {
        let error_message = args.first().and_then(serde_json::Value::as_str)?;
        let symbol_map_text = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let symbol_map = core_minify::SymbolMap::parse(symbol_map_text);
        let mut translated = core_minify::unminify_error(error_message, &symbol_map);
        // When both the minified and original sources are supplied,
        // remap minified line references to approximate original lines.
        let minified = args.get(2).and_then(serde_json::Value::as_str);
        let original = args.get(3).and_then(serde_json::Value::as_str);
        if let (Some(minified), Some(original)) = (minified, original) {
            if !minified.is_empty() && !original.is_empty() {
                translated = core_minify::remap_line_references(&translated, minified, original);
            }
        }
        Some(serde_json::json!({
            "originalError": error_message,
            "translatedError": translated,
            "changed": translated != error_message,
        }))
    }

    /// Handle `tcl-lsp.describeIruleEvent`: deterministic registry metadata
    /// for an iRules event.  Mirrors `server/commands.py::on_describe_irule_event`.
    /// `validCommandCount` counts the iRules commands (those carrying
    /// `event_requires`) not excluded from the event.
    async fn describe_irule_event_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let event = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let events = tcl_registry::events::EventRegistry::build();
        let known = !event.is_empty() && events.is_known(&event);
        let (count, sample) = if known {
            let registry = self.registry_for_dialect("f5-irules").await;
            let profiles = tcl_registry::profiles::ProfileRegistry::build();
            // Mirror Python `_build_event_set`: every f5-irules command valid
            // in the event, not just those that carry `event_requires`.
            let names = registry.valid_irules_commands_for_event(&event, &events, &profiles);
            let count = names.len();
            let sample: Vec<String> = names.into_iter().take(80).map(str::to_owned).collect();
            (count, sample)
        } else {
            (0, Vec::new())
        };
        // The contract derives `deprecated` from the `when` event-completion
        // detail (`get_event_detail`), which never contains the word — so the
        // describe-event surface always reports false, deliberately *not*
        // `EventProps.deprecated` (the orthogonal F5 deprecation fact).  See
        // `server/commands.py::on_describe_irule_event`.
        let deprecated = false;
        Ok(Some(serde_json::json!({
            "event": event,
            "known": known,
            "deprecated": deprecated,
            "validCommandCount": count,
            "sampleCommands": sample,
        })))
    }

    /// Handle `tcl-lsp.describeIruleCommand`: registry metadata for an iRules
    /// command (exact match, then case-insensitive).  Mirrors
    /// `server/commands.py::on_describe_irule_command`.
    async fn describe_irule_command_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let registry = self.registry_for_dialect("f5-irules").await;
        let resolved = if !name.is_empty() && registry.get(&name).is_some() {
            Some(name.clone())
        } else if name.is_empty() {
            None
        } else {
            let lowered = name.to_lowercase();
            registry
                .command_names()
                .find(|c| c.to_lowercase() == lowered)
                .map(str::to_owned)
        };
        let value = match resolved {
            Some(canonical) => {
                let summary = registry
                    .get(&canonical)
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned());
                serde_json::json!({
                    "found": true,
                    "command": canonical,
                    "summary": summary,
                })
            }
            None => serde_json::json!({ "found": false, "command": name }),
        };
        Ok(Some(value))
    }

    /// Handle `tcl-lsp.listIruleEvents`: all known iRules event names (sorted).
    /// Mirrors `server/commands.py::on_list_irule_events`.
    fn list_irule_events_command() -> serde_json::Value {
        let events = tcl_registry::events::EventRegistry::build();
        let mut names: Vec<&str> = events.all_event_names();
        names.sort_unstable();
        serde_json::json!({ "events": names })
    }

    /// Handle `tcl-lsp.diagramData`: extract the `when EVENT` event names from
    /// a source string.  Mirrors `server/commands.py::on_diagram_data`'s event
    /// extraction (the events slice the e2e test asserts).
    fn diagram_data_command(args: &[serde_json::Value]) -> Option<serde_json::Value> {
        let source = args.first().and_then(serde_json::Value::as_str)?;
        let events: Vec<serde_json::Value> =
            tcl_lsp_core::irules_context::scan_file_events(source, "f5-irules")
                .into_iter()
                .map(|name| serde_json::json!({ "name": name }))
                .collect();
        Some(serde_json::json!({ "events": events }))
    }

    /// Handle `tcl-lsp.fixAllSafeIssues`: apply every non-overlapping safe
    /// diagnostic fix iteratively until the source stabilises.  Mirrors
    /// `server/commands.py::on_fix_all_safe_issues` (the `_SAFE_FIX_CODES`
    /// whitelist + multi-pass apply).
    async fn fix_all_safe_issues_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let (disabled, na_mode) = self.analyser_config().await;
        let dialect = doc.dialect.clone();
        let mut source = doc.text.clone();
        let value = tokio::task::spawn_blocking(move || {
            const SAFE: &[&str] = &["W100", "W105", "W108", "W110", "W201", "W304", "IRULE2001"];
            let mut applied: Vec<serde_json::Value> = Vec::new();
            for _ in 0..4 {
                let mut analyser = Self::configured_analyser(disabled.clone(), na_mode);
                let analysis = analyser.analyse(&source, &dialect).clone();
                // First fix per safe diagnostic, sorted by start offset.
                let mut fixes: Vec<(u32, u32, String, String, String)> = Vec::new();
                for d in &analysis.diagnostics {
                    if !SAFE.contains(&d.code.as_str()) {
                        continue;
                    }
                    if let Some(f) = d.fixes.first() {
                        fixes.push((
                            f.span.start(),
                            f.span.end(),
                            f.new_text.clone(),
                            d.code.clone(),
                            f.description.clone(),
                        ));
                    }
                }
                fixes.sort_by_key(|f| f.0);
                // Keep non-overlapping fixes in order.
                let mut chosen: Vec<(u32, u32, String, String, String)> = Vec::new();
                let mut last_end = 0u32;
                for f in fixes {
                    if f.0 >= last_end {
                        last_end = f.1;
                        chosen.push(f);
                    }
                }
                if chosen.is_empty() {
                    break;
                }
                // Apply from the end so earlier byte offsets stay valid.
                let mut new_source = source.clone();
                for f in chosen.iter().rev() {
                    let (s, e) = (f.0 as usize, f.1 as usize);
                    if s <= e && e <= new_source.len() {
                        new_source.replace_range(s..e, &f.2);
                    }
                }
                if new_source == source {
                    break;
                }
                for f in &chosen {
                    applied.push(serde_json::json!({ "code": f.3, "description": f.4 }));
                }
                source = new_source;
            }
            serde_json::json!({ "source": source, "applied": applied })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("fix-all worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle `tcl-lsp.getEffectiveConfig`: the resolved per-document config —
    /// active dialect, the resolved `features` toggle map, the optimiser
    /// switch, line length, and analyser settings.  Mirrors
    /// `server/commands.py::on_get_effective_config`.  Tests poll this command
    /// after a `tclLsp.features.X = false` config change to confirm the server
    /// has applied the toggle without sleeping on wall-clock time.
    async fn get_effective_config_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let uri_str = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        // Resolve the dialect via the open document when the URI names one,
        // else the session default — never `null`, so a polling client always
        // sees a concrete value.
        let dialect = match Url::parse(uri_str) {
            Ok(uri) => match self.read_document(&uri).await {
                Some(doc) => doc.dialect,
                None => self.default_dialect.lock().await.clone(),
            },
            Err(_) => self.default_dialect.lock().await.clone(),
        };
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let line_length = *self.line_length.lock().await;
        let (disabled, mode) = self.analyser_config().await;
        let mut disabled_sorted: Vec<String> = disabled.into_iter().collect();
        disabled_sorted.sort();
        Ok(Some(serde_json::json!({
            "uri": uri_str,
            "dialect": dialect,
            "features": features,
            "optimiser_enabled": optimiser_enabled,
            "line_length": line_length,
            "non_ascii_mode": non_ascii_mode_str(mode),
            "disabled_diagnostics": disabled_sorted,
        })))
    }

    /// Pull the `tclLsp` configuration section from the client and apply it.
    ///
    /// The editor (and the e2e harness) answer `workspace/configuration` with
    /// the resolved `tclLsp` settings; a bare `didChangeConfiguration` with an
    /// empty payload is the signal to re-pull.  Applies `features.*`, the
    /// optimiser switch, line length, dialect, and the analyser knobs (W108
    /// mode + disabled diagnostics) — only the keys the reply carries, so
    /// omitted keys keep their last-applied value.
    async fn pull_and_apply_config(&self) {
        let items = vec![ConfigurationItem {
            scope_uri: None,
            section: Some("tclLsp".to_owned()),
        }];
        let Ok(values) = self.client.configuration(items).await else {
            return;
        };
        let Some(cfg) = values.into_iter().next() else {
            return;
        };
        if !cfg.is_object() {
            return;
        }
        if let Some(features) = cfg.get("features").and_then(serde_json::Value::as_object) {
            self.feature_toggles.lock().await.apply(features);
        }
        if let Some(flag) = cfg
            .get("optimiser")
            .and_then(|o| o.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            *self.optimiser_enabled.lock().await = flag;
        }
        if let Some(profile) = cfg
            .get("optimiser")
            .and_then(|o| o.get("profile"))
            .and_then(serde_json::Value::as_str)
        {
            *self.optimiser_profile.lock().await =
                tcl_compiler::optimiser::profiles::OptimisationProfile::parse(profile);
        }
        // Per-code overrides: any `optimiser.<CODE>` boolean other than the
        // `enabled` / `profile` keys force-enables (true) or force-disables
        // (false) that O-code on top of the profile. Mirrors Python `settings.py`.
        if let Some(opt) = cfg.get("optimiser").and_then(serde_json::Value::as_object) {
            let mut overrides = self.optimiser_code_overrides.lock().await;
            for (key, val) in opt {
                if key == "enabled" || key == "profile" {
                    continue;
                }
                if let Some(b) = val.as_bool() {
                    overrides.insert(key.clone(), b);
                }
            }
        }
        if let Some(len) = cfg
            .get("formatting")
            .and_then(|f| f.get("lineLength"))
            .or_else(|| cfg.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
        {
            *self.line_length.lock().await = u32::try_from(len).unwrap_or(80);
        }
        if let Some(dialect) = cfg.get("dialect").and_then(serde_json::Value::as_str) {
            *self.default_dialect.lock().await = dialect.to_owned();
        }
        // The pulled value is the *content* of the `tclLsp` section; the
        // `settings_*` helpers expect it wrapped (they look under `tclLsp`),
        // so re-wrap before reusing them for the W108 mode + disabled codes.
        let wrapped = serde_json::json!({ "tclLsp": cfg });
        if let Some(mode) = settings_non_ascii_mode(&wrapped) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(&wrapped) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
    }

    /// Whether the named `tclLsp.features.*` provider is enabled.
    async fn feature_enabled(&self, feature: &str) -> bool {
        self.feature_toggles.lock().await.is_enabled(feature)
    }

    /// Handle `tcl-lsp.listSubcommands`: subcommand metadata for `command`
    /// from the registry.  Mirrors `server/commands.py::on_list_subcommands`.
    /// An unknown command (or one with no subcommands) yields an empty list.
    async fn list_subcommands_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let dialect = self.default_dialect.lock().await.clone();
        let registry = self.registry_for_dialect(&dialect).await;
        let parsed = DialectSet::parse(&dialect).unwrap_or(DialectSet::ALL_TCL);
        let mut subs: Vec<serde_json::Value> = registry
            .get_for_dialect(&name, parsed)
            .map(|spec| {
                spec.subcommands
                    .iter()
                    .map(|sub| {
                        serde_json::json!({
                            "name": sub.name,
                            "detail": sub.detail,
                            "synopsis": sub.synopsis,
                            "pure": sub.pure,
                            "mutator": sub.mutator,
                            // The registry tracks deprecation at command level,
                            // not per subcommand — report the shape with a
                            // conservative default.
                            "deprecated": false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        subs.sort_by(|a, b| {
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .cmp(&b.get("name").and_then(serde_json::Value::as_str))
        });
        Some(serde_json::json!({ "command": name, "subcommands": subs }))
    }

    /// Handle `tcl-lsp.listKnownPackages`.  The native server has no on-disk
    /// `package require` resolver yet (a follow-up), so it reports the empty
    /// set — the contract is a `packages` list, which downstream callers can
    /// rely on regardless of population.
    fn list_known_packages_command() -> serde_json::Value {
        serde_json::json!({ "packages": serde_json::Value::Array(vec![]) })
    }

    /// Handle `tcl-lsp.suggestPackagesForSymbol`.  Echoes the symbol and an
    /// (empty, pending the package resolver) suggestion list.
    fn suggest_packages_for_symbol_command(args: &[serde_json::Value]) -> serde_json::Value {
        let symbol = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        serde_json::json!({ "symbol": symbol, "suggestions": serde_json::Value::Array(vec![]) })
    }

    /// Handle `tcl-lsp.exportConfig`: write the resolved settings to the
    /// user config file (`$XDG_CONFIG_HOME/tcl-lsp/config.ini`, else
    /// `~/.config/tcl-lsp/config.ini`) and return its path.  Mirrors
    /// `server/commands.py::on_export_config`; only the resolved
    /// feature / optimiser / style / disabled-diagnostic state is written.
    async fn export_config_command(&self) -> serde_json::Value {
        let path = config_ini_path();
        let contents = self.render_config_ini().await;
        let result = path.parent().map_or_else(
            || Err(std::io::Error::other("no config directory")),
            std::fs::create_dir_all,
        );
        match result.and_then(|()| std::fs::write(&path, contents)) {
            Ok(()) => serde_json::json!({
                "success": true,
                "path": path.to_string_lossy(),
            }),
            Err(err) => serde_json::json!({
                "success": false,
                "error": err.to_string(),
            }),
        }
    }

    /// Render the resolved config as an INI document (the format the server
    /// reads from `config.ini`): `[features]`, `[optimiser]`, `[style]`, and
    /// the disabled `[diagnostics]` codes.
    async fn render_config_ini(&self) -> String {
        use std::fmt::Write as _;
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let line_length = *self.line_length.lock().await;
        let mut disabled: Vec<String> = self
            .disabled_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        disabled.sort();

        let mut out = String::from("# tcl-lsp configuration (exported)\n\n[features]\n");
        let mut feature_keys: Vec<&String> = features.keys().collect();
        feature_keys.sort();
        for key in feature_keys {
            let enabled = features.get(key).and_then(serde_json::Value::as_bool) == Some(true);
            let _ = writeln!(out, "{key} = {enabled}");
        }
        let _ = write!(out, "\n[optimiser]\nenabled = {optimiser_enabled}\n");
        let _ = write!(out, "\n[style]\nlineLength = {line_length}\n");
        if !disabled.is_empty() {
            out.push_str("\n[diagnostics]\n");
            for code in disabled {
                let _ = writeln!(out, "{code} = false");
            }
        }
        out
    }

    /// Snapshot the user-configured analyser settings (disabled
    /// diagnostic codes + W108 non-ASCII mode) so a blocking analysis
    /// worker can build its `Analyser` without holding any async mutex.
    async fn analyser_config(&self) -> (HashSet<String>, NonAsciiMode) {
        let disabled = self.disabled_diagnostics.lock().await.clone();
        let mode = *self.non_ascii_mode.lock().await;
        (disabled, mode)
    }

    /// Build an `Analyser` carrying the configured disabled-diagnostics
    /// set and W108 mode.
    fn configured_analyser(disabled: HashSet<String>, mode: NonAsciiMode) -> Analyser {
        Analyser::with_disabled_diagnostics(disabled).with_non_ascii_mode(mode)
    }

    /// Return an `Arc<CommandRegistry>` with `dialect` loaded on
    /// top of the default Tcl + stdlib + tcllib specs.
    ///
    /// The result is cached per canonical dialect key. Concurrent
    /// first-use requests may build the same registry more than once,
    /// but only one `Arc` is inserted; keeping construction outside the
    /// mutex avoids blocking unrelated cache hits on spec loading.
    /// Unparseable dialect strings collapse to the empty-string key so
    /// they share a single cached "plain Tcl" registry rather than
    /// leaking a fresh allocation per typo.
    async fn registry_for_dialect(&self, dialect: &str) -> Arc<CommandRegistry> {
        let parsed = DialectSet::parse(dialect);
        // Canonicalise the cache key: parseable dialects keep
        // their string; unparseable / plain-Tcl collapse to "".
        let key = if parsed.is_some() { dialect } else { "" };
        self.cached_registry(key, parsed).await
    }

    /// Cache helper for [`Self::registry_for_dialect`].
    ///
    /// Builds (or fetches) a registry holding the base specs plus,
    /// when `dialect` is `Some`, the dialect-loaded specs. The
    /// cache owns the registries via `Arc`, so they are dropped
    /// when the `Backend` is dropped instead of living for the
    /// process lifetime.
    async fn cached_registry(
        &self,
        key: &str,
        dialect: Option<DialectSet>,
    ) -> Arc<CommandRegistry> {
        if let Some(r) = self.dialect_registries.lock().await.get(key).cloned() {
            return r;
        }
        let mut r = CommandRegistry::build_default();
        if let Some(d) = dialect {
            r.load_dialect(d);
        }
        let arc = Arc::new(r);
        let mut cache = self.dialect_registries.lock().await;
        let entry = cache
            .entry(key.to_owned())
            .or_insert_with(|| Arc::clone(&arc));
        Arc::clone(entry)
    }

    /// Run the analyser on `text` and push the resulting
    /// diagnostics to the LSP client for `uri`.  Mirrors
    /// the Python server's `publish_diagnostics` flow.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.  `S-async-diagnostics`
    /// minimal port — the cached-analysis surface and
    /// debounced streaming contract lands in a follow-up.
    async fn publish_analyser_diagnostics(
        &self,
        uri: Url,
        text: String,
        dialect: String,
        revision: u64,
        version: Option<i32>,
    ) {
        // GAP-C1: the base analyser only surfaces `Analyser::analyse`
        // diagnostics.  The optimiser O-codes, GVN redundancies,
        // shimmer/thunking, taint (W2xx / T1xx), and the iRules
        // control-flow checks are all implemented in `tcl-compiler`
        // but, until now, reached no server caller — only the PyO3
        // bridge.  Run `compiler_checks::run_all_checks` +
        // `optimiser::optimise_with_dialect` on the same worker and
        // merge their diagnostics into the published set.  Both need
        // the dialect-aware registry, so resolve it before the
        // blocking hop and move an `Arc` clone in.
        //
        // Lifecycle/timing instrumentation routed to the client as
        // `window/logMessage` (mirrors the Python server's `[timing]` lines so
        // the same logs appear in editors' LSP output channels — and so the
        // e2e harness can await the per-URI snapshot-built marker).
        // F5 BIG-IP config (`bigip.conf`, …) is not Tcl source. The
        // general Tcl analyser must never run on it — doing so mis-reads
        // BIG-IP encrypted-string markers (`$M$…$`) as Tcl `$var`
        // references (W210) and flags stanza syntax like `ltm pool …` as
        // bad Tcl (W123 / E002). Publish an empty (no general-Tcl)
        // diagnostic set keyed on the document version and return before
        // any analysis, mirroring the Python `f5-bigip` diagnostics skip
        // (#571). The Tcl `workspace_state.update` timing marker is
        // deliberately *not* emitted on this path — there is no Tcl
        // analysis snapshot to advertise.
        if Self::is_bigip_dialect(&dialect) {
            let is_current = {
                let docs = self.documents.lock().await;
                docs.get(&uri).is_some_and(|doc| doc.revision == revision)
            };
            if is_current {
                self.client
                    .publish_diagnostics(uri, Vec::new(), version)
                    .await;
            }
            return;
        }
        let started = std::time::Instant::now();
        let uri_str = uri.to_string();
        let line_count = text.lines().count();
        let registry = self.registry_for_dialect(&dialect).await;
        let (disabled, na_mode) = self.analyser_config().await;
        // The optimiser O-codes the diagnostics path must suppress: the active
        // profile's disabled categories, then per-code user overrides applied on
        // top (`tclLsp.optimiser.O111=false` adds, `=true` removes). The master
        // `tclLsp.optimiser.enabled=false` switch suppresses every O-code.
        // Mirrors Python's `disabled_optimisations` + `optimiser_enabled`.
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let opt_disabled: std::collections::HashSet<String> = {
            let mut set: std::collections::HashSet<String> =
                tcl_compiler::optimiser::profiles::profile_to_disabled(
                    *self.optimiser_profile.lock().await,
                )
                .into_iter()
                .map(str::to_owned)
                .collect();
            for (code, enabled) in self.optimiser_code_overrides.lock().await.iter() {
                if *enabled {
                    set.remove(code);
                } else {
                    set.insert(code.clone());
                }
            }
            set
        };
        let result = tokio::task::spawn_blocking(move || {
            let mut analyser = Self::configured_analyser(disabled.clone(), na_mode);
            let analysis = analyser.analyse(&text, &dialect).clone();
            let mut diagnostics = lift_analyser_diagnostics(&text, &analysis.diagnostics);
            diagnostics.extend(lift_compiler_diagnostics(
                &text,
                &registry,
                &dialect,
                optimiser_enabled,
                &opt_disabled,
            ));
            // GAP-C1 strip 2: source-style pass (W111 / W112 / W115
            // / W118), suppression-filtered via the analyser's
            // `suppressed_lines` and the user's disabled-diagnostics set.
            diagnostics.extend(lift_source_style_diagnostics(
                &text,
                &analysis.suppressed_lines,
                &disabled,
            ));
            (analysis, diagnostics)
        })
        .await;
        match result {
            Ok((analysis, diags)) => {
                {
                    let _gate = self.document_analysis_gate.lock().await;
                    let is_current = {
                        let docs = self.documents.lock().await;
                        docs.get(&uri).is_some_and(|doc| doc.revision == revision)
                    };
                    if !is_current {
                        return;
                    }
                    // Cache the analysis so the per-method
                    // handlers don't have to re-run it on every
                    // request.
                    {
                        // Refresh the cross-document workspace index
                        // for this URI (remove stale entries, then
                        // re-add the fresh definitions).
                        let mut index = self.workspace_index.lock().await;
                        index.remove_document(uri.as_str());
                        index.add_document(uri.as_str(), &analysis);
                    }
                    self.analyses.lock().await.insert(uri.clone(), analysis);
                }
                // The LSP version is attached to normal analyser
                // diagnostics, so clients can discard this publish if a
                // newer edit overtakes it after the cache/index update.
                let diag_count = diags.len();
                self.client.publish_diagnostics(uri, diags, version).await;
                // Snapshot-built marker: emitted once the analysis cache +
                // workspace index are populated and diagnostics published, so
                // analysis-backed handlers (hover, definition, …) are ready.
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                self.client
                    .log_message(
                        MessageType::LOG,
                        format!(
                            "[timing] workspace_state.update {elapsed_ms:.0}ms \
                             (uri={uri_str}, lines={line_count}, diags={diag_count})"
                        ),
                    )
                    .await;
            }
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("diagnostics worker panicked: {err}"),
                    )
                    .await;
            }
        }
    }

    /// On-disk workspace-folder scan: analyse `.tcl` / `.tm`
    /// files in the workspace folders that the editor hasn't
    /// opened, merging their definitions / invocation sites into
    /// the cross-document workspace index.
    ///
    /// Editor-opened documents already feed the index through
    /// `publish_analyser_diagnostics`; this fills the gap for
    /// project files the user hasn't opened yet so cross-document
    /// features (references / rename / call-hierarchy /
    /// completion) see the whole project, not just open tabs.
    ///
    /// The directory walk, file reads and per-file analysis run on
    /// a `spawn_blocking` worker so the LSP event loop stays
    /// responsive.  URIs already present in `self.documents` are
    /// skipped so the on-disk copy never clobbers a live (and
    /// possibly unsaved) editor buffer.  The walk is capped at
    /// [`WORKSPACE_SCAN_FILE_CAP`] files so a large tree can't
    /// stall start-up.
    async fn scan_workspace_folders(&self) {
        let folders = self.workspace_folder_urls().await;
        let roots: Vec<PathBuf> = folders
            .iter()
            .filter_map(|f| f.to_file_path().ok())
            .collect();
        if roots.is_empty() {
            return;
        }
        // Snapshot the dialect-resolution inputs and the set of
        // open documents so the blocking worker can run without
        // touching any async mutex.
        let open: HashSet<Url> = self.documents.lock().await.keys().cloned().collect();
        let folder_dialects = self.folder_dialects.lock().await.clone();
        let default_dialect = self.default_dialect.lock().await.clone();

        let analysed = tokio::task::spawn_blocking(move || {
            let mut files: Vec<PathBuf> = Vec::new();
            for root in &roots {
                collect_tcl_files(root, WORKSPACE_SCAN_FILE_CAP, &mut files);
            }
            let mut out: Vec<(String, AnalysisResult)> = Vec::new();
            for path in files {
                let Ok(uri) = Url::from_file_path(&path) else {
                    continue;
                };
                if open.contains(&uri) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let dialect = folder_dialect_for(&uri, &folder_dialects)
                    .unwrap_or_else(|| default_dialect.clone());
                let mut analyser = Analyser::new();
                let analysis = analyser.analyse(&text, &dialect).clone();
                out.push((uri.to_string(), analysis));
            }
            out
        })
        .await
        .unwrap_or_default();

        self.merge_workspace_scan_results(&analysed).await;
    }

    /// Merge disk-backed workspace scan results into the shared index.
    ///
    /// The blocking scan snapshots open documents before it starts, but
    /// an editor can open a file while the scan is still running. Recheck
    /// the live document map at publication time so stale on-disk
    /// analysis cannot overwrite the open-buffer entry.
    async fn merge_workspace_scan_results(&self, analysed: &[(String, AnalysisResult)]) {
        let _gate = self.document_analysis_gate.lock().await;
        let open: HashSet<String> = self
            .documents
            .lock()
            .await
            .keys()
            .map(ToString::to_string)
            .collect();
        let mut index = self.workspace_index.lock().await;
        for (uri, analysis) in analysed {
            if open.contains(uri) {
                continue;
            }
            index.remove_document(uri);
            index.add_document(uri, analysis);
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        Ok(InitializeResult {
            capabilities: build_server_capabilities(),
            server_info: Some(ServerInfo {
                // Protocol identity must match the Python server / editor
                // expectations ("tcl-lsp"), not the crate/binary name.
                name: "tcl-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
        // Pull the resolved `tclLsp` config once the client is ready so
        // feature toggles / optimiser switch / analyser knobs are in effect
        // before the first request.
        self.pull_and_apply_config().await;
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        self.scan_workspace_folders().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let dialect = self
            .dialect_for_open(&params.text_document.uri, &params.text_document.language_id)
            .await;
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let version = params.text_document.version;
        let dialect_for_diags = dialect.clone();
        let (revision, version) = {
            let _gate = self.document_analysis_gate.lock().await;
            let mut docs = self.documents.lock().await;
            docs.insert(
                uri.clone(),
                DocumentState::with_version(params.text_document.text, dialect, version),
            );
            drop(docs);
            self.hover_cache.lock().await.invalidate_uri(&uri);
            self.semantic_tokens_cache.lock().await.remove(&uri);
            self.analyses.lock().await.remove(&uri);
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            (0, Some(version))
        };
        self.publish_analyser_diagnostics(uri, text, dialect_for_diags, revision, version)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // INCREMENTAL sync: each content change is either a full-document
        // replacement (`range == None`) or a ranged edit applied to the
        // current text. Apply them in order. (The re-analysis below is
        // still whole-document; bounding it to `reparse_window` is a
        // documented follow-up — the primitives exist in `tcl-lexer`.)
        let uri = params.text_document.uri.clone();
        if params.content_changes.is_empty() {
            return;
        }
        let default_dialect = self.default_dialect.lock().await.clone();
        let change_version = params.text_document.version;
        let (text, dialect, revision, version) = {
            let _gate = self.document_analysis_gate.lock().await;
            let mut docs = self.documents.lock().await;
            let entry = docs
                .entry(uri.clone())
                // didChange before didOpen — start from empty text and the
                // session default dialect (no languageId is available here).
                .or_insert_with(|| DocumentState::new(String::new(), default_dialect));
            let mut text = std::mem::take(&mut entry.text);
            for change in &params.content_changes {
                text = apply_content_change(&text, change.range, &change.text);
            }
            entry.text = text.clone();
            entry.bump_revision(change_version);
            let dialect = entry.dialect.clone();
            let revision = entry.revision;
            let version = entry.version;
            drop(docs);
            // `S-hover-sync11`: drop every cached hover response
            // for this URI so subsequent requests return answers
            // against the freshly-edited source rather than stale
            // pre-edit results.
            self.hover_cache.lock().await.invalidate_uri(&uri);
            // `S-semantic-tokens-rich` delta: drop the cached
            // token snapshot so the next `semanticTokens/full/delta`
            // returns a fresh full result instead of an empty edit
            // list against an outdated baseline.
            self.semantic_tokens_cache.lock().await.remove(&uri);
            // Evict the stale `AnalysisResult` so any request that
            // arrives before `publish_analyser_diagnostics` finishes
            // re-running the analyser falls through to a fresh
            // run via `analysis_for` rather than serving pre-edit
            // results (PR #454 Codex review P1).  `publish_*` will
            // reinsert the fresh entry when it completes.
            self.analyses.lock().await.remove(&uri);
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            (text, dialect, revision, version)
        };
        self.publish_analyser_diagnostics(uri, text, dialect, revision, version)
            .await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        // Accept ``{"tclLsp": {"dialect": "<name>"}}`` in either the
        // VS Code-style nested shape or the flat ``{"dialect":
        // "<name>"}`` shape (used by the MCP bridge). Update the
        // session default so newly opened documents pick up the
        // change; existing documents keep the dialect they were
        // opened with.
        let dialect = params
            .settings
            .get("tclLsp")
            .and_then(|v| v.get("dialect"))
            .or_else(|| params.settings.get("dialect"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if let Some(d) = dialect {
            *self.default_dialect.lock().await = d;
        }
        // W108 mode + disabled-diagnostics reconfiguration. Existing
        // documents pick the change up on their next analyse (the
        // analyses cache is keyed per URI and refreshed on edit).
        if let Some(mode) = settings_non_ascii_mode(&params.settings) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(&params.settings) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
        // VS Code (and the e2e harness) push an empty/partial payload as a
        // signal to re-pull the full resolved config via
        // `workspace/configuration`.  Always re-pull so `features.*`, the
        // optimiser switch, and the analyser knobs reflect the latest editor
        // settings — the inline `params.settings` handling above covers the
        // flat MCP-bridge shape that carries the values directly.
        self.pull_and_apply_config().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        {
            let _gate = self.document_analysis_gate.lock().await;
            self.documents.lock().await.remove(uri);
            self.analyses.lock().await.remove(uri);
            self.hover_cache.lock().await.invalidate_uri(uri);
            self.semantic_tokens_cache.lock().await.remove(uri);
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            // Clear any previously-published diagnostics before a later
            // didOpen for the same URI can run. Close publishes do not
            // carry a reliable document version, so this short publish
            // stays ordered by the gate.
            self.client
                .publish_diagnostics(uri.clone(), Vec::new(), None)
                .await;
        }
        // Re-index the file from disk rather than dropping it: the
        // file still exists on disk and was (or would be) part of
        // the on-disk index, so cross-document definition /
        // references / rename / call-hierarchy must keep seeing it
        // after the editor closes the buffer.  `scan_workspace_folders`
        // only runs at `initialized`, so a plain `remove_document`
        // here would make the file vanish until restart. The helper
        // rechecks that this URI is still closed before publishing
        // the disk-backed index entry.
        self.reindex_index_from_disk(uri).await;
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        if !self.feature_enabled("folding").await {
            return Ok(None);
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let ranges = tcl_lsp_core::folding::folding_ranges(&doc.text, &doc.dialect, &registry);
        Ok(Some(ranges.into_iter().map(lift_folding_range).collect()))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        if !self.feature_enabled("documentSymbols").await {
            return Ok(None);
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        // BIG-IP config documents get a `module → kind → object` outline
        // built from their stanza tree rather than the Tcl scope walk
        // (which would find nothing in non-Tcl config text). Nameless
        // singletons fall back to their kind label so no outline symbol
        // ever carries an empty `name` (#534).
        let symbols = if Self::is_bigip_dialect(&doc.dialect) {
            core_bigip::document_symbols(&doc.text)
        } else {
            // Reuse the cached per-document analysis instead of re-running the
            // full analyser on every request (the dominant documentSymbol cost
            // on large files).
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await;
            core_symbols::document_symbols_from_analysis(&doc.text, &analysis)
        };
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        if !self.feature_enabled("completion").await {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position.position;
        // Fetch the cached per-dialect registry on the async
        // path before spawning so the worker can read built-in
        // command names alongside the user-defined procs the
        // analyser surfaces.  Threads through as
        // `S-completion-rich`: minimal completion only knows
        // user procs / vars; rich completion adds the registry's
        // command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Snapshot the cross-document index so the worker can
        // enumerate procs from sibling files.  Cloning is the
        // price of moving it into `spawn_blocking`; the index
        // holds only definition metadata, not document text.
        let workspace = self.workspace_index.lock().await.clone();
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = tokio::task::spawn_blocking(move || {
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&workspace),
                &doc.dialect,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items
            .into_iter()
            .map(|item| lift_completion_item(item, pos.line))
            .collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self.feature_enabled("definition").await {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        // GAP-B3 strip 3: for a `$var`, go-to-declaration resolves the
        // visible `global` / `variable` / `upvar` scoping statement that
        // declares the name; for any other symbol it defers to plain
        // go-to-definition (handled inside `core_declaration`).
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_declaration::declaration(
                &text,
                pos.line,
                pos.character,
                &dialect,
                &analysis,
                &registry,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("declaration worker panicked: {err}").into(),
            data: None,
        })?;
        let locations: Vec<Location> = if ranges.is_empty() {
            // No in-document declaration / definition — try the
            // cross-document index (a sibling-file proc / class), the
            // same fallback go-to-definition uses.
            self.compute_definition(&uri, pos).await?
        } else {
            ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect()
        };
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        // GAP-B3 strip 2: type-definition jumps to the class that types
        // the symbol (a `$obj` instance's class, or a method's owning
        // class) — not the plain definition site it used to alias.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_type_definition::type_definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type-definition worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        Ok(Some(GotoTypeDefinitionResponse::Array(locations)))
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> jsonrpc::Result<Option<GotoImplementationResponse>> {
        // GAP-B3 strip 1: go-to-implementation is the TclOO subclass /
        // method-override fan-out, not the plain definition site it used
        // to alias.  Resolve in-document via `core_implementation`.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_implementation::implementation(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("implementation worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        Ok(Some(GotoImplementationResponse::Array(locations)))
    }

    async fn references(&self, params: ReferenceParams) -> jsonrpc::Result<Option<Vec<Location>>> {
        if !self.feature_enabled("references").await {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_references::references(
                &text,
                &dialect,
                pos.line,
                pos.character,
                &analysis_for_worker,
                include_decl,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("references worker panicked: {err}").into(),
            data: None,
        })?;
        // Current-document hits.
        let mut locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        // Cross-document call sites (and, when requested,
        // sibling-document definition sites).
        let cross = self
            .cross_document_references(&uri, &doc.text, &analysis, pos, include_decl)
            .await;
        locations.extend(cross);
        dedup_locations(&mut locations);
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let entries = tokio::task::spawn_blocking(move || {
            // `S-document-highlight-rich`: the kinded entry
            // point tags variable defining spans as `Write` and
            // their reads as `Read`; command-invocation heads
            // stay `Text`.
            core_references::document_highlights(
                &doc.text,
                &doc.dialect,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_highlight worker panicked: {err}").into(),
            data: None,
        })?;
        if entries.is_empty() {
            return Ok(None);
        }
        let highlights = entries
            .into_iter()
            .map(|(r, kind)| DocumentHighlight {
                range: lift_lsp_range(r),
                kind: Some(match kind {
                    core_references::HighlightKind::Read => DocumentHighlightKind::READ,
                    core_references::HighlightKind::Write => DocumentHighlightKind::WRITE,
                    core_references::HighlightKind::Text => DocumentHighlightKind::TEXT,
                }),
            })
            .collect();
        Ok(Some(highlights))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::prepare(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("call_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = items
            .into_iter()
            .map(|i| CallHierarchyItem {
                name: i.name,
                kind: SymbolKind::FUNCTION,
                tags: None,
                detail: i.detail,
                uri: uri.clone(),
                range: lift_lsp_range(i.range),
                selection_range: lift_lsp_range(i.selection_range),
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let qualified = core_item.name.clone();
        let doc_text = doc.text.clone();
        let doc_dialect = doc.dialect.clone();
        let local_item = core_item.clone();
        let local_analysis = analysis.clone();
        let local = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::incoming_calls(
                &doc_text,
                &doc_dialect,
                &local_item,
                &local_analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("incoming_calls worker panicked: {err}").into(),
            data: None,
        })?;
        // Tag local results with the request URI.
        let mut tagged: Vec<(Url, core_call_hierarchy::IncomingCall)> =
            local.into_iter().map(|c| (uri.clone(), c)).collect();
        // Cross-document callers: run the externally-targeted
        // incoming-calls pass over every *other* analysed doc.
        tagged.extend(self.cross_document_incoming_calls(&uri, &qualified).await);
        let lifted = tagged
            .into_iter()
            .map(|(doc_uri, c)| CallHierarchyIncomingCall {
                from: CallHierarchyItem {
                    name: c.from.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.from.detail,
                    uri: doc_uri,
                    range: lift_lsp_range(c.from.range),
                    selection_range: lift_lsp_range(c.from.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Cross-document edges: callees defined in sibling files.
        let cross = self
            .cross_document_outgoing_calls(&uri, &doc.text, &doc.dialect, &core_item, &analysis)
            .await;
        let local_uri = uri.clone();
        let local_analysis = analysis.clone();
        let outgoing = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::outgoing_calls(
                &doc.text,
                &doc.dialect,
                &core_item,
                &local_analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("outgoing_calls worker panicked: {err}").into(),
            data: None,
        })?;
        let mut lifted: Vec<CallHierarchyOutgoingCall> = outgoing
            .into_iter()
            .map(|c| CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: c.to.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.to.detail,
                    uri: local_uri.clone(),
                    range: lift_lsp_range(c.to.range),
                    selection_range: lift_lsp_range(c.to.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        lifted.extend(cross);
        Ok(Some(lifted))
    }

    // Note: type_hierarchy LSP methods aren't exposed by
    // tower-lsp 0.20's `LanguageServer` trait. The core
    // provider in `tcl_lsp_core::type_hierarchy` is ready;
    // wiring lands once we move to a tower-lsp version
    // that surfaces these methods (tracked under
    // `S-type-hierarchy-rich`).

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        // `S-linked-editing-range-rich`: when the cursor sits
        // on a proc declaration's name or inside the proc's
        // body, return every range that should be edited in
        // lock-step (the declaration plus any self-call sites
        // inside the body).  Editors then paint these as
        // linked-edit chips so renaming one updates the others
        // as the user types.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_linked_editing_range::linked_editing_ranges(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("linked_editing_range worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(linked) = result else {
            return Ok(None);
        };
        Ok(Some(LinkedEditingRanges {
            ranges: linked.ranges.into_iter().map(lift_lsp_range).collect(),
            word_pattern: Some(linked.word_pattern),
        }))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        // `S-diagnostics-pipeline`: pull-based diagnostic
        // report.  Editors that support
        // `textDocument/diagnostic` request diagnostics on
        // demand rather than relying on the
        // `publish_diagnostics` push.  We run the analyser via
        // the cached-analysis surface (the same shape every
        // other request uses) and lift each analyser
        // diagnostic to the LSP wire shape.
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(empty_diagnostic_report());
        };
        // BIG-IP config text carries no general Tcl diagnostics — the
        // analyser never runs on it (#571), so the pull report is empty
        // too (matching the push path in `publish_analyser_diagnostics`).
        if Self::is_bigip_dialect(&doc.dialect) {
            return Ok(empty_diagnostic_report());
        }
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || {
            lift_analyser_diagnostics(&doc.text, &analysis.diagnostics)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("diagnostic worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            }),
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: real classification.  The
        // packed integer stream is 5 ints per token
        // `[deltaLine, deltaCol, length, type, modifiers]`.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let core_data = core_semantic_tokens::full(&doc.text, &doc.dialect, &registry).data;
        let result_id = next_semantic_tokens_id();
        self.semantic_tokens_cache.lock().await.insert(
            uri,
            SemanticTokensEntry {
                result_id: result_id.clone(),
                data: core_data.clone(),
            },
        );
        Ok(Some(SemanticTokensResult::Tokens(LspSemanticTokens {
            result_id: Some(result_id),
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let previous_result_id = params.previous_result_id;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let new_data = core_semantic_tokens::full(&doc.text, &doc.dialect, &registry).data;
        let new_result_id = next_semantic_tokens_id();

        // Compare against the cached snapshot.  When the client's
        // `previous_result_id` matches the stream we last handed
        // out for this URI, return the minimal token-aligned edit
        // that turns the cached stream into the new one (an empty
        // edit list when nothing changed).  When the ids don't
        // line up (stale / unknown previous result) fall back to a
        // fresh full token set, which the LSP spec accepts.
        let mut cache = self.semantic_tokens_cache.lock().await;
        let prev_match = cache
            .get(&uri)
            .filter(|entry| entry.result_id == previous_result_id)
            .map(|entry| entry.data.clone());
        cache.insert(
            uri,
            SemanticTokensEntry {
                result_id: new_result_id.clone(),
                data: new_data.clone(),
            },
        );
        drop(cache);
        if let Some(prev_data) = prev_match {
            let edits = match core_semantic_tokens::diff(&prev_data, &new_data) {
                None => Vec::new(),
                Some(edit) => vec![SemanticTokensEdit {
                    start: edit.start,
                    delete_count: edit.delete_count,
                    data: Some(lift_semantic_token_data(&edit.data)),
                }],
            };
            return Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
                SemanticTokensDelta {
                    result_id: Some(new_result_id),
                    edits,
                },
            )));
        }
        Ok(Some(SemanticTokensFullDeltaResult::Tokens(
            LspSemanticTokens {
                result_id: Some(new_result_id),
                data: lift_semantic_token_data(&new_data),
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: filter tokens to those that
        // start inside the requested range.  Editors call this
        // when only the visible viewport needs colouring.
        let core_range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let core_data =
            core_semantic_tokens::range(&doc.text, &doc.dialect, core_range, &registry).data;
        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        // Walk every cached document and collect matching
        // symbols.  The minimal port iterates the document
        // store on the LSP loop (acquiring the mutex briefly);
        // a future workspace-index chunk will move this to a
        // pre-computed index.
        let docs = self.documents.lock().await.clone();
        if docs.is_empty() {
            return Ok(None);
        }
        let query = params.query;
        let mut all: Vec<SymbolInformation> = Vec::new();
        for (uri, doc) in docs {
            let text = doc.text.clone();
            let dialect = doc.dialect.clone();
            let q = query.clone();
            let analysis = self.analysis_for(&uri, text.clone(), dialect.clone()).await;
            let symbols = tokio::task::spawn_blocking(move || {
                core_workspace_symbols::workspace_symbols(&text, &q, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("workspace_symbols worker panicked: {err}").into(),
                data: None,
            })?;
            for s in symbols {
                #[allow(deprecated)]
                all.push(SymbolInformation {
                    name: s.name,
                    kind: match s.kind {
                        CoreWorkspaceSymbolKind::Function => SymbolKind::FUNCTION,
                        CoreWorkspaceSymbolKind::Class => SymbolKind::CLASS,
                        CoreWorkspaceSymbolKind::Method => SymbolKind::METHOD,
                        CoreWorkspaceSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
                    },
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: lift_lsp_range(s.range),
                    },
                    container_name: s.container_name,
                });
            }
        }
        if all.is_empty() {
            return Ok(None);
        }
        Ok(Some(all))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentLink>>> {
        if !self.feature_enabled("documentLinks").await {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-document-links-rich`: pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let workspace_root = uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        let links =
            core_document_links::document_links(&doc.text, &doc.dialect, workspace_root.as_deref());
        if links.is_empty() {
            return Ok(None);
        }
        let lifted = links
            .into_iter()
            .map(|l| DocumentLink {
                range: Range {
                    start: Position {
                        line: l.start_line,
                        character: l.start_character,
                    },
                    end: Position {
                        line: l.end_line,
                        character: l.end_character,
                    },
                },
                target: Url::parse(&l.target).ok(),
                tooltip: l.tooltip,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> jsonrpc::Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Build analysis on a worker so the inlay-hints
        // provider can surface parameter-name hints at user-
        // proc call sites (`S-inlay-hints-rich`) plus built-in
        // command synopsis hints.  When the analyser surfaces an
        // empty all_procs map (no user procs in the document),
        // the provider still returns built-in hints from the
        // registry.
        let hints = tokio::task::spawn_blocking(move || {
            core_inlay_hints::inlay_hints(
                &doc.text,
                &doc.dialect,
                range,
                Some(&analysis),
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("inlay_hint worker panicked: {err}").into(),
            data: None,
        })?;
        if hints.is_empty() {
            return Ok(None);
        }
        let lifted = hints
            .into_iter()
            .map(|h| InlayHint {
                position: Position {
                    line: h.position_line,
                    character: h.position_character,
                },
                label: InlayHintLabel::String(h.label),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn code_lens(&self, params: CodeLensParams) -> jsonrpc::Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // S-code-lens-rich: surface per-proc reference counts
        // above each definition.  The provider walks
        // `analysis.command_invocations` per proc, plus the
        // workspace index for cross-document call sites, so the
        // count reflects workspace-wide usage.
        let workspace = self.workspace_index.lock().await.clone();
        let uri_str = uri.to_string();
        let lenses = tokio::task::spawn_blocking(move || {
            core_code_lens::code_lenses(
                &doc.text,
                &doc.dialect,
                Some(&analysis),
                Some(&workspace),
                &uri_str,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_lens worker panicked: {err}").into(),
            data: None,
        })?;
        if lenses.is_empty() {
            return Ok(None);
        }
        let lifted = lenses
            .into_iter()
            .map(|l| CodeLens {
                range: lift_lsp_range(l.range),
                data: (!l.qname.is_empty()).then(|| serde_json::json!({ "qname": l.qname })),
                command: Some(tower_lsp::lsp_types::Command {
                    title: l.command_title,
                    command: l.command,
                    arguments: None,
                }),
            })
            .collect();
        Ok(Some(lifted))
    }

    #[allow(clippy::too_many_lines)]
    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // `S-code-actions-rich`: walks the analyser's
        // diagnostics for fixes whose span overlaps the
        // requested range, plus fuzzy `package require`
        // suggestions for the word at the cursor.  Run on a worker.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Lift the request-context diagnostics (the editor sends the ones it
        // currently shows) so context-driven quick-fixes — e.g. the iRules
        // taint encode-wrap fixes — can act on them even when the analyser
        // didn't re-emit them.
        let context_diags: Vec<core_code_actions::ContextDiagnostic> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|d| {
                let code = match d.code.as_ref()? {
                    tower_lsp::lsp_types::NumberOrString::String(s) => s.clone(),
                    tower_lsp::lsp_types::NumberOrString::Number(n) => n.to_string(),
                };
                Some(core_code_actions::ContextDiagnostic {
                    code,
                    message: d.message.clone(),
                    range: CoreLspRange {
                        start_line: d.range.start.line,
                        start_character: d.range.start.character,
                        end_line: d.range.end.line,
                        end_character: d.range.end.character,
                    },
                })
            })
            .collect();
        let dialect = doc.dialect.clone();
        let actions = tokio::task::spawn_blocking(move || {
            let mut actions = core_code_actions::code_actions(&doc.text, range, Some(&analysis));
            actions.extend(core_code_actions::package_require_actions(
                &doc.text, range, &registry,
            ));
            actions.extend(core_code_actions::context_diagnostic_actions(
                &doc.text,
                &context_diags,
            ));
            // iRules-only: the `# Profiles:` header source action.
            if dialect == "f5-irules" {
                if let Some(a) = core_code_actions::profiles_action(&doc.text, &analysis, &registry)
                {
                    actions.push(a);
                }
            }
            actions
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_action worker panicked: {err}").into(),
            data: None,
        })?;
        if actions.is_empty() {
            return Ok(None);
        }
        // Honour the client's `only` filter (e.g. `["refactor.extract"]`): an
        // action is kept when its kind prefix-matches a requested kind in
        // either direction (`refactor` matches `refactor.extract` and vice
        // versa).
        let only: Option<Vec<String>> = params.context.only.as_ref().map(|kinds| {
            kinds
                .iter()
                .map(|k| k.as_str().to_owned())
                .collect::<Vec<_>>()
        });
        let lifted: Vec<CodeActionOrCommand> = actions
            .into_iter()
            .filter(|a| {
                only.as_ref().map_or(true, |wanted| {
                    let k = a.kind.as_str();
                    // Keep an action when its kind exactly matches a requested
                    // kind, or is a *subtype* of one (`refactor.extract`
                    // satisfies a `refactor` request). A requested
                    // `refactor.extract` must NOT pull in a generic `refactor`
                    // action — the requested kind is not a parent of the
                    // action's kind in that direction (LSP CodeActionKind
                    // matching is prefix-on-the-action-side only).
                    wanted
                        .iter()
                        .any(|w| k == w || k.starts_with(&format!("{w}.")))
                })
            })
            .map(|a| {
                let mut changes = std::collections::HashMap::new();
                let lifted_edits: Vec<TextEdit> = a
                    .edits
                    .into_iter()
                    .map(|e| TextEdit {
                        range: lift_lsp_range(e.range),
                        new_text: e.new_text,
                    })
                    .collect();
                changes.insert(uri.clone(), lifted_edits);
                let command = a.command.map(|c| tower_lsp::lsp_types::Command {
                    title: a.title.clone(),
                    command: c.command,
                    arguments: Some(c.args.into_iter().map(serde_json::Value::from).collect()),
                });
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: a.title,
                    kind: Some(tower_lsp::lsp_types::CodeActionKind::new(a.kind.as_str())),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command,
                    is_preferred: None,
                    disabled: None,
                    data: None,
                })
            })
            .collect();
        if lifted.is_empty() {
            return Ok(None);
        }
        Ok(Some(lifted))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "tcl-lsp.minifyDocument" => self.minify_document_command(&params.arguments).await,
            "tcl-lsp.optimiseDocument" => self.optimise_document_command(&params.arguments).await,
            "tcl-lsp.unminifyError" => Ok(Self::unminify_error_command(&params.arguments)),
            "tcl-lsp.describeIruleEvent" => {
                self.describe_irule_event_command(&params.arguments).await
            }
            "tcl-lsp.describeIruleCommand" => {
                self.describe_irule_command_command(&params.arguments).await
            }
            "tcl-lsp.listIruleEvents" => Ok(Some(Self::list_irule_events_command())),
            "tcl-lsp.diagramData" => Ok(Self::diagram_data_command(&params.arguments)),
            "tcl-lsp.getEffectiveConfig" => {
                self.get_effective_config_command(&params.arguments).await
            }
            "tcl-lsp.fixAllSafeIssues" => self.fix_all_safe_issues_command(&params.arguments).await,
            "tcl-lsp.listSubcommands" => Ok(self.list_subcommands_command(&params.arguments).await),
            "tcl-lsp.listKnownPackages" => Ok(Some(Self::list_known_packages_command())),
            "tcl-lsp.suggestPackagesForSymbol" => Ok(Some(
                Self::suggest_packages_for_symbol_command(&params.arguments),
            )),
            "tcl-lsp.exportConfig" => Ok(Some(self.export_config_command().await)),
            _ => Ok(None),
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // An explicit client `FormattingOptions.tabSize` / `insertSpaces`
        // overrides the server's indentation by LSP contract.
        let config = formatter_config_from_options(&params.options);
        let edits = core_formatting::formatting_with(&doc.text, &config, &registry);
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let edits = core_formatting::range_formatting(
            &doc.text,
            range,
            &core_formatting::FormatterConfig::default(),
            &registry,
        );
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        if !self.feature_enabled("selectionRange").await {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain = core_selection_range::selection_range(
                        &doc.text,
                        pos.line,
                        pos.character,
                        Some(&analysis),
                    );
                    materialise_selection_range(&chain)
                })
                .collect::<Vec<Option<SelectionRange>>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<SelectionRange> = result.into_iter().flatten().collect();
        if lifted.is_empty() {
            return Ok(None);
        }
        Ok(Some(lifted))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_rename::prepare_rename(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(p) = result else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: lift_lsp_range(p.range),
            placeholder: p.placeholder,
        }))
    }

    async fn rename(&self, params: RenameParams) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // Per-dialect cached registry for `S-rename-rich`
        // safety gating — proc renames refuse to overwrite
        // built-in command names.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        let new_name_worker = new_name.clone();
        let registry_worker = Arc::clone(&registry);
        let edits = tokio::task::spawn_blocking(move || {
            core_rename::rename(
                &text,
                &dialect,
                pos.line,
                pos.character,
                &new_name_worker,
                &analysis_for_worker,
                Some(&registry_worker),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("rename worker panicked: {err}").into(),
            data: None,
        })?;
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        // An empty in-document result means the rename was *rejected*
        // (collision with an existing symbol, an unsafe / built-in target
        // name) or the cursor isn't on a renameable symbol.  In every one
        // of those cases the rename must abort wholesale — adding
        // cross-document edits for a locally-rejected rename would leak a
        // partial, inconsistent edit set into sibling documents.
        let local_rejected = edits.is_empty();
        if !edits.is_empty() {
            let lifted: Vec<TextEdit> = edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect();
            changes.insert(uri.clone(), lifted);
        }
        // Cross-document rename: rewrite the symbol's call /
        // definition sites in sibling documents.  Gated on the
        // same safety checks as the in-document path
        // (`is_safe_symbol_name`, no built-in shadow) so a
        // cross-doc rename can't produce an unsafe edit set — and
        // skipped entirely when the in-document rename was rejected.
        if !local_rejected
            && core_rename::is_safe_symbol_name(&new_name)
            && !core_rename::is_builtin_command_name(&new_name, &registry)
        {
            self.add_cross_document_rename_edits(
                &uri,
                &doc.text,
                &analysis,
                pos,
                &new_name,
                &mut changes,
            )
            .await;
        }
        if changes.is_empty() {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    /// `workspace/willRenameFiles` (GAP-A9): when `.tcl` files are
    /// renamed in the editor, rewrite every dependent file's `source`
    /// literal so the workspace still loads.  The pure core
    /// (`core_file_ops::compute_rename_edits`) returns byte-span edits
    /// keyed by dependent URI; here we resolve each span to an LSP
    /// range against the dependent's current text and assemble a
    /// `WorkspaceEdit` (one `TextDocumentEdit` per dependent, matching
    /// the Python `compute_batch_rename_edits`).
    async fn will_rename_files(
        &self,
        params: RenameFilesParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        // Collect the byte-span edits for every rename in the batch.
        let roots: Vec<String> = self
            .workspace_folder_urls()
            .await
            .iter()
            .map(Url::to_string)
            .collect();
        let raw_edits: Vec<core_file_ops::RenameEdit> = {
            let index = self.workspace_index.lock().await;
            params
                .files
                .iter()
                .flat_map(|f| {
                    core_file_ops::compute_rename_edits(&f.old_uri, &f.new_uri, &index, &roots)
                })
                .collect()
        };
        if raw_edits.is_empty() {
            return Ok(None);
        }
        // Group by dependent URI, resolving each span against that
        // document's source (open buffer or on-disk fallback).
        let mut by_dep: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for edit in raw_edits {
            let Ok(dep_url) = Url::parse(&edit.uri) else {
                continue;
            };
            let Some(doc) = self.read_document(&dep_url).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&doc.text);
            by_dep.entry(dep_url).or_default().push(TextEdit {
                range: lift_span(&doc.text, &line_index, edit.span),
                new_text: edit.new_text,
            });
        }
        if by_dep.is_empty() {
            return Ok(None);
        }
        let document_changes: Vec<TextDocumentEdit> = by_dep
            .into_iter()
            .map(|(uri, edits)| TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
                edits: edits.into_iter().map(OneOf::Left).collect(),
            })
            .collect();
        Ok(Some(WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(document_changes)),
            change_annotations: None,
        }))
    }

    /// `workspace/didRenameFiles` (GAP-A9): after the client applies a
    /// rename on disk, refresh the workspace index — drop the old URI's
    /// entries and re-index the renamed file from its new path so
    /// cross-document features (including future renames) stay current.
    /// Mirrors the Python `on_did_rename_files`.
    async fn did_rename_files(&self, params: RenameFilesParams) {
        for f in &params.files {
            if let Ok(old_url) = Url::parse(&f.old_uri) {
                self.workspace_index
                    .lock()
                    .await
                    .remove_document(old_url.as_str());
            }
            if let Ok(new_url) = Url::parse(&f.new_uri) {
                self.reindex_index_from_disk(&new_url).await;
            }
        }
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        if !self.feature_enabled("signatureHelp").await {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // Per-dialect cached registry — same source the
        // completion handler uses.  Threading it through lets
        // `S-signature-help-rich` surface signatures for
        // built-in commands (e.g. `puts`, `lsearch`) without
        // requiring a user proc with the same name.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_sig::signature_help(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("signature_help worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_signature_help))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        if !self.feature_enabled("hover").await {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // `S-hover-sync11`: consult the LRU(256) result cache
        // first.  Cache hits return the previous hover for
        // `(uri, line, character)` directly without re-running
        // the provider.  The cache is invalidated by URI in
        // `did_change` / `did_close`, so the stored entry is
        // always pinned to the source text the editor last sent.
        let cache_key = (uri.clone(), pos.line, pos.character);
        match self.hover_cache.lock().await.get(&cache_key) {
            HoverLookup::HitEmpty => return Ok(None),
            HoverLookup::Hit(h) => return Ok(Some(lift_hover(h))),
            HoverLookup::Miss => {}
        }
        // Cache miss: run the provider.  Consults the cached
        // analysis from `did_open` / `did_change`; the
        // `analysis_for` helper falls back to a fresh
        // `analyser.analyse(...)` when the publisher hasn't
        // populated the cache yet.  Worker-offload via
        // `spawn_blocking` keeps the LSP event loop responsive.
        // SYNC11's debounce + `[timing] hover` debug logs are
        // documented but not yet wired — they need an upgraded
        // logging layer beyond the bare `Client::log_message`.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_hover::hover(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("hover worker panicked: {err}").into(),
            data: None,
        })?;
        self.hover_cache.lock().await.put(cache_key, result.clone());
        Ok(result.map(lift_hover))
    }
}

fn lift_completion_kind(k: CoreCompletionKind) -> CompletionItemKind {
    match k {
        CoreCompletionKind::Variable => CompletionItemKind::VARIABLE,
        CoreCompletionKind::Function => CompletionItemKind::FUNCTION,
        CoreCompletionKind::EnumValue => CompletionItemKind::ENUM_MEMBER,
        CoreCompletionKind::Snippet => CompletionItemKind::SNIPPET,
    }
}

fn lift_completion_item(item: CoreCompletionItem, line: u32) -> CompletionItem {
    // An explicit single-line replacement edit (var / switch / array) — the
    // editor applies it verbatim so the `$`/`-` prefix isn't dropped/duplicated.
    let text_edit = item.text_edit.map(|e| {
        tower_lsp::lsp_types::CompletionTextEdit::Edit(tower_lsp::lsp_types::TextEdit {
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position::new(line, e.start_char),
                end: tower_lsp::lsp_types::Position::new(line, e.end_char),
            },
            new_text: e.new_text,
        })
    });
    let documentation = item
        .documentation
        .map(tower_lsp::lsp_types::Documentation::String);
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        documentation,
        sort_text: item.sort_text,
        // GAP-A9: snippet items carry VS Code tabstop syntax and
        // filter on their `tcl-…` prefix.
        insert_text_format: item
            .is_snippet
            .then_some(tower_lsp::lsp_types::InsertTextFormat::SNIPPET),
        filter_text: item.filter_text,
        text_edit,
        ..CompletionItem::default()
    }
}

/// Materialise the `tcl-lsp-core::selection_range` flat-vector
/// representation into a `tower_lsp` `SelectionRange` tree.
///
/// The chain in `core` is innermost first, with each link
/// pointing at its parent index; the LSP wire shape recurses
/// outward via `parent: Option<Box<SelectionRange>>`.
fn materialise_selection_range(
    chain: &[core_selection_range::SelectionRange],
) -> Option<SelectionRange> {
    if chain.is_empty() {
        return None;
    }
    // Build LSP records bottom-up so the parent box for index
    // `i` is already constructed when we wrap index `i-1`.
    let mut wrapped: Vec<Option<SelectionRange>> = chain.iter().map(|_| None).collect();
    // Process in reverse (outermost first) so each link's
    // parent — at a higher index — is already populated.
    for (i, link) in chain.iter().enumerate().rev() {
        let parent_box = link
            .parent_index
            .and_then(|p| wrapped[p].clone().map(Box::new));
        wrapped[i] = Some(SelectionRange {
            range: lift_lsp_range(link.range),
            parent: parent_box,
        });
    }
    wrapped.into_iter().next().flatten()
}

/// Byte offset of the LSP `(line, character)` cursor in `source`.
/// `character` is a UTF-16 code-unit offset. `None` when the line is
/// out of range.
fn line_col_to_byte_offset(source: &str, line: u32, col: u32) -> Option<usize> {
    let index = tcl_lexer::LineIndex::new(source);
    if line as usize >= index.line_count() {
        return None;
    }
    Some(index.offset_at_utf16(line, col, source) as usize)
}

/// Whether the cursor at `pos` sits on a command head (the first
/// word of a command), per the analyser's recorded command
/// invocations.  Used to gate the cross-document go-to-definition
/// fallback so it only fires on call sites, not on argument words
/// that happen to collide with a sibling proc/class name.
fn position_is_command_head(source: &str, pos: Position, analysis: &AnalysisResult) -> bool {
    let Some(offset) = line_col_to_byte_offset(source, pos.line, pos.character) else {
        return false;
    };
    analysis.command_invocations.iter().any(|inv| {
        let (start, end) = (inv.range.start() as usize, inv.range.end() as usize);
        offset >= start && offset < end
    })
}

fn lift_lsp_range(r: CoreLspRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

/// Deduplicate `Location`s by `(uri, range)`, preserving first-
/// seen order.  Used to merge current-document and cross-
/// document reference hits without double-listing a location.
fn dedup_locations(locations: &mut Vec<Location>) {
    let mut seen: std::collections::HashSet<(String, u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    locations.retain(|loc| {
        seen.insert((
            loc.uri.to_string(),
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        ))
    });
}

/// Lift the analyser's diagnostic records into the LSP wire
/// shape.  Shared by both the push-based `publish_diagnostics`
/// path (via `publish_analyser_diagnostics`) and the pull-
/// based `textDocument/diagnostic` handler.
/// Resolve a byte `span` to an LSP `Range` through `line_index`.
/// Shared by every `lift_*_diagnostics` helper so the offset →
/// (line, character) mapping is identical for analyser, compiler-
/// check, and optimiser diagnostics.
/// The startup optimisation profile: the `TCL_LSP_OPTIMISER_PROFILE` env
/// override (used by the fp/ground-truth battery harness to request the
/// `full` profile), else the editor default
/// ([`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]).
/// A `tclLsp.optimiser.profile` config pull overrides it at runtime.
fn default_optimiser_profile() -> tcl_compiler::optimiser::profiles::OptimisationProfile {
    std::env::var("TCL_LSP_OPTIMISER_PROFILE").map_or(
        tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE,
        |s| tcl_compiler::optimiser::profiles::OptimisationProfile::parse(&s),
    )
}

/// Map a `tclLsp.style.nonAscii` setting string to a [`NonAsciiMode`].
/// An unknown / absent value resolves to [`NonAsciiMode::Default`] (the
/// per-dialect auto behaviour).
fn parse_non_ascii_mode(s: &str) -> NonAsciiMode {
    match s {
        "off" => NonAsciiMode::Off,
        "strict" => NonAsciiMode::Strict,
        "confusables" => NonAsciiMode::Confusables,
        "common" => NonAsciiMode::Common,
        _ => NonAsciiMode::Default,
    }
}

/// Build a [`core_formatting::FormatterConfig`] from an LSP request's
/// [`FormattingOptions`].  An explicit client `tabSize` / `insertSpaces`
/// overrides the server's default indentation by LSP contract; every
/// other formatter knob keeps its default.
fn formatter_config_from_options(
    options: &tower_lsp::lsp_types::FormattingOptions,
) -> core_formatting::FormatterConfig {
    let indent_size = usize::try_from(options.tab_size).unwrap_or(4).max(1);
    let indent_style = if options.insert_spaces {
        core_formatting::IndentStyle::Spaces
    } else {
        core_formatting::IndentStyle::Tabs
    };
    core_formatting::FormatterConfig {
        indent_size,
        indent_style,
        continuation_indent: indent_size,
        ..core_formatting::FormatterConfig::default()
    }
}

/// Resolve the user config-file path for `tcl-lsp.exportConfig`:
/// `$XDG_CONFIG_HOME/tcl-lsp/config.ini`, falling back to
/// `$HOME/.config/tcl-lsp/config.ini`.  Both env vars are honoured on every
/// platform so tests can isolate the server's config via `XDG_CONFIG_HOME`.
fn config_ini_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("tcl-lsp").join("config.ini")
}

/// Render a [`NonAsciiMode`] for `getEffectiveConfig`.  The unset
/// [`NonAsciiMode::Default`] reports as JSON `null` (the per-dialect
/// auto behaviour, mirroring the Python server's `None`); every explicit
/// mode reports its setting string.
fn non_ascii_mode_str(mode: NonAsciiMode) -> serde_json::Value {
    let label = match mode {
        NonAsciiMode::Default => return serde_json::Value::Null,
        NonAsciiMode::Off => "off",
        NonAsciiMode::Strict => "strict",
        NonAsciiMode::Confusables => "confusables",
        NonAsciiMode::Common => "common",
    };
    serde_json::Value::String(label.to_owned())
}

/// Extract `tclLsp.style.nonAscii` from an LSP settings payload, accepting
/// both the nested (`{"tclLsp":{"style":{"nonAscii":"…"}}}`) and the
/// flat-dotted (`{"tclLsp.style.nonAscii":"…"}`) shapes editors send.
fn settings_non_ascii_mode(settings: &serde_json::Value) -> Option<NonAsciiMode> {
    let nested = settings
        .get("tclLsp")
        .and_then(|v| v.get("style"))
        .and_then(|v| v.get("nonAscii"));
    nested
        .or_else(|| settings.get("tclLsp.style.nonAscii"))
        .and_then(serde_json::Value::as_str)
        .map(parse_non_ascii_mode)
}

/// Extract the disabled diagnostic codes from a settings payload — the
/// `tclLsp.diagnostics.<CODE>` booleans whose value is `false`. Accepts
/// the nested object (`{"tclLsp":{"diagnostics":{"W001":false}}}`) and
/// the flat-dotted (`{"tclLsp.diagnostics.W001":false}`) shapes. Returns
/// `None` when no diagnostics config is present (so the caller leaves the
/// current set untouched).
fn settings_disabled_diagnostics(settings: &serde_json::Value) -> Option<HashSet<String>> {
    if let Some(map) = settings
        .get("tclLsp")
        .and_then(|v| v.get("diagnostics"))
        .and_then(serde_json::Value::as_object)
    {
        return Some(
            map.iter()
                .filter(|(_, v)| v.as_bool() == Some(false))
                .map(|(k, _)| k.clone())
                .collect(),
        );
    }
    let obj = settings.as_object()?;
    let mut set = HashSet::new();
    let mut found = false;
    for (k, v) in obj {
        if let Some(code) = k.strip_prefix("tclLsp.diagnostics.") {
            found = true;
            if v.as_bool() == Some(false) {
                set.insert(code.to_owned());
            }
        }
    }
    found.then_some(set)
}

/// Apply one LSP content change to `text` (incremental document sync).
///
/// A `None` range is a full-document replacement; a `Some` range is a
/// ranged edit whose `(line, character)` UTF-16 positions are resolved to
/// byte offsets via [`tcl_lexer::LineIndex::offset_at_utf16`] and spliced.
/// Offsets are clamped and ordered so the splice indices are always valid
/// char boundaries within `text`.
fn apply_content_change(text: &str, range: Option<Range>, new_text: &str) -> String {
    let Some(range) = range else {
        return new_text.to_owned();
    };
    let index = tcl_lexer::LineIndex::new(text);
    let a = index.offset_at_utf16(range.start.line, range.start.character, text) as usize;
    let b = index.offset_at_utf16(range.end.line, range.end.character, text) as usize;
    let len = text.len();
    let start = a.min(b).min(len);
    let end = a.max(b).min(len);
    let mut out = String::with_capacity(len - (end - start) + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[end..]);
    out
}

fn lift_span(source: &str, line_index: &tcl_lexer::LineIndex, span: tcl_lexer::Span) -> Range {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Range {
        start: Position {
            line: start.line,
            character: start.character,
        },
        end: Position {
            line: end.line,
            character: end.character,
        },
    }
}

/// Diagnostic codes that are *default-off* in the editor catalogue —
/// the analyser emits them (matching the Python `analyse` contract) but
/// the LSP layer is the consuming filter, so they are dropped from the
/// published set unless explicitly enabled.  Mirrors Python's
/// default-disabled code handling; the Rust server has no per-code
/// enable config yet, so these are simply never published.
const DEFAULT_OFF_CODES: &[&str] = &["W242"];

fn lift_analyser_diagnostics(
    text: &str,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new(text);
    diagnostics
        .iter()
        .filter(|d| !DEFAULT_OFF_CODES.contains(&d.code.as_str()))
        .cloned()
        .map(|d| tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, d.span),
            severity: Some(match d.severity {
                tcl_compiler::analyser::Severity::Error => {
                    tower_lsp::lsp_types::DiagnosticSeverity::ERROR
                }
                tcl_compiler::analyser::Severity::Warning => {
                    tower_lsp::lsp_types::DiagnosticSeverity::WARNING
                }
                tcl_compiler::analyser::Severity::Hint
                | tcl_compiler::analyser::Severity::Suggestion => {
                    tower_lsp::lsp_types::DiagnosticSeverity::HINT
                }
            }),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(d.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// GAP-C1 strip 2: lift the source-style pass (W111 line length,
/// W112 trailing whitespace, W115 comment continuation, W118 line
/// endings) into LSP diagnostics.  These are pure source-text
/// checks (no analyser / compiler unit needed); see
/// `tcl_lsp_core::source_style`.
///
/// `suppressed` is the analyser's `suppressed_lines` map — it
/// carries both inline `# noqa` line suppressions and the
/// file-level (`-1`) `# tcl-lsp: disable=…` directive set, so the
/// style pass honours the same suppression the analyser diagnostics
/// do.  The checks run with the Python defaults (line length 120,
/// expected line ending `\n`); a per-check feature-config surface is
/// a documented GAP-C1 strip-2 follow-up.
fn lift_source_style_diagnostics(
    text: &str,
    suppressed: &std::collections::HashMap<i32, std::collections::HashSet<String>>,
    user_disabled: &std::collections::HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tcl_lsp_core::source_style::{
        style_diagnostics, StyleSeverity, DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH,
    };
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

    // The file-level (`-1`) directive bucket doubles as a per-code
    // disabled set (mirrors how the Rust analyser folds a
    // `# tcl-lsp: disable=…` directive into both `suppressed_lines`
    // and its internal `disabled_diagnostics`); union it with the
    // user's `tclLsp.diagnostics.<CODE> = false` settings so W111 /
    // W112 / W115 / W118 can be turned off from the editor.
    let mut disabled = suppressed.get(&-1).cloned().unwrap_or_default();
    disabled.extend(user_disabled.iter().cloned());

    style_diagnostics(
        text,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &disabled,
        suppressed,
    )
    .into_iter()
    .map(|d| tower_lsp::lsp_types::Diagnostic {
        range: Range {
            start: Position {
                line: d.range.start_line,
                character: d.range.start_character,
            },
            end: Position {
                line: d.range.end_line,
                character: d.range.end_character,
            },
        },
        severity: Some(match d.severity {
            StyleSeverity::Warning => DiagnosticSeverity::WARNING,
            StyleSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(d.code.to_string())),
        code_description: None,
        source: Some("tcl-lsp".to_string()),
        message: d.message,
        related_information: None,
        tags: None,
        data: None,
    })
    .collect()
}

/// GAP-C1: lift the compiler-checks pipeline (GVN redundancies,
/// shimmer / thunking, taint W2xx / T1xx, iRules control-flow
/// IRULE1xxx-5xxx, SCCP constant branches) **and** the optimiser
/// O-codes into LSP diagnostics.  These analyses are implemented in
/// `tcl-compiler` but were previously only reachable through the
/// `PyO3` bridge — the native server published nothing from them.
///
/// `dialect` is the resolved per-document dialect string (empty for
/// plain Tcl); `registry` must already carry that dialect's specs
/// loaded (the caller resolves it via `registry_for_dialect`).
///
/// Note: this builds a `CompilationUnit` for the checks and the
/// optimiser builds its own internally, so the source is lowered
/// twice.  That is acceptable for the initial wiring (the analyses
/// themselves are the cost); sharing a single lowered unit across
/// both is a follow-up once the document-store lands.
fn lift_compiler_diagnostics(
    text: &str,
    registry: &CommandRegistry,
    dialect: &str,
    optimiser_enabled: bool,
    disabled_optimisations: &std::collections::HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::{run_all_checks, Severity as CheckSeverity};
    use tcl_compiler::optimiser::optimise_with_dialect;
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

    let line_index = tcl_lexer::LineIndex::new(text);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    let mut out: Vec<tower_lsp::lsp_types::Diagnostic> = Vec::new();

    // Compiler checks: GVN / shimmer / thunking / taint / iRules-flow
    // / SCCP, all keyed off a single interprocedurally-summarised
    // compilation unit (mirrors the `compiler_checks_run_all` PyO3
    // bridge's construction).
    let cu = CompilationUnit::build_for_with_config(
        text,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    )
    .with_interprocedural(registry, dialect_opt);
    // An optimiser O-code (`O1xx`) is gated by the `tclLsp.optimiser.enabled`
    // master switch and the profile + per-code `disabled_optimisations` set,
    // wherever it is emitted (some — e.g. the constant-branch `O100` — come
    // from `run_all_checks` rather than `optimise_with_dialect`). Mirrors
    // Python, whose `optimiser_enabled` gate covers every O-code.
    let optimiser_suppressed = |code: &str| {
        code.starts_with('O') && (!optimiser_enabled || disabled_optimisations.contains(code))
    };
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if optimiser_suppressed(&d.code) {
            continue;
        }
        out.push(tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, d.span),
            severity: Some(match d.severity {
                CheckSeverity::Error => DiagnosticSeverity::ERROR,
                CheckSeverity::Warning => DiagnosticSeverity::WARNING,
                CheckSeverity::Hint | CheckSeverity::Suggestion => DiagnosticSeverity::HINT,
            }),
            code: Some(NumberOrString::String(d.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Optimiser O-codes — actionable + hint-only rewrites, surfaced
    // as HINT-severity suggestions (the editor renders the code-action
    // fix from the diagnostic; the fix plumbing itself is GAP-C3). The
    // `tclLsp.optimiser.enabled=false` master switch suppresses the whole
    // block, mirroring Python's `if optimiser_enabled:` gate.
    for o in optimise_with_dialect(text, registry, dialect_opt)
        .into_iter()
        .filter(|_| optimiser_enabled)
    {
        // Profile + per-code gate: the active profile disables whole O-code
        // categories (the default `readability` profile surfaces only
        // readability rewrites) and per-code `tclLsp.optimiser.O1xx=false`
        // overrides add to that. Mirrors the Python server's
        // `disabled_optimisations` filter.
        if disabled_optimisations.contains(o.code.as_str()) {
            continue;
        }
        // Surface the fold/rewrite text as the quick-fix `data.replacement`
        // (mirrors Python's `Optimisation.replacement` -> diagnostic data), so
        // editors and the e2e battery can apply the suggested replacement.
        let data = (!o.replacement.is_empty()).then(|| {
            serde_json::json!({
                "replacement": o.replacement,
                "startOffset": o.span.start(),
                "endOffset": o.span.end(),
            })
        });
        out.push(tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, o.span),
            severity: Some(DiagnosticSeverity::HINT),
            code: Some(NumberOrString::String(o.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: o.message,
            related_information: None,
            tags: None,
            data,
        });
    }

    out
}

/// Build an empty `DocumentDiagnosticReportResult` for callers
/// that hit a no-document or no-analysis path.
fn empty_diagnostic_report() -> DocumentDiagnosticReportResult {
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: Vec::new(),
            },
        },
    ))
}

/// Resolve the per-folder dialect override for the given URI. The
/// longest matching folder prefix wins so a nested folder mapping
/// shadows its parent. Returns `None` when no folder covers the
/// URI, in which case the caller falls back to the session
/// default.
fn folder_dialect_for(uri: &Url, folders: &[(Url, String)]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let target = uri.as_str();
    for (folder, dialect) in folders {
        if !uri_under_folder(target, folder.as_str()) {
            continue;
        }
        let len = folder.as_str().len();
        let take = match best {
            Some((existing, _)) => len > existing,
            None => true,
        };
        if take {
            best = Some((len, dialect.as_str()));
        }
    }
    best.map(|(_, d)| d.to_owned())
}

/// Upper bound on the number of files the on-disk workspace scan
/// will analyse, so a pathologically large tree can't stall
/// start-up.  Open documents are always indexed regardless of
/// this cap (they flow through `publish_analyser_diagnostics`).
const WORKSPACE_SCAN_FILE_CAP: usize = 2000;

/// `true` when `path` names a directory the workspace scan should
/// not descend into: hidden directories (dot-prefixed) and the
/// common vendor / build / scratch trees that never hold
/// first-party Tcl sources worth indexing.
fn is_skipped_scan_dir(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with('.') || matches!(name, "node_modules" | "target" | "tmp"),
        // No representable file name (e.g. `..`): skip to be safe.
        None => true,
    }
}

/// `true` when `path` has a Tcl source extension the analyser can
/// usefully index (`.tcl` scripts and `.tm` Tcl modules).
fn is_tcl_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("tcl" | "tm")
    )
}

/// Iteratively walk `root`, appending the paths of Tcl source
/// files to `out` until `out` reaches `cap` entries.  Skips the
/// directories [`is_skipped_scan_dir`] rejects and silently
/// ignores unreadable directories so a single permission error
/// doesn't abort the whole scan.  Iterative (explicit stack) to
/// avoid unbounded recursion on deep trees.
fn collect_tcl_files(root: &Path, cap: usize, out: &mut Vec<PathBuf>) {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= cap {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                if !is_skipped_scan_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() && is_tcl_source(&path) {
                if out.len() >= cap {
                    return;
                }
                out.push(path);
            }
        }
    }
}

/// `true` when `uri` sits under the workspace folder `folder`.
///
/// Performs the directory-boundary check the prefix-only
/// `target.starts_with(folder)` form gets wrong — without
/// the boundary check `file:///workspace/app` would also
/// match `file:///workspace/app2/...` (PR #454 Codex review P2).
/// The match succeeds when:
///
/// * `uri == folder`, OR
/// * `uri == folder` with the folder's trailing slash trimmed, OR
/// * `uri` starts with `folder + "/"`.
fn uri_under_folder(uri: &str, folder: &str) -> bool {
    let folder_trimmed = folder.trim_end_matches('/');
    if uri == folder_trimmed || uri == folder {
        return true;
    }
    // The boundary check: the byte immediately after the
    // folder prefix in `uri` must be `/`.  Compare against the
    // trimmed form so a trailing slash on `folder` doesn't
    // change the behaviour.
    let needle = format!("{folder_trimmed}/");
    uri.starts_with(&needle)
}

/// Return a fresh `result_id` for a semantic-tokens response.
/// Monotonically increasing via an atomic counter so each
/// snapshot we hand out is uniquely identifiable, letting
/// clients consult our cache via
/// `semanticTokens/full/delta { previousResultId }`.
/// Build the `ServerCapabilities` advertised in the response
/// to `initialize`.  Kept as a free function so the
/// `LanguageServer::initialize` handler stays focused on
/// state setup and result construction — the long capability
/// literal lives here rather than inside the trait method.
fn build_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["$".to_owned()]),
            ..CompletionOptions::default()
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![" ".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        // Note: tower-lsp 0.20 does not expose
        // `type_hierarchy_provider` on `ServerCapabilities`;
        // the type-hierarchy core provider lives in
        // `tcl-lsp-core::type_hierarchy` and will be wired
        // once we upgrade tower-lsp (tracked under
        // `S-type-hierarchy-rich`).
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        linked_editing_range_provider: Some(
            tower_lsp::lsp_types::LinkedEditingRangeServerCapabilities::Simple(true),
        ),
        // `S-diagnostics-pipeline`: advertise pull-based
        // diagnostics so editors that support it can request
        // diagnostics on demand alongside the existing
        // push-based `publish_diagnostics`.
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: Some("tcl-lsp".to_string()),
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        // Editor-invoked workspace commands (currently the
        // minify-document command family).
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![
                "tcl-lsp.minifyDocument".to_owned(),
                "tcl-lsp.optimiseDocument".to_owned(),
                "tcl-lsp.unminifyError".to_owned(),
                "tcl-lsp.describeIruleEvent".to_owned(),
                "tcl-lsp.describeIruleCommand".to_owned(),
                "tcl-lsp.listIruleEvents".to_owned(),
                "tcl-lsp.diagramData".to_owned(),
                "tcl-lsp.getEffectiveConfig".to_owned(),
                "tcl-lsp.fixAllSafeIssues".to_owned(),
                "tcl-lsp.listSubcommands".to_owned(),
                "tcl-lsp.listKnownPackages".to_owned(),
                "tcl-lsp.suggestPackagesForSymbol".to_owned(),
                "tcl-lsp.exportConfig".to_owned(),
            ],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        // `S-workspace-file-ops` (GAP-A9): advertise willRename /
        // didRename so the editor consults us before/after a `.tcl`
        // rename — the willRename handler rewrites dependents' `source`
        // literals, the didRename handler reindexes the moved file.
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: None,
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                will_rename: Some(rename_file_operation_options()),
                did_rename: Some(rename_file_operation_options()),
                ..WorkspaceFileOperationsServerCapabilities::default()
            }),
        }),
        ..ServerCapabilities::default()
    }
}

/// File-operation filter for `willRename` / `didRename`: match the Tcl
/// source extensions (`.tcl` / `.tm` / `.itcl` / `.irule` / `.irul`),
/// mirroring the Python `_RENAME_FILE_OPERATION_OPTIONS` glob.
fn rename_file_operation_options() -> FileOperationRegistrationOptions {
    FileOperationRegistrationOptions {
        filters: vec![FileOperationFilter {
            scheme: Some("file".to_owned()),
            pattern: FileOperationPattern {
                glob: "**/*.{tcl,tm,itcl,irule,irul}".to_owned(),
                matches: None,
                options: None,
            },
        }],
    }
}

/// Build the semantic-tokens capability advertising the
/// classifier's legend, range support, and delta support.
fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: core_semantic_tokens::legend_token_types()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenType::new)
                .collect(),
            token_modifiers: core_semantic_tokens::legend_token_modifiers()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenModifier::new)
                .collect(),
        },
        range: Some(true),
        // Delta support — the handler implements the minimal
        // valid shape (returns full tokens when
        // previousResultId doesn't match, otherwise empty
        // edits).
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
    })
}

fn next_semantic_tokens_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("st-{id}")
}

/// Convert the core's `Vec<u32>` packed semantic-tokens
/// stream (`[deltaLine, deltaCol, length, type, modifiers]`
/// per token) into a `Vec<SemanticToken>` for the wire layer.
fn lift_semantic_token_data(data: &[u32]) -> Vec<tower_lsp::lsp_types::SemanticToken> {
    let mut tokens: Vec<tower_lsp::lsp_types::SemanticToken> = Vec::with_capacity(data.len() / 5);
    for chunk in data.chunks_exact(5) {
        tokens.push(tower_lsp::lsp_types::SemanticToken {
            delta_line: chunk[0],
            delta_start: chunk[1],
            length: chunk[2],
            token_type: chunk[3],
            token_modifiers_bitset: chunk[4],
        });
    }
    tokens
}

fn lift_parameter_information(p: CoreParameterInformation) -> ParameterInformation {
    ParameterInformation {
        label: ParameterLabel::Simple(p.label),
        documentation: None,
    }
}

fn lift_signature_information(s: CoreSignatureInformation) -> SignatureInformation {
    SignatureInformation {
        label: s.label,
        documentation: s.documentation.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        parameters: Some(
            s.parameters
                .into_iter()
                .map(lift_parameter_information)
                .collect(),
        ),
        active_parameter: None,
    }
}

fn lift_signature_help(h: CoreSignatureHelp) -> SignatureHelp {
    SignatureHelp {
        signatures: h
            .signatures
            .into_iter()
            .map(lift_signature_information)
            .collect(),
        active_signature: Some(h.active_signature),
        active_parameter: Some(h.active_parameter),
    }
}

fn lift_hover(h: CoreHover) -> Hover {
    let kind = match h.kind {
        CoreHoverKind::Markdown => MarkupKind::Markdown,
    };
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind,
            value: h.value,
        }),
        range: None,
    }
}

fn lift_line_range(r: core_symbols::LineRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

fn lift_symbol_kind(k: CoreSymbolKind) -> SymbolKind {
    match k {
        CoreSymbolKind::Function => SymbolKind::FUNCTION,
        CoreSymbolKind::Method => SymbolKind::METHOD,
        CoreSymbolKind::Class => SymbolKind::CLASS,
        CoreSymbolKind::Property => SymbolKind::PROPERTY,
        CoreSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
        CoreSymbolKind::Namespace => SymbolKind::NAMESPACE,
        CoreSymbolKind::Variable => SymbolKind::VARIABLE,
        CoreSymbolKind::Module => SymbolKind::MODULE,
    }
}

fn lift_document_symbol(s: core_symbols::DocumentSymbol) -> DocumentSymbol {
    let children = if s.children.is_empty() {
        None
    } else {
        Some(s.children.into_iter().map(lift_document_symbol).collect())
    };
    #[allow(deprecated)] // `deprecated` is required by lsp-types but unused.
    DocumentSymbol {
        name: s.name,
        detail: s.detail,
        kind: lift_symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        range: lift_line_range(s.range),
        selection_range: lift_line_range(s.selection_range),
        children,
    }
}

fn lift_folding_range(r: tcl_lsp_core::folding::FoldingRange) -> FoldingRange {
    FoldingRange {
        start_line: r.start_line,
        start_character: None,
        end_line: r.end_line,
        end_character: None,
        kind: Some(match r.kind {
            FoldKind::Region => FoldingRangeKind::Region,
            FoldKind::Comment => FoldingRangeKind::Comment,
        }),
        collapsed_text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{
        PartialResultParams, ReferenceContext, TextDocumentIdentifier, WorkDoneProgressParams,
    };

    #[test]
    fn default_off_w242_is_not_published() {
        // `while {$x < 10} {puts hi}` emits the default-off W242 hint
        // from the analyser, which the lift layer must drop (no consuming
        // config exists in the Rust server) while keeping other codes.
        let src = "while {$x < 10} {puts hi}\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "W242"),
            "analyser should still emit W242"
        );
        let lifted = lift_analyser_diagnostics(src, &analysis.diagnostics);
        assert!(
            !lifted.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "W242"
            )),
            "W242 must be filtered from the published set"
        );
    }

    /// GAP-C1: the constant-true `if` is folded by SCCP and surfaced
    /// as an `O100` constant-branch diagnostic from `run_all_checks`.
    /// The base analyser never emits it, so a non-empty result with an
    /// O-code proves `lift_compiler_diagnostics` carries the compiler-
    /// check pipeline's output into the published diagnostic set.
    #[test]
    fn lift_compiler_diagnostics_surfaces_compiler_check_codes() {
        let registry = CommandRegistry::build_default();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let diags =
            lift_compiler_diagnostics(src, &registry, "", true, &std::collections::HashSet::new());
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "O100"
            )),
            "expected an O100 constant-branch diagnostic, got: {:?}",
            diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
        // Every lifted diagnostic carries the shared source tag.
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    #[test]
    fn lift_compiler_diagnostics_honours_optimiser_master_switch_and_per_code() {
        let registry = CommandRegistry::build_default();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let is_o100 = |d: &tower_lsp::lsp_types::Diagnostic| matches!(&d.code, Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "O100");
        // Master switch off: no optimiser O-codes at all (compiler checks still run).
        let off =
            lift_compiler_diagnostics(src, &registry, "", false, &std::collections::HashSet::new());
        assert!(
            !off.iter().any(is_o100),
            "O100 must be suppressed when the optimiser master switch is off: {:?}",
            off.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
        // Per-code disable: O100 specifically suppressed even with the optimiser on.
        let mut disabled = std::collections::HashSet::new();
        disabled.insert("O100".to_string());
        let per_code = lift_compiler_diagnostics(src, &registry, "", true, &disabled);
        assert!(
            !per_code.iter().any(is_o100),
            "O100 must be suppressed when disabled per-code: {:?}",
            per_code.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    /// An iRules taint flow (`HTTP::uri` → `HTTP::respond`) is an
    /// IRULE3001 the base analyser does not emit; the dialect-aware
    /// registry path must surface it through `lift_compiler_diagnostics`.
    #[test]
    fn lift_compiler_diagnostics_surfaces_irules_taint_flow() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
        let diags = lift_compiler_diagnostics(
            src,
            &registry,
            "f5-irules",
            true,
            &std::collections::HashSet::new(),
        );
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "IRULE3001"
            )),
            "expected IRULE3001, got: {:?}",
            diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn parse_non_ascii_mode_maps_settings() {
        assert_eq!(parse_non_ascii_mode("off"), NonAsciiMode::Off);
        assert_eq!(parse_non_ascii_mode("strict"), NonAsciiMode::Strict);
        assert_eq!(
            parse_non_ascii_mode("confusables"),
            NonAsciiMode::Confusables
        );
        assert_eq!(parse_non_ascii_mode("common"), NonAsciiMode::Common);
        assert_eq!(parse_non_ascii_mode("bogus"), NonAsciiMode::Default);
    }

    #[test]
    fn settings_non_ascii_mode_nested_and_flat() {
        let nested = serde_json::json!({"tclLsp": {"style": {"nonAscii": "common"}}});
        assert_eq!(settings_non_ascii_mode(&nested), Some(NonAsciiMode::Common));
        let flat = serde_json::json!({"tclLsp.style.nonAscii": "off"});
        assert_eq!(settings_non_ascii_mode(&flat), Some(NonAsciiMode::Off));
        let none = serde_json::json!({"tclLsp": {"dialect": "tcl9.0"}});
        assert_eq!(settings_non_ascii_mode(&none), None);
    }

    #[test]
    fn settings_disabled_diagnostics_nested_and_flat() {
        let nested = serde_json::json!({
            "tclLsp": {"diagnostics": {"W001": true, "W108": false, "W111": false}}
        });
        let got = settings_disabled_diagnostics(&nested).unwrap();
        assert!(got.contains("W108") && got.contains("W111") && !got.contains("W001"));
        let flat = serde_json::json!({
            "tclLsp.diagnostics.W210": false, "tclLsp.diagnostics.W211": true
        });
        let got = settings_disabled_diagnostics(&flat).unwrap();
        assert!(got.contains("W210") && !got.contains("W211"));
        // No diagnostics config -> None (leave current set untouched).
        assert!(settings_disabled_diagnostics(&serde_json::json!({"x": 1})).is_none());
    }

    #[test]
    fn feature_toggles_default_on_and_merge() {
        let mut toggles = FeatureToggles::default();
        // Absent keys default to enabled.
        assert!(toggles.is_enabled("hover"));
        assert!(toggles.is_enabled("completion"));
        // Applying a partial `features` object sets only the named keys.
        let features = serde_json::json!({"hover": false, "folding": false});
        toggles.apply(features.as_object().unwrap());
        assert!(!toggles.is_enabled("hover"));
        assert!(!toggles.is_enabled("folding"));
        // Unnamed keys keep their default-on value.
        assert!(toggles.is_enabled("completion"));
        // Re-enabling merges (does not reset the rest).
        toggles.apply(serde_json::json!({"hover": true}).as_object().unwrap());
        assert!(toggles.is_enabled("hover"));
        assert!(!toggles.is_enabled("folding"));
    }

    #[test]
    fn feature_toggles_resolved_map_covers_known_keys() {
        let mut toggles = FeatureToggles::default();
        toggles.apply(serde_json::json!({"hover": false}).as_object().unwrap());
        let map = toggles.resolved_map();
        // Every advertised key is present and boolean.
        for key in FeatureToggles::KEYS {
            assert_eq!(
                map.get(*key).and_then(serde_json::Value::as_bool),
                Some(*key != "hover"),
                "feature {key}"
            );
        }
        // The toggleable keys the e2e contract checks are all reported.
        for key in [
            "hover",
            "completion",
            "documentSymbols",
            "definition",
            "references",
            "signatureHelp",
            "folding",
            "selectionRange",
            "documentLinks",
        ] {
            assert!(map.contains_key(key), "missing reported feature {key}");
        }
    }

    #[test]
    fn non_ascii_mode_str_renders_null_for_default() {
        assert!(non_ascii_mode_str(NonAsciiMode::Default).is_null());
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Off), "off");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Strict), "strict");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Common), "common");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Confusables), "confusables");
    }

    #[test]
    fn suggest_packages_echoes_symbol_with_list() {
        let args = vec![serde_json::json!("json::write")];
        let out = Backend::suggest_packages_for_symbol_command(&args);
        assert_eq!(
            out.get("symbol").and_then(|v| v.as_str()),
            Some("json::write")
        );
        assert!(out
            .get("suggestions")
            .is_some_and(serde_json::Value::is_array));
    }

    #[test]
    fn list_known_packages_reports_a_list() {
        let out = Backend::list_known_packages_command();
        assert!(out.get("packages").is_some_and(serde_json::Value::is_array));
    }

    #[test]
    fn config_ini_path_honours_xdg() {
        // `config_ini_path` reads process env; guard the global with a mutex so
        // concurrent env-mutating tests don't race.
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-probe");
        let path = config_ini_path();
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/xdg-probe/tcl-lsp/config.ini")
        );
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn formatter_config_honours_request_indent() {
        let mut opts = tower_lsp::lsp_types::FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        let cfg = formatter_config_from_options(&opts);
        assert_eq!(cfg.indent_size, 2);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
        // A zero tabSize is clamped to a usable minimum.
        opts.tab_size = 0;
        assert_eq!(formatter_config_from_options(&opts).indent_size, 1);
        // insertSpaces=false selects tab indentation.
        opts.tab_size = 4;
        opts.insert_spaces = false;
        let cfg = formatter_config_from_options(&opts);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Tabs);
    }

    #[test]
    fn configured_analyser_threads_mode_and_disabled() {
        // `Off` mode suppresses W108 entirely.
        let mut a = Backend::configured_analyser(HashSet::new(), NonAsciiMode::Off);
        let r = a.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W108"));
        // A disabled code is filtered from the analyser's output.
        let mut disabled = HashSet::new();
        disabled.insert("W108".to_string());
        let mut b = Backend::configured_analyser(disabled, NonAsciiMode::Strict);
        let r = b.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W108"));
    }

    #[test]
    fn apply_content_change_full_replacement() {
        // A `None` range replaces the whole document.
        assert_eq!(apply_content_change("old text", None, "new"), "new");
    }

    #[test]
    fn apply_content_change_ranged_edits() {
        // Replace "hi" with "bye" on line 1.
        let text = "set x 1\nputs hi\n";
        let r = Range {
            start: pos(1, 5),
            end: pos(1, 7),
        };
        assert_eq!(
            apply_content_change(text, Some(r), "bye"),
            "set x 1\nputs bye\n"
        );

        // Pure insertion (empty range) at the start of line 1.
        let r = Range {
            start: pos(1, 0),
            end: pos(1, 0),
        };
        assert_eq!(
            apply_content_change(text, Some(r), "# "),
            "set x 1\n# puts hi\n",
        );

        // Multi-line deletion: drop from mid-line-0 through mid-line-1.
        let r = Range {
            start: pos(0, 4),
            end: pos(1, 5),
        };
        assert_eq!(apply_content_change(text, Some(r), ""), "set hi\n");
    }

    #[test]
    fn lsp_position_helpers_use_utf16_columns() {
        let text = "é😀x\n";
        assert_eq!(line_col_to_byte_offset(text, 0, 0), Some(0));
        assert_eq!(line_col_to_byte_offset(text, 0, 1), Some(2));
        assert_eq!(line_col_to_byte_offset(text, 0, 3), Some(6));

        let line_index = tcl_lexer::LineIndex::new(text);
        let range = lift_span(text, &line_index, tcl_lexer::Span::new(6, 7));
        assert_eq!(range.start, pos(0, 3));
        assert_eq!(range.end, pos(0, 4));
    }

    #[test]
    fn apply_content_change_sequence_matches_full_text() {
        // Applying a sequence of ranged edits (as INCREMENTAL sync
        // delivers them) reproduces the intended final document.
        let mut text = "abc\ndef\n".to_string();
        // Edit 1: insert "X" at (0,1) -> "aXbc".
        text = apply_content_change(
            &text,
            Some(Range {
                start: pos(0, 1),
                end: pos(0, 1),
            }),
            "X",
        );
        assert_eq!(text, "aXbc\ndef\n");
        // Edit 2: replace "def" with "ghij".
        text = apply_content_change(
            &text,
            Some(Range {
                start: pos(1, 0),
                end: pos(1, 3),
            }),
            "ghij",
        );
        assert_eq!(text, "aXbc\nghij\n");
    }

    /// GAP-C1 strip 2: the source-style pass must reach the
    /// published set.  A long line + trailing whitespace + CRLF
    /// endings exercise W111 / W112 / W118 — none of which the
    /// analyser or compiler-check pipelines emit — so a non-empty
    /// result with those codes proves `lift_source_style_diagnostics`
    /// is wired in.
    #[test]
    fn lift_source_style_diagnostics_surfaces_style_codes() {
        let long = "x".repeat(130);
        let src = format!("{long}  \r\nputs ok\r\n");
        let diags = lift_source_style_diagnostics(
            &src,
            &std::collections::HashMap::new(),
            &std::collections::HashSet::new(),
        );
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect();
        for want in ["W111", "W112", "W118"] {
            assert!(
                codes.iter().any(|c| c == want),
                "expected {want}, got: {codes:?}",
            );
        }
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    /// A file-level `# tcl-lsp: disable=W111` directive (recorded by
    /// the analyser against the `-1` suppression bucket) must drop
    /// W111 from the lifted style set while leaving the other style
    /// codes intact.
    #[test]
    fn lift_source_style_diagnostics_honours_file_suppression() {
        let long = "y".repeat(130);
        let src = format!("{long}  \n");
        let mut suppressed: std::collections::HashMap<i32, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        suppressed.insert(-1, std::iter::once("W111".to_string()).collect());
        let diags =
            lift_source_style_diagnostics(&src, &suppressed, &std::collections::HashSet::new());
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !codes.iter().any(|c| c == "W111"),
            "W111 should be suppressed"
        );
        assert!(codes.iter().any(|c| c == "W112"), "W112 should remain");
    }

    #[test]
    fn dialect_from_language_id_recognises_editor_ids() {
        assert_eq!(
            Backend::dialect_from_language_id("tcl-irule"),
            Some("f5-irules"),
        );
        assert_eq!(
            Backend::dialect_from_language_id("tcl-iapp"),
            Some("f5-iapps"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl"), Some("tcl8.6"));
    }

    #[test]
    fn dialect_from_language_id_passes_through_canonical_names() {
        assert_eq!(
            Backend::dialect_from_language_id("f5-irules"),
            Some("f5-irules"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl9.0"), Some("tcl9.0"),);
        assert_eq!(
            Backend::dialect_from_language_id("synopsys-eda-tcl"),
            Some("synopsys-eda-tcl"),
        );
    }

    #[test]
    fn dialect_from_language_id_returns_none_for_unknown_ids() {
        assert!(Backend::dialect_from_language_id("plaintext").is_none());
        assert!(Backend::dialect_from_language_id("").is_none());
        // ``tcl-bigip`` is a BIG-IP config language id with no Tcl
        // dialect counterpart — falls back to the session default.
        assert!(Backend::dialect_from_language_id("tcl-bigip").is_none());
    }

    /// The folding-range lift maps `FoldKind::Comment` to the LSP
    /// `Comment` kind and `FoldKind::Region` to `Region`. The
    /// `start_character` / `end_character` fields stay `None`
    /// because the core provider returns line-only ranges.
    #[test]
    fn lift_folding_range_preserves_kind_and_lines() {
        let r = tcl_lsp_core::folding::FoldingRange {
            start_line: 2,
            end_line: 5,
            kind: FoldKind::Region,
        };
        let lifted = lift_folding_range(r);
        assert_eq!(lifted.start_line, 2);
        assert_eq!(lifted.end_line, 5);
        assert_eq!(lifted.kind, Some(FoldingRangeKind::Region));
        assert!(lifted.start_character.is_none());
        assert!(lifted.end_character.is_none());

        let comment = tcl_lsp_core::folding::FoldingRange {
            start_line: 0,
            end_line: 3,
            kind: FoldKind::Comment,
        };
        assert_eq!(
            lift_folding_range(comment).kind,
            Some(FoldingRangeKind::Comment),
        );
    }

    /// `lift_symbol_kind` maps every `CoreSymbolKind` variant to the
    /// matching LSP `SymbolKind` constant. The mapping is exhaustive
    /// so additions to `CoreSymbolKind` force an explicit decision
    /// about the corresponding LSP `SymbolKind`.
    #[test]
    fn lift_symbol_kind_covers_all_variants() {
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Function),
            SymbolKind::FUNCTION
        );
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Method), SymbolKind::METHOD);
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Class), SymbolKind::CLASS);
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Property),
            SymbolKind::PROPERTY
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Constructor),
            SymbolKind::CONSTRUCTOR,
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Namespace),
            SymbolKind::NAMESPACE
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Variable),
            SymbolKind::VARIABLE
        );
    }

    /// `lift_document_symbol` preserves the name / detail / kind /
    /// range / selection-range fields and recursively lifts
    /// children. An empty child list maps to `None` rather than
    /// `Some(Vec::new())` so the serialised JSON omits the field
    /// entirely (per `#[serde(skip_serializing_if)]` on the LSP
    /// type).
    #[test]
    fn lift_document_symbol_preserves_fields_and_recurses() {
        let inner = core_symbols::DocumentSymbol {
            name: "child".to_owned(),
            detail: None,
            kind: CoreSymbolKind::Variable,
            range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            selection_range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            children: Vec::new(),
        };
        let outer = core_symbols::DocumentSymbol {
            name: "demo".to_owned(),
            detail: Some("()".to_owned()),
            kind: CoreSymbolKind::Function,
            range: core_symbols::LineRange {
                start_line: 0,
                start_character: 0,
                end_line: 4,
                end_character: 1,
            },
            selection_range: core_symbols::LineRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 9,
            },
            children: vec![inner],
        };

        let lifted = lift_document_symbol(outer);
        assert_eq!(lifted.name, "demo");
        assert_eq!(lifted.detail.as_deref(), Some("()"));
        assert_eq!(lifted.kind, SymbolKind::FUNCTION);
        assert_eq!(
            lifted.range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            lifted.range.end,
            Position {
                line: 4,
                character: 1
            }
        );
        assert_eq!(
            lifted.selection_range.start,
            Position {
                line: 0,
                character: 5
            },
        );
        let kids = lifted.children.expect("non-empty children lift to Some");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].name, "child");
        assert_eq!(kids[0].kind, SymbolKind::VARIABLE);
        assert!(
            kids[0].children.is_none(),
            "leaf symbol's empty children must lift to None",
        );
    }

    /// Build a fresh `Backend` for unit tests that only
    /// exercise the dialect-resolution helpers.  `Client`
    /// isn't publicly constructible, so we build via
    /// `LspService::new` and copy the wrapped `Client` into a
    /// fresh `Backend` with reset state.
    fn test_backend() -> Backend {
        let (service, _socket) = tower_lsp::LspService::new(Backend::new);
        Backend {
            client: service.inner().client.clone(),
            documents: Mutex::new(HashMap::new()),
            document_analysis_gate: Mutex::new(()),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            non_ascii_mode: Mutex::new(NonAsciiMode::Default),
            disabled_diagnostics: Mutex::new(HashSet::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
            workspace_index: Mutex::new(core_workspace_index::WorkspaceIndex::new()),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
        }
    }

    #[tokio::test]
    async fn stale_diagnostics_worker_cannot_overwrite_current_analysis() {
        let backend = test_backend();
        let uri = Url::parse("file:///stale.tcl").unwrap();
        let current_src = "proc current {} {}\n";
        let stale_src = "proc stale {} {}\n";

        let mut analyser = Analyser::new();
        let current_analysis = analyser.analyse(current_src, "tcl8.6").clone();
        {
            let _gate = backend.document_analysis_gate.lock().await;
            let mut doc =
                DocumentState::with_version(current_src.to_owned(), "tcl8.6".to_owned(), 2);
            doc.revision = 2;
            backend.documents.lock().await.insert(uri.clone(), doc);
            backend
                .analyses
                .lock()
                .await
                .insert(uri.clone(), current_analysis.clone());
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &current_analysis);
        }

        backend
            .publish_analyser_diagnostics(
                uri.clone(),
                stale_src.to_owned(),
                "tcl8.6".to_owned(),
                1,
                Some(1),
            )
            .await;

        let analysis = backend
            .analyses
            .lock()
            .await
            .get(&uri)
            .cloned()
            .expect("current analysis remains cached");
        assert!(analysis.all_procs.contains_key("::current"));
        assert!(!analysis.all_procs.contains_key("::stale"));

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("current", "other").is_empty(),
            "current document should remain indexed",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "stale worker must not re-index obsolete text",
        );
    }

    #[tokio::test]
    async fn scan_workspace_folders_indexes_unopened_files() {
        let root = unique_scratch_dir("scan");
        std::fs::write(
            root.join("lib.tcl"),
            "proc greet {name} { puts \"hi $name\" }\n",
        )
        .unwrap();
        std::fs::write(root.join("main.tcl"), "greet world\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        let defs = index.proc_definitions("greet", "greet");
        assert!(
            !defs.is_empty(),
            "expected the unopened lib.tcl proc to be indexed",
        );
        // The call site in main.tcl should be indexed too (no
        // current-URI exclusion here).
        let invs = index.invocations_of("greet", "greet", "");
        assert!(
            !invs.is_empty(),
            "expected the unopened main.tcl call site to be indexed",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn scan_workspace_folders_skips_open_documents() {
        let root = unique_scratch_dir("scan-open");
        let on_disk = root.join("buf.tcl");
        // The on-disk copy defines `stale`; the live buffer (below)
        // defines `fresh`. After the scan the index must reflect the
        // live buffer, not the on-disk copy.
        std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        let uri = Url::from_file_path(&on_disk).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Seed the index from the live buffer the way did_open would.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "fresh").is_empty(),
            "live buffer's proc must survive the scan",
        );
        assert!(
            index.proc_definitions("stale", "stale").is_empty(),
            "on-disk copy must not clobber the open document",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn workspace_scan_merge_does_not_clobber_open_document() {
        let backend = test_backend();
        let uri = Url::parse("file:///workspace/live.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );

        let mut analyser = Analyser::new();
        let fresh_analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .lock()
            .await
            .add_document(uri.as_str(), &fresh_analysis);

        let mut analyser = Analyser::new();
        let stale_analysis = analyser.analyse("proc stale {} {}\n", "tcl8.6").clone();
        let scan_results = vec![(uri.to_string(), stale_analysis)];

        backend.merge_workspace_scan_results(&scan_results).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "other").is_empty(),
            "open-buffer analysis must survive stale workspace scan results",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "workspace scan must not overwrite a live buffer",
        );
    }

    #[tokio::test]
    async fn reindex_from_disk_refreshes_index_rather_than_dropping() {
        let root = unique_scratch_dir("reindex");
        let on_disk = root.join("lib.tcl");
        std::fs::write(&on_disk, "proc helper {} {}\n").unwrap();

        let backend = test_backend();
        let uri = Url::from_file_path(&on_disk).unwrap();
        // Seed the index with a now-closed buffer version (proc `fresh`),
        // standing in for what did_open would have indexed.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        // What did_close does: refresh from disk instead of removing.
        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("helper", "other").is_empty(),
            "on-disk proc must remain indexed after the buffer closes",
        );
        assert!(
            index.proc_definitions("fresh", "other").is_empty(),
            "stale buffer-only proc must be gone after reindex from disk",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn reindex_from_disk_does_not_clobber_open_document() {
        let root = unique_scratch_dir("reindex-open");
        let on_disk = root.join("buf.tcl");
        std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

        let backend = test_backend();
        let uri = Url::from_file_path(&on_disk).unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "other").is_empty(),
            "open-buffer definitions must remain indexed",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "disk reindex must not overwrite a live buffer",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn reindex_from_disk_drops_entry_when_file_is_gone() {
        let backend = test_backend();
        let uri = Url::parse("file:///does/not/exist/gone.tcl").unwrap();
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc ghost {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            index.proc_definitions("ghost", "other").is_empty(),
            "an entry whose file no longer exists must be dropped",
        );
    }

    #[tokio::test]
    async fn resolve_folder_dialect_picks_deepest_prefix() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/").unwrap(),
                "tcl9.0".to_owned(),
            ),
            (
                Url::parse("file:///workspace/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        let inside = Url::parse("file:///workspace/irules/rule.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&inside).await,
            Some("f5-irules".to_owned()),
        );
        let outside_irules = Url::parse("file:///workspace/main.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&outside_irules).await,
            Some("tcl9.0".to_owned()),
        );
        let unrelated = Url::parse("file:///elsewhere/x.tcl").unwrap();
        assert_eq!(backend.resolve_folder_dialect(&unrelated).await, None);
    }

    #[tokio::test]
    async fn resolve_folder_dialect_respects_directory_boundary() {
        // Regression for PR #454 Codex review P2: a prefix-only
        // match would incorrectly select `file:///workspace/app`'s
        // dialect for a document inside `file:///workspace/app2/`.
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/app").unwrap(),
                "f5-irules".to_owned(),
            ),
            (
                Url::parse("file:///workspace/app2/").unwrap(),
                "tcl9.0".to_owned(),
            ),
        ];
        let sibling = Url::parse("file:///workspace/app2/main.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&sibling).await,
            Some("tcl9.0".to_owned()),
            "sibling folder must not inherit prefix-matched dialect",
        );
        let inside_app = Url::parse("file:///workspace/app/inner.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&inside_app).await,
            Some("f5-irules".to_owned()),
        );
    }

    #[test]
    fn uri_under_folder_handles_trailing_slash_and_boundary() {
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app/",
        ));
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app",
        ));
        // The folder itself counts as a match.
        assert!(uri_under_folder("file:///ws/app", "file:///ws/app/",));
        // A sibling folder does not.
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app",
        ));
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app/",
        ));
    }

    #[test]
    fn folder_dialect_for_prefers_deepest_match() {
        let folders = vec![
            (Url::parse("file:///ws/").unwrap(), "tcl8.6".to_owned()),
            (
                Url::parse("file:///ws/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        // A file under the nested folder picks up the nested
        // (deepest-prefix) dialect, not the parent's.
        let nested = Url::parse("file:///ws/irules/app.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&nested, &folders).as_deref(),
            Some("f5-irules"),
        );
        // A file only under the root folder gets the root dialect.
        let top = Url::parse("file:///ws/util.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&top, &folders).as_deref(),
            Some("tcl8.6"),
        );
        // A file outside every folder has no override.
        let outside = Url::parse("file:///other/x.tcl").unwrap();
        assert_eq!(folder_dialect_for(&outside, &folders), None);
    }

    #[test]
    fn is_tcl_source_matches_tcl_and_tm_only() {
        assert!(is_tcl_source(Path::new("/a/b.tcl")));
        assert!(is_tcl_source(Path::new("/a/b.tm")));
        assert!(!is_tcl_source(Path::new("/a/b.txt")));
        assert!(!is_tcl_source(Path::new("/a/b")));
    }

    #[test]
    fn is_skipped_scan_dir_skips_vendor_and_hidden() {
        assert!(is_skipped_scan_dir(Path::new("/a/.git")));
        assert!(is_skipped_scan_dir(Path::new("/a/node_modules")));
        assert!(is_skipped_scan_dir(Path::new("/a/target")));
        assert!(is_skipped_scan_dir(Path::new("/a/tmp")));
        assert!(!is_skipped_scan_dir(Path::new("/a/src")));
        assert!(!is_skipped_scan_dir(Path::new("/a/irules")));
    }

    /// Build a unique scratch directory under the system temp dir
    /// for a disk-walk test, returning its path. The caller removes
    /// it when done.
    fn unique_scratch_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tcl-lsp-scan-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collect_tcl_files_walks_recursively_and_skips_vendor() {
        let root = unique_scratch_dir("walk");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("top.tcl"), "set x 1\n").unwrap();
        std::fs::write(root.join("mod.tm"), "set y 2\n").unwrap();
        std::fs::write(root.join("readme.txt"), "not tcl\n").unwrap();
        std::fs::write(root.join("sub/nested.tcl"), "set z 3\n").unwrap();
        std::fs::write(root.join("node_modules/dep.tcl"), "set q 4\n").unwrap();
        std::fs::write(root.join(".git/hook.tcl"), "set w 5\n").unwrap();

        let mut out = Vec::new();
        collect_tcl_files(&root, 100, &mut out);
        let mut names: Vec<String> = out
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(names, vec!["mod.tm", "nested.tcl", "top.tcl"]);
    }

    #[test]
    fn collect_tcl_files_respects_cap() {
        let root = unique_scratch_dir("cap");
        for i in 0..10 {
            std::fs::write(root.join(format!("f{i}.tcl")), "set x 1\n").unwrap();
        }
        let mut out = Vec::new();
        collect_tcl_files(&root, 3, &mut out);
        let got = out.len();
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(got, 3);
    }

    #[test]
    fn hover_cache_round_trips_an_entry() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let key = (uri.clone(), 0, 5);
        // Miss before insertion.
        assert!(matches!(cache.get(&key), HoverLookup::Miss));
        // Insert and read back.
        let h = tcl_lsp_core::hover::Hover {
            value: "demo".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        cache.put(key.clone(), Some(h.clone()));
        match cache.get(&key) {
            HoverLookup::Hit(got) => assert_eq!(got.value, "demo"),
            other => panic!("expected Hit, got {other:?}"),
        }
    }

    #[test]
    fn hover_cache_records_empty_hovers() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let key = (uri, 0, 5);
        cache.put(key.clone(), None);
        // An empty hover is distinct from a miss.
        assert!(matches!(cache.get(&key), HoverLookup::HitEmpty));
    }

    #[test]
    fn hover_cache_invalidates_by_uri() {
        let mut cache = HoverCache::default();
        let uri_a = Url::parse("file:///a.tcl").unwrap();
        let uri_b = Url::parse("file:///b.tcl").unwrap();
        let h = tcl_lsp_core::hover::Hover {
            value: "x".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        cache.put((uri_a.clone(), 0, 0), Some(h.clone()));
        cache.put((uri_b.clone(), 0, 0), Some(h));
        cache.invalidate_uri(&uri_a);
        assert!(matches!(cache.get(&(uri_a, 0, 0)), HoverLookup::Miss));
        assert!(matches!(cache.get(&(uri_b, 0, 0)), HoverLookup::Hit(_)));
    }

    #[test]
    fn hover_cache_caps_at_256_entries() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let h = tcl_lsp_core::hover::Hover {
            value: "x".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        for i in 0..300 {
            cache.put((uri.clone(), 0, i), Some(h.clone()));
        }
        assert_eq!(cache.entries.len(), HoverCache::CAP);
        // Earliest insertions were evicted.
        assert!(matches!(cache.get(&(uri.clone(), 0, 0)), HoverLookup::Miss,));
        // Most-recent insertions survive.
        assert!(matches!(cache.get(&(uri, 0, 299)), HoverLookup::Hit(_),));
    }

    #[tokio::test]
    async fn dialect_for_open_falls_back_to_folder_dialect() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![(
            Url::parse("file:///workspace/").unwrap(),
            "f5-irules".to_owned(),
        )];
        let doc = Url::parse("file:///workspace/main.tcl").unwrap();
        // A `tcl` language id maps to `tcl8.6` directly, so the
        // folder override is *not* consulted (language_id is the
        // most specific signal we have).
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl").await,
            "tcl8.6".to_owned(),
        );
        // An unknown language id (`plaintext`) lets the folder
        // override take effect.
        assert_eq!(
            backend.dialect_for_open(&doc, "plaintext").await,
            "f5-irules".to_owned(),
        );
    }

    #[tokio::test]
    async fn registry_for_dialect_reuses_cached_arc() {
        let backend = test_backend();
        let first = backend.registry_for_dialect("f5-irules").await;
        let second = backend.registry_for_dialect("f5-irules").await;

        assert!(
            Arc::ptr_eq(&first, &second),
            "repeated dialect lookups should return the cached registry",
        );
    }

    #[test]
    fn command_head_gate_distinguishes_head_from_argument() {
        let src = "proc greet {x} {}\ngreet arg\n";
        let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
        // `greet` at line 1 is a command head (the call site).
        assert!(position_is_command_head(
            src,
            Position {
                line: 1,
                character: 2
            },
            &analysis,
        ));
        // `arg` at line 1 is an argument word, not a command head, so
        // the cross-document fallback must not fire on it.
        assert!(!position_is_command_head(
            src,
            Position {
                line: 1,
                character: 7
            },
            &analysis,
        ));
    }

    #[tokio::test]
    async fn cross_document_definition_resolves_sibling_proc() {
        let backend = test_backend();
        let lib_uri = Url::parse("file:///lib.tcl").unwrap();
        let main_uri = Url::parse("file:///main.tcl").unwrap();
        let lib_src = "proc shared_helper {} {}\n";
        let main_src = "shared_helper\n";
        // Register both documents.
        {
            let mut docs = backend.documents.lock().await;
            docs.insert(
                lib_uri.clone(),
                DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
            );
            docs.insert(
                main_uri.clone(),
                DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
            );
        }
        // Index the library document's proc.
        {
            let mut a = Analyser::new();
            let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(lib_uri.as_str(), &lib_analysis);
        }
        // From main.tcl, jumping on `shared_helper` resolves to
        // lib.tcl's declaration.
        let locs = backend
            .compute_definition(&main_uri, Position::new(0, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, lib_uri);
        // `proc shared_helper` — name token starts at column 5.
        assert_eq!(locs[0].range.start.line, 0);
        assert_eq!(locs[0].range.start.character, 5);
    }

    #[tokio::test]
    async fn minify_document_command_returns_minified_source() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("# c\nputs   hi\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Plain minify (compact=false, aggressive=false).
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(result["source"], serde_json::json!("puts hi"));
        assert_eq!(result["minifiedLength"], serde_json::json!(7));
        assert!(result.get("symbolMap").is_none());
    }

    #[tokio::test]
    async fn minify_document_command_compact_includes_symbol_map() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(
                "proc greet {name} {\n    return $name\n}\n".to_owned(),
                "tcl8.6".to_owned(),
            ),
        );
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(true), // compact
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            result["source"],
            serde_json::json!("proc a {a} {return $a}")
        );
        assert!(
            result["symbolMap"]
                .as_str()
                .is_some_and(|s| s.contains("a <- greet")),
            "{result:?}"
        );
    }

    #[test]
    fn unminify_error_command_translates_names() {
        let args = vec![
            serde_json::Value::String("can't read \"a\": no such variable".to_owned()),
            serde_json::Value::String("# Procs\n  a <- greet".to_owned()),
        ];
        let result = Backend::unminify_error_command(&args).expect("some");
        assert_eq!(
            result["translatedError"],
            serde_json::json!("can't read \"greet\": no such variable")
        );
        assert_eq!(result["changed"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn cross_document_definition_skipped_when_local_match_exists() {
        let backend = test_backend();
        let uri = Url::parse("file:///main.tcl").unwrap();
        let src = "proc greet {} {}\ngreet\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        // Cursor on the `greet` call (line 1) resolves locally;
        // the result points at the same document, line 0.
        let locs = backend
            .compute_definition(&uri, Position::new(1, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, uri);
        assert_eq!(locs[0].range.start.line, 0);
    }

    /// Register a document in the store and the workspace index.
    async fn register(backend: &Backend, uri: &Url, src: &str) {
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        backend
            .workspace_index
            .lock()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    #[tokio::test]
    async fn cross_document_references_finds_sibling_call_sites() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        let lib_src = "proc helper {} {}\nhelper\n";
        register(&backend, &lib, lib_src).await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        // From lib.tcl's `helper` declaration, cross-doc refs
        // are the two calls in consumer.tcl.
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(lib_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_references(&lib, lib_src, &analysis, Position::new(0, 5), false)
            .await;
        assert_eq!(cross.len(), 2, "{cross:?}");
        assert!(cross.iter().all(|l| l.uri == consumer));
    }

    #[tokio::test]
    async fn references_handler_merges_local_and_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        // lib.tcl: declaration + one local call.
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        // consumer.tcl: one cross-doc call.
        register(&backend, &consumer, "helper\n").await;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let result = backend.references(params).await.expect("ok").expect("some");
        // Declaration (lib:0) + local call (lib:1) + cross-doc
        // call (consumer:0) = 3 locations across 2 files.
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(result.iter().any(|l| l.uri == consumer));
        assert!(result.iter().filter(|l| l.uri == lib).count() == 2);
    }

    #[tokio::test]
    async fn rename_edits_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            new_name: "do_it".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        // Both documents get edits.
        assert!(changes.contains_key(&lib), "lib edits missing: {changes:?}");
        assert!(
            changes.contains_key(&consumer),
            "consumer edits missing: {changes:?}",
        );
        // consumer.tcl: two call sites rewritten to `do_it`.
        let consumer_edits = &changes[&consumer];
        assert_eq!(consumer_edits.len(), 2, "{consumer_edits:?}");
        assert!(consumer_edits.iter().all(|e| e.new_text == "do_it"));
    }

    #[tokio::test]
    async fn rename_to_builtin_blocked_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "helper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            // `if` is a built-in command — renaming to it must be
            // refused (no edits in any document).
            new_name: "if".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let result = backend.rename(params).await.expect("ok");
        assert!(result.is_none(), "rename to a built-in should be refused");
    }

    #[tokio::test]
    async fn will_rename_rewrites_dependent_source_literal() {
        let backend = test_backend();
        let main = Url::parse("file:///proj/main.tcl").unwrap();
        // main.tcl sources lib/old.tcl via a relative literal.
        register(&backend, &main, "source lib/old.tcl\nputs hi\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: "file:///proj/lib/old.tcl".to_owned(),
                new_uri: "file:///proj/lib/new.tcl".to_owned(),
            }],
        };
        let edit = backend
            .will_rename_files(params)
            .await
            .expect("ok")
            .expect("some");
        let DocumentChanges::Edits(doc_edits) = edit.document_changes.expect("document_changes")
        else {
            panic!("expected Edits");
        };
        assert_eq!(doc_edits.len(), 1);
        assert_eq!(doc_edits[0].text_document.uri, main);
        assert_eq!(doc_edits[0].edits.len(), 1);
        let OneOf::Left(text_edit) = &doc_edits[0].edits[0] else {
            panic!("expected plain TextEdit");
        };
        assert_eq!(text_edit.new_text, "lib/new.tcl");
        // The edit range covers the original `lib/old.tcl` literal on
        // the first line.
        assert_eq!(text_edit.range.start.line, 0);
    }

    #[tokio::test]
    async fn will_rename_returns_none_without_dependents() {
        let backend = test_backend();
        let main = Url::parse("file:///proj/main.tcl").unwrap();
        register(&backend, &main, "source other.tcl\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: "file:///proj/lib/old.tcl".to_owned(),
                new_uri: "file:///proj/lib/new.tcl".to_owned(),
            }],
        };
        // Nothing sources lib/old.tcl → no edit.
        assert!(backend
            .will_rename_files(params)
            .await
            .expect("ok")
            .is_none());
    }

    #[tokio::test]
    async fn did_rename_reindexes_moved_document() {
        let backend = test_backend();
        let old = Url::parse("file:///gone.tcl").unwrap();
        register(&backend, &old, "proc helper {} {}\n").await;
        assert_eq!(backend.workspace_index.lock().await.procs().len(), 1);
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: old.to_string(),
                // New path doesn't exist on disk (test env) — reindex
                // drops the stale old entry and finds nothing to add.
                new_uri: "file:///moved.tcl".to_owned(),
            }],
        };
        backend.did_rename_files(params).await;
        // The old document's proc is gone from the index.
        let index = backend.workspace_index.lock().await;
        assert!(index.procs().iter().all(|p| p.uri != old.as_str()));
    }

    #[tokio::test]
    async fn incoming_calls_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "proc caller {} { helper }\n").await;
        // prepare the `helper` item from lib.tcl.
        let prep = {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc helper {} {}\n", "tcl8.6").clone();
            core_call_hierarchy::prepare("proc helper {} {}\n", 0, 5, &analysis)
        };
        let core_item = prep.into_iter().next().expect("prepared item");
        // Call-hierarchy items now carry the short display name.
        assert_eq!(core_item.name, "helper");
        let cross = backend
            .cross_document_incoming_calls(&lib, &core_item.name)
            .await;
        // The only caller is `caller` in consumer.tcl.
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].0, consumer);
        assert_eq!(cross[0].1.from.name, "caller");
    }

    #[tokio::test]
    async fn outgoing_calls_resolve_sibling_document_callee() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let main = Url::parse("file:///main.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        // `caller` calls a builtin (`puts`) and the sibling-file
        // proc `helper`; only `helper` resolves cross-document.
        let main_src = "proc caller {} { puts hi\n helper }\n";
        register(&backend, &main, main_src).await;

        let item = core_call_hierarchy::CallHierarchyItem {
            name: "::caller".to_owned(),
            detail: None,
            range: CoreLspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            selection_range: CoreLspRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 11,
            },
        };
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(main_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_outgoing_calls(&main, main_src, "tcl", &item, &analysis)
            .await;
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].to.name, "::helper");
        assert_eq!(cross[0].to.uri, lib);
        assert_eq!(cross[0].from_ranges.len(), 1, "{cross:?}");
    }
}
