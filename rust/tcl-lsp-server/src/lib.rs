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

use std::collections::HashMap;
use std::sync::Arc;

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::call_hierarchy as core_call_hierarchy;
use tcl_lsp_core::code_actions as core_code_actions;
use tcl_lsp_core::code_lens as core_code_lens;
use tcl_lsp_core::completion::{
    self as core_completion, CompletionItem as CoreCompletionItem,
    CompletionKind as CoreCompletionKind,
};
use tcl_lsp_core::definition::{self as core_definition, LspRange as CoreLspRange};
use tcl_lsp_core::document_links as core_document_links;
use tcl_lsp_core::document_symbols::{self as core_symbols, SymbolKind as CoreSymbolKind};
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::formatting as core_formatting;
use tcl_lsp_core::hover::{self as core_hover, Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::inlay_hints as core_inlay_hints;
use tcl_lsp_core::linked_editing_range as core_linked_editing_range;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::selection_range as core_selection_range;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help::{
    self as core_sig, ParameterInformation as CoreParameterInformation,
    SignatureHelp as CoreSignatureHelp, SignatureInformation as CoreSignatureInformation,
};
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
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    DeclarationCapability, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams,
    DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams, DocumentLink,
    DocumentLinkOptions, DocumentLinkParams, DocumentRangeFormattingParams, DocumentSymbol,
    DocumentSymbolParams, DocumentSymbolResponse, Documentation, FoldingRange, FoldingRangeKind,
    FoldingRangeParams, FoldingRangeProviderCapability, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
    ImplementationProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, LinkedEditingRangeParams,
    LinkedEditingRanges, Location, MarkupContent, MarkupKind, MessageType, OneOf,
    ParameterInformation, ParameterLabel, Position, Range, ReferenceParams, RenameParams,
    SelectionRange, SelectionRangeParams, SelectionRangeProviderCapability,
    DiagnosticOptions, DiagnosticServerCapabilities, DocumentDiagnosticParams,
    DocumentDiagnosticReport, DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
    PrepareRenameResponse, RelatedFullDocumentDiagnosticReport, RenameOptions,
    SemanticTokens as LspSemanticTokens, SemanticTokensDelta, SemanticTokensDeltaParams,
    SemanticTokensFullDeltaResult, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, SignatureInformation, SymbolInformation, SymbolKind,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
    TypeDefinitionProviderCapability,
    Url, WorkDoneProgressOptions, WorkspaceEdit, WorkspaceSymbolParams,
};
use tower_lsp::{Client, LanguageServer};

/// Document store value: source text + dialect string.
#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    dialect: String,
}

impl DocumentState {
    fn new(text: String, dialect: String) -> Self {
        Self { text, dialect }
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
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
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
        tokio::task::spawn_blocking(move || {
            let mut analyser = Analyser::new();
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
        if let Some(d) = Self::dialect_from_language_id(language_id) {
            return d.to_owned();
        }
        if let Some(d) = self.resolve_folder_dialect(uri).await {
            return d;
        }
        self.default_dialect.lock().await.clone()
    }

    /// Look up the per-folder dialect override for `uri`,
    /// preferring the deepest (longest-prefix) match so nested
    /// folders shadow their parents.
    async fn resolve_folder_dialect(&self, uri: &Url) -> Option<String> {
        let folders = self.folder_dialects.lock().await;
        let mut best: Option<(usize, &str)> = None;
        let target = uri.as_str();
        for (folder, dialect) in folders.iter() {
            if !uri_under_folder(target, folder.as_str()) {
                continue;
            }
            // Track the longest-prefix match so nested folder
            // mappings (`file:///workspace/irules/` inside
            // `file:///workspace/`) shadow their parents.
            let len = folder.as_str().len();
            let take = match best {
                Some((existing, _)) => len > existing,
                None => true,
            };
            if take {
                best = Some((len, dialect));
            }
        }
        best.map(|(_, d)| d.to_owned())
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
        self.documents.lock().await.get(url).cloned()
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    async fn compute_definition(
        &self,
        uri: &Url,
        pos: Position,
    ) -> jsonrpc::Result<Vec<CoreLspRange>> {
        let Some(doc) = self.read_document(uri).await else {
            return Ok(Vec::new());
        };
        let analysis = self
            .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
            .await;
        tokio::task::spawn_blocking(move || {
            core_definition::definition(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("definition worker panicked: {err}").into(),
            data: None,
        })
    }

    /// Return an `Arc<CommandRegistry>` with `dialect` loaded on
    /// top of the default Tcl + stdlib + tcllib specs.
    ///
    /// The result is cached per canonical dialect key so each
    /// session builds at most one registry per requested dialect.
    /// Unparseable dialect strings collapse to the empty-string
    /// key so they share a single cached "plain Tcl" registry
    /// rather than leaking a fresh allocation per typo.
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
        let mut cache = self.dialect_registries.lock().await;
        if let Some(r) = cache.get(key) {
            return Arc::clone(r);
        }
        let mut r = CommandRegistry::build_default();
        if let Some(d) = dialect {
            r.load_dialect(d);
        }
        let arc = Arc::new(r);
        cache.insert(key.to_owned(), Arc::clone(&arc));
        arc
    }

    /// Run the analyser on `text` and push the resulting
    /// diagnostics to the LSP client for `uri`.  Mirrors
    /// the Python server's `publish_diagnostics` flow.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.  `S-async-diagnostics`
    /// minimal port — the cached-analysis surface and
    /// debounced streaming contract lands in a follow-up.
    async fn publish_analyser_diagnostics(&self, uri: Url, text: String, dialect: String) {
        let result = tokio::task::spawn_blocking(move || {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(&text, &dialect).clone();
            let diagnostics = lift_analyser_diagnostics(&text, &analysis.diagnostics);
            (analysis, diagnostics)
        })
        .await;
        match result {
            Ok((analysis, diags)) => {
                // Cache the analysis so the per-method
                // handlers don't have to re-run it on every
                // request.
                self.analyses.lock().await.insert(uri.clone(), analysis);
                self.client.publish_diagnostics(uri, diags, None).await;
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
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    #[allow(clippy::too_many_lines)]
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        // `S-workspace-init`: capture the workspace folders
        // the editor sent so cross-document features can
        // resolve relative paths and (in the future) scan
        // folder contents for a workspace index.  Both the
        // newer `workspace_folders` field and the
        // single-root `root_uri` fallback are supported.
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Url> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
        // `SYNC-MAY19-dialect-contextvar`: parse the
        // `folderDialects` object from `initializationOptions`
        // (when present) so multi-folder workspaces can mix
        // dialects.  Shape: `{ "folderDialects": { "file:///path":
        // "f5-irules", "file:///other": "tcl9.0" } }`.  Unknown
        // dialect names are dropped silently rather than
        // failing the whole initialise.
        if let Some(opts) = &params.initialization_options {
            let folder_map = opts
                .as_object()
                .and_then(|m| m.get("folderDialects"))
                .and_then(serde_json::Value::as_object);
            if let Some(entries) = folder_map {
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
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
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
                            // Advertise delta support — the
                            // handler implements the minimal
                            // valid shape (returns full tokens
                            // when previousResultId doesn't
                            // match, otherwise empty edits).
                            full: Some(SemanticTokensFullOptions::Delta {
                                delta: Some(true),
                            }),
                        },
                    ),
                ),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                linked_editing_range_provider: Some(
                    tower_lsp::lsp_types::LinkedEditingRangeServerCapabilities::Simple(true),
                ),
                // `S-diagnostics-pipeline`: advertise pull-based
                // diagnostics so editors that support it can
                // request diagnostics on demand alongside the
                // existing push-based `publish_diagnostics`.
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("tcl-lsp".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "tcl-lsp-server".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let dialect = self
            .dialect_for_open(
                &params.text_document.uri,
                &params.text_document.language_id,
            )
            .await;
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let dialect_for_diags = dialect.clone();
        let mut docs = self.documents.lock().await;
        docs.insert(
            params.text_document.uri,
            DocumentState::new(params.text_document.text, dialect),
        );
        drop(docs);
        self.publish_analyser_diagnostics(uri, text, dialect_for_diags)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync — the last content-change carries the entire
        // document. INCREMENTAL sync is a follow-up chunk.
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let uri = params.text_document.uri.clone();
        let mut docs = self.documents.lock().await;
        let (text, dialect) = if let Some(doc) = docs.get_mut(&uri) {
            // Preserve the document's dialect across edits; only the
            // text content changes here.
            doc.text.clone_from(&change.text);
            (change.text, doc.dialect.clone())
        } else {
            // didChange before didOpen — fall back to the session
            // default dialect; the languageId is not available here.
            let dialect = self.default_dialect.lock().await.clone();
            docs.insert(
                uri.clone(),
                DocumentState::new(change.text.clone(), dialect.clone()),
            );
            (change.text, dialect)
        };
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
        self.publish_analyser_diagnostics(uri, text, dialect).await;
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
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        self.documents.lock().await.remove(uri);
        self.analyses.lock().await.remove(uri);
        self.hover_cache.lock().await.invalidate_uri(uri);
        self.semantic_tokens_cache.lock().await.remove(uri);
        // Clear any previously-published diagnostics so the
        // editor's problem panel doesn't keep showing them
        // for a closed file.
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
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
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let symbols = core_symbols::document_symbols(&doc.text, &doc.dialect);
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
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
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = tokio::task::spawn_blocking(move || {
            core_completion::completions(
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
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items.into_iter().map(lift_completion_item).collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let ranges = self.compute_definition(&uri, pos).await?;
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
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let ranges = self.compute_definition(&uri, pos).await?;
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
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let ranges = self.compute_definition(&uri, pos).await?;
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
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let ranges = self.compute_definition(&uri, pos).await?;
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
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let ranges = tokio::task::spawn_blocking(move || {
            core_references::references(&doc.text, pos.line, pos.character, &analysis, include_decl)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("references worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
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
            core_references::document_highlights(&doc.text, pos.line, pos.character, &analysis)
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
        let incoming = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::incoming_calls(&doc.text, &core_item, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("incoming_calls worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted = incoming
            .into_iter()
            .map(|c| CallHierarchyIncomingCall {
                from: CallHierarchyItem {
                    name: c.from.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.from.detail,
                    uri: uri.clone(),
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
        let outgoing = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::outgoing_calls(&doc.text, &core_item, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("outgoing_calls worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted = outgoing
            .into_iter()
            .map(|c| CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: c.to.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.to.detail,
                    uri: uri.clone(),
                    range: lift_lsp_range(c.to.range),
                    selection_range: lift_lsp_range(c.to.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
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
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || lift_analyser_diagnostics(&doc.text, &analysis.diagnostics))
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
        let core_data = core_semantic_tokens::full(&doc.text).data;
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
        let new_data = core_semantic_tokens::full(&doc.text).data;
        let new_result_id = next_semantic_tokens_id();

        // Compare against the cached snapshot.  When the
        // previous_result_id matches the cached one *and* the
        // packed data is unchanged, return an empty edit list.
        // Otherwise return a fresh full token set — the LSP
        // spec accepts this as a valid response.  Computing a
        // minimal edit diff is a further follow-up.
        let mut cache = self.semantic_tokens_cache.lock().await;
        let prev = cache.get(&uri);
        let return_empty_delta = prev
            .is_some_and(|entry| entry.result_id == previous_result_id && entry.data == new_data);
        cache.insert(
            uri,
            SemanticTokensEntry {
                result_id: new_result_id.clone(),
                data: new_data.clone(),
            },
        );
        drop(cache);
        if return_empty_delta {
            return Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
                SemanticTokensDelta {
                    result_id: Some(new_result_id),
                    edits: Vec::new(),
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
        let core_data = core_semantic_tokens::range(&doc.text, core_range).data;
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
            let analysis = self
                .analysis_for(&uri, text.clone(), dialect.clone())
                .await;
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
        let links = core_document_links::document_links(&doc.text, workspace_root.as_deref());
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
                tooltip: None,
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
        // Build analysis on a worker so the inlay-hints
        // provider can surface parameter-name hints at user-
        // proc call sites (`S-inlay-hints-rich`).  When the
        // analyser surfaces an empty all_procs map (no user
        // procs in the document), the provider still returns
        // an empty hint set.
        let hints = tokio::task::spawn_blocking(move || {
            core_inlay_hints::inlay_hints(&doc.text, range, Some(&analysis))
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
        // `analysis.command_invocations` per proc, so the
        // worker needs the full analysis result.
        let lenses = tokio::task::spawn_blocking(move || {
            core_code_lens::code_lenses(&doc.text, Some(&analysis))
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
                command: Some(tower_lsp::lsp_types::Command {
                    title: l.command_title,
                    command: l.command,
                    arguments: None,
                }),
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

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
        // requested range.  Run analysis on a worker.
        let actions = tokio::task::spawn_blocking(move || {
            core_code_actions::code_actions(&doc.text, range, Some(&analysis))
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
        let lifted = actions
            .into_iter()
            .map(|a| {
                // Build a WorkspaceEdit from the action's
                // edits so accepting the action actually
                // applies the fix.
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
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: a.title,
                    kind: Some(tower_lsp::lsp_types::CodeActionKind::QUICKFIX),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: None,
                    disabled: None,
                    data: None,
                })
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let edits = core_formatting::formatting(&doc.text);
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
        let edits = core_formatting::range_formatting(&doc.text, range);
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
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let result = tokio::task::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain =
                        core_selection_range::selection_range(&doc.text, pos.line, pos.character);
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
        let edits = tokio::task::spawn_blocking(move || {
            core_rename::rename(
                &doc.text,
                pos.line,
                pos.character,
                &new_name,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("rename worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        let lifted: Vec<TextEdit> = edits
            .into_iter()
            .map(|e| TextEdit {
                range: lift_lsp_range(e.range),
                new_text: e.new_text,
            })
            .collect();
        let mut changes = std::collections::HashMap::new();
        changes.insert(uri, lifted);
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
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
        self.hover_cache
            .lock()
            .await
            .put(cache_key, result.clone());
        Ok(result.map(lift_hover))
    }
}

fn lift_completion_kind(k: CoreCompletionKind) -> CompletionItemKind {
    match k {
        CoreCompletionKind::Variable => CompletionItemKind::VARIABLE,
        CoreCompletionKind::Function => CompletionItemKind::FUNCTION,
    }
}

fn lift_completion_item(item: CoreCompletionItem) -> CompletionItem {
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        sort_text: item.sort_text,
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

/// Lift the analyser's diagnostic records into the LSP wire
/// shape.  Shared by both the push-based `publish_diagnostics`
/// path (via `publish_analyser_diagnostics`) and the pull-
/// based `textDocument/diagnostic` handler.
fn lift_analyser_diagnostics(
    text: &str,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new(text);
    diagnostics
        .iter()
        .cloned()
        .map(|d| {
            let start = line_index.position_at(d.span.start());
            let end = line_index.position_at(d.span.end());
            tower_lsp::lsp_types::Diagnostic {
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
            }
        })
        .collect()
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
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
        }
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
        let inside =
            Url::parse("file:///workspace/irules/rule.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&inside).await,
            Some("f5-irules".to_owned()),
        );
        let outside_irules =
            Url::parse("file:///workspace/main.tcl").expect("parse target uri");
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
        assert!(uri_under_folder(
            "file:///ws/app",
            "file:///ws/app/",
        ));
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
        assert!(matches!(
            cache.get(&(uri.clone(), 0, 0)),
            HoverLookup::Miss,
        ));
        // Most-recent insertions survive.
        assert!(matches!(
            cache.get(&(uri, 0, 299)),
            HoverLookup::Hit(_),
        ));
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
}
