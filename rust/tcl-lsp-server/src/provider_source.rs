// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Each package implementation keeps its own configured source owner.

use super::{
    Analyser, AnalysisResult, Arc, Backend, DocumentPathAdmission, FolderConfig, HashMap,
    HashSet, Mutex, NonAsciiMode, PackageResolver, Path, PathBuf, PublishedSourcePath,
    PublishedSourcePaths, ResourceAnalyserInputs, RwLock, TrackedMutex, Uri, longest_folder_match,
    merged_folder_dialect_overrides, session_dialect_from, vfs,
};
#[cfg(test)]
use super::{
    ControlFlow, DiagCode, RecoveryWidenCtx, SalsaAnalysisCtx, W120Availability,
    compute_base_analysis, defined_original_commands_from_analysis,
    refine_original_w123_diagnostics, widen_recovery_extra_commands,
};
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_registry::model::OverlayMiss;

#[derive(Clone)]
struct ProviderDialectInputs {
    document_overrides: Arc<HashMap<String, String>>,
    folders: Vec<(Uri, String)>,
    default: String,
}

/// The session's dialect settings, read when a provider capture runs rather
/// than when its diagnostics run is scheduled.
#[derive(Clone)]
pub(super) struct ProviderDialectSources {
    document_overrides: Arc<Mutex<HashMap<String, String>>>,
    folder_dialects: Arc<Mutex<Vec<(Uri, String)>>>,
    folder_configs: Arc<Mutex<Vec<(Uri, FolderConfig)>>>,
    session_override: Arc<Mutex<Option<String>>>,
    default_dialect: Arc<Mutex<String>>,
}

impl ProviderDialectSources {
    pub(super) fn of(backend: &Backend) -> Self {
        Self {
            document_overrides: Arc::clone(&backend.document_dialect_overrides),
            folder_dialects: Arc::clone(&backend.folder_dialects),
            folder_configs: Arc::clone(&backend.folder_configs),
            session_override: Arc::clone(&backend.session_dialect_override),
            default_dialect: Arc::clone(&backend.default_dialect),
        }
    }

    async fn capture(&self) -> ProviderDialectInputs {
        ProviderDialectInputs {
            document_overrides: Arc::new(self.document_overrides.lock().await.clone()),
            folders: merged_folder_dialect_overrides(&self.folder_dialects, &self.folder_configs)
                .await,
            default: session_dialect_from(&self.session_override, &self.default_dialect).await,
        }
    }
}

impl ProviderDialectInputs {

    fn dialect(&self, uri: &Uri, source: &str, target: Option<tcl_dialect::TclVersion>) -> String {
        // The resolver's release pin is independent of the provider's own
        // document, source and folder dialect choices.
        Backend::dialect_for_closed_sync(
            uri,
            source,
            self.document_overrides
                .get(uri.as_str())
                .map(String::as_str),
            &self.folders,
            target.map_or(self.default.as_str(), |version| version.dialect_name()),
        )
    }
}

pub(super) struct ProviderCaptureInputs<'a> {
    pub db: &'a Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    pub files: &'a Arc<TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    pub global: &'a Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    pub folders: &'a Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    pub store: &'a Arc<dyn vfs::SourceStore>,
    pub source_path_admissions: &'a PublishedSourcePaths,
    pub dialects: &'a ProviderDialectSources,
    pub proposal: Option<&'a ProposalProviderSources>,
}

struct ProviderSettings {
    disabled: HashSet<String>,
    non_ascii: NonAsciiMode,
    extra: HashSet<String>,
    overlay: u64,
    resource: ResourceAnalyserInputs,
}

impl ProviderSettings {
    fn from_config(config: tcl_lsp_db::AnalyserConfig, db: &dyn salsa::Database) -> Self {
        Self {
            disabled: config.disabled_diagnostics(db).iter().cloned().collect(),
            non_ascii: config.non_ascii_mode(db),
            extra: config.extra_commands(db).iter().cloned().collect(),
            overlay: config.spec_pack_key(db),
            resource: ResourceAnalyserInputs::from_db_config(config, db),
        }
    }

    fn analyser(&self, path: &Path) -> Analyser {
        Backend::configured_analyser(
            self.disabled.clone(),
            self.non_ascii,
            self.extra.clone(),
            self.overlay,
            self.resource.clone(),
        )
        .with_file_path(Some(path.display().to_string()))
    }
}

enum ProviderSource {
    Indexed {
        file: tcl_lsp_db::SourceFile,
        text: Arc<str>,
        dialect: String,
        input: Result<Arc<ResolvedAnalysisInput>, OverlayMiss>,
    },
    Disk,
}

struct ProviderRow {
    uri: Uri,
    settings: Arc<ProviderSettings>,
    factories: Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
    subclasses: Option<Arc<tcl_compiler::analyser::SubclassProvidedMethods>>,
    source: ProviderSource,
    admission: Option<Arc<DocumentPathAdmission>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ProviderSourceUnavailable {
    MissingMapping,
    MissingSource,
    Overlay(OverlayMiss),
    SourceCurrency,
    ResolverChanged,
    /// The original selected resource cannot supply its old provider schema
    /// to a hypothetical-source report. No proposal provider overlay is issued.
    ProposalSelectedSource,
    /// `SourceStore` supplies no retained byte-version lease for this disk input.
    ProposalDiskSource,
}

pub(super) struct ProviderSourceInventory {
    pub text: Arc<str>,
    pub input: Arc<ResolvedAnalysisInput>,
    pub analysis: AnalysisResult,
}

impl ProviderSourceInventory {
    fn is_current(&self) -> bool {
        self.analysis.resolved_input.as_ref() == Some(self.input.as_ref())
            && self.analysis.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.text),
                self.input.lexer_config(),
            )
    }
}

type InventoryResult = Result<Arc<ProviderSourceInventory>, ProviderSourceUnavailable>;
type InventoryCache = HashMap<(PathBuf, Option<tcl_dialect::TclVersion>), InventoryResult>;

pub(super) struct ConfiguredProviderSources {
    resolver_revision: u64,
    rows: HashMap<PathBuf, ProviderRow>,
    store: Arc<dyn vfs::SourceStore>,
    dialects: ProviderDialectInputs,
    inventories: std::sync::Mutex<InventoryCache>,
    proposal_selected: Option<ProposalProviderSelection>,
    proposal_captures: Option<std::sync::Weak<ProviderCaptureLedger>>,
}

#[derive(Clone)]
struct ProposalProviderSelection {
    uri: Uri,
    file: tcl_lsp_db::SourceFile,
    admission: Arc<DocumentPathAdmission>,
}

type ProviderCaptureLedger = std::sync::Mutex<Vec<Arc<ConfiguredProviderSources>>>;

/// Request-local original source dependency ledger. A capture is retained
/// before any inventory can be consumed, including recovery and cache hits.
/// Selected-resource providers remain unavailable without a proposal overlay.
pub(super) struct ProposalProviderSources {
    selected: ProposalProviderSelection,
    captures: Arc<ProviderCaptureLedger>,
}

impl ProposalProviderSources {
    pub(super) fn new(
        uri: Uri,
        file: tcl_lsp_db::SourceFile,
        admission: Arc<DocumentPathAdmission>,
    ) -> Option<Self> {
        (admission.uri() == &uri).then_some(Self {
            selected: ProposalProviderSelection {
                uri,
                file,
                admission,
            },
            captures: Arc::new(std::sync::Mutex::new(Vec::new())),
        })
    }

    fn retain(&self, sources: &Arc<ConfiguredProviderSources>) {
        self.captures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Arc::clone(sources));
    }
}

/// All actual source captures consumed by one deep report and its base passes.
/// Original external receipts remain separate from hypothetical source input.
pub(super) struct ConfiguredProviderSourceReport {
    captures: Vec<Arc<ConfiguredProviderSources>>,
    capture_failure: Option<ProviderSourceUnavailable>,
}

impl ConfiguredProviderSourceReport {
    pub(super) fn from_capture(deep: Arc<ConfiguredProviderSources>) -> Self {
        let snapshot = deep
            .proposal_captures
            .as_ref()
            .map(|retained| {
                let ledger = retained
                    .upgrade()
                    .ok_or(ProviderSourceUnavailable::SourceCurrency)?;
                let captures = ledger
                    .lock()
                    .map_err(|_| ProviderSourceUnavailable::SourceCurrency)?
                    .clone();
                Ok::<_, ProviderSourceUnavailable>(captures)
            })
            .transpose();
        let (mut captures, capture_failure) = match snapshot {
            Ok(Some(captures)) => (captures, None),
            Ok(None) => (Vec::new(), None),
            Err(error) => (Vec::new(), Some(error)),
        };
        if !captures.iter().any(|capture| Arc::ptr_eq(capture, &deep)) {
            captures.push(deep);
        }
        Self {
            captures,
            capture_failure,
        }
    }

    /// Original provider failure, independently of currentness. Missing source
    /// or unsupported proposal inputs cannot be reported as completed empty.
    pub(super) fn source_failure(&self) -> Option<ProviderSourceUnavailable> {
        self.capture_failure.clone().or_else(|| {
            self.captures
                .iter()
                .find_map(|capture| match capture.inventories.lock() {
                    Ok(inventories) => inventories
                        .values()
                        .find_map(|result| result.as_ref().err().cloned()),
                    Err(_) => Some(ProviderSourceUnavailable::SourceCurrency),
                })
        })
    }

    pub(super) fn is_current(&self, resolver: &PackageResolver) -> bool {
        self.capture_failure.is_none()
            && self
                .captures
                .iter()
                .all(|capture| capture.is_current(resolver))
    }

    pub(super) fn proposal_is_current(
        &self,
        resolver: &PackageResolver,
        db: &dyn salsa::Database,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &HashMap<Uri, PublishedSourcePath>,
    ) -> bool {
        self.capture_failure.is_none()
            && self
                .captures
                .iter()
                .all(|capture| capture.proposal_is_current(resolver, db, files, published))
    }

    pub(super) fn proposal_admissions(
        &self,
    ) -> Option<Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>> {
        if self.capture_failure.is_some() {
            return None;
        }
        let mut admissions = Vec::new();
        for capture in &self.captures {
            admissions.extend(capture.proposal_admissions()?);
        }
        Some(admissions)
    }
}

impl ProviderCaptureInputs<'_> {
    pub(super) async fn capture(
        &self,
        resolver: &Arc<RwLock<PackageResolver>>,
        needed: bool,
    ) -> Arc<ConfiguredProviderSources> {
        let (resolver_revision, paths) = {
            let resolver = resolver.read().await;
            (
                resolver.revision(),
                needed.then(|| resolver.provider_source_files()),
            )
        };
        let dialects = self.dialects.capture().await;
        if !needed {
            return self.retain_capture(resolver_revision, HashMap::new(), dialects);
        }
        let paths = paths.unwrap_or_default();
        // Configuration handles are copied before taking the DB lock. Source
        // walks and disk reads retain none of these locks or a Salsa snapshot.
        let folders = self.folders.lock().await.clone();
        let global = *self.global.lock().await;
        let source_admissions: HashMap<_, _> = {
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            published
                .iter()
                .map(|(uri, published)| {
                    (
                        uri.clone(),
                        (published.file, Arc::clone(&published.admission)),
                    )
                })
                .collect()
        };
        let rows = {
            let db = self.db.lock().await;
            let files = self.files.lock().await;
            let factories = Backend::published_class_factories(&db, &files);
            let mut settings = HashMap::new();
            paths
                .into_iter()
                .filter_map(|path| {
                    let uri = Uri::from_file_path(&path)?;
                    let config = longest_folder_match(&folders, &uri)
                        .copied()
                        .unwrap_or(global);
                    let settings =
                        Arc::clone(settings.entry(config).or_insert_with(|| {
                            Arc::new(ProviderSettings::from_config(config, &*db))
                        }));
                    let (source, factories, subclasses) = if let Some(&file) = files.get(&uri) {
                        (
                            ProviderSource::Indexed {
                                file,
                                text: Arc::from(file.text(&*db).as_str()),
                                dialect: file.dialect(&*db).clone(),
                                input: tcl_lsp_db::document_analysis_input(&*db, file, config),
                            },
                            file.workspace_class_factories(&*db).clone(),
                            file.workspace_subclass_methods(&*db).clone(),
                        )
                    } else {
                        (ProviderSource::Disk, factories.clone(), None)
                    };
                    let admission = match &source {
                        ProviderSource::Indexed { file, .. } => source_admissions
                            .get(&uri)
                            .filter(|(published_file, admission)| {
                                published_file == file
                                    && admission
                                        .path()
                                        .is_some_and(|selected| selected == path.as_path())
                                    && file.path(&*db).as_deref() == admission.analyser_path()
                            })
                            .map(|(_, admission)| Arc::clone(admission)),
                        ProviderSource::Disk => None,
                    };
                    Some((
                        path,
                        ProviderRow {
                            uri,
                            settings,
                            factories,
                            subclasses,
                            source,
                            admission,
                        },
                    ))
                })
                .collect()
        };
        self.retain_capture(resolver_revision, rows, dialects)
    }

    fn retain_capture(
        &self,
        resolver_revision: u64,
        rows: HashMap<PathBuf, ProviderRow>,
        dialects: ProviderDialectInputs,
    ) -> Arc<ConfiguredProviderSources> {
        let sources = Arc::new(ConfiguredProviderSources {
            resolver_revision,
            rows,
            store: Arc::clone(self.store),
            dialects,
            inventories: std::sync::Mutex::new(HashMap::new()),
            proposal_selected: self.proposal.map(|proposal| proposal.selected.clone()),
            proposal_captures: self
                .proposal
                .map(|proposal| Arc::downgrade(&proposal.captures)),
        });
        if let Some(proposal) = self.proposal {
            proposal.retain(&sources);
        }
        sources
    }
}

impl ConfiguredProviderSources {
    /// Whether every source inventory consulted by the report still matches
    /// its captured source/input, and the package resolver is unchanged.
    pub(super) fn is_current(&self, resolver: &PackageResolver) -> bool {
        if resolver.revision() != self.resolver_revision {
            return false;
        }
        let inventories = match self.inventories.lock() {
            Ok(inventories) => inventories,
            // Ordinary original-source behavior remains separate. A proposal
            // cannot infer complete dependencies from a partially updated map.
            Err(_) if self.proposal_selected.is_some() => return false,
            Err(error) => error.into_inner(),
        };
        inventories.values().all(|inventory| {
            inventory
                .as_ref()
                .is_ok_and(|inventory| inventory.is_current())
        })
    }

    /// Strict currency predicate for proposed-source reports. Indexed
    /// providers are checked against the retained live Salsa handles and
    /// admissions. A disk provider has no byte-version lease in `SourceStore`,
    /// so a report which consumed one cannot claim currentness.
    pub(super) fn proposal_is_current(
        &self,
        resolver: &PackageResolver,
        db: &dyn salsa::Database,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &HashMap<Uri, PublishedSourcePath>,
    ) -> bool {
        if resolver.revision() != self.resolver_revision {
            return false;
        }
        let Ok(inventories) = self.inventories.lock() else {
            return false;
        };
        inventories.iter().all(|((path, _), result)| {
            let Ok(inventory) = result else {
                return false;
            };
            let Some(row) = self.rows.get(path) else {
                return false;
            };
            let ProviderSource::Indexed {
                file,
                text,
                dialect,
                ..
            } = &row.source
            else {
                return false;
            };
            let Some(admission) = row.admission.as_ref() else {
                return false;
            };
            let Some(current) = published.get(&row.uri) else {
                return false;
            };
            current.file == *file
                && Arc::ptr_eq(&current.admission, admission)
                && files.get(&row.uri) == Some(file)
                && file.text(db).as_str() == text.as_ref()
                && file.dialect(db) == dialect
                && inventory.is_current()
        })
    }

    pub(super) fn proposal_admissions(
        &self,
    ) -> Option<Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>> {
        let inventories = self.inventories.lock().ok()?;
        inventories
            .iter()
            .map(|((path, _), result)| {
                result.as_ref().ok()?;
                let row = self.rows.get(path)?;
                let ProviderSource::Indexed { file, .. } = &row.source else {
                    return None;
                };
                Some((row.uri.clone(), *file, Arc::clone(row.admission.as_ref()?)))
            })
            .collect()
    }

    pub(super) fn inventory(
        &self,
        path: &Path,
        target: Option<tcl_dialect::TclVersion>,
        resolver: &PackageResolver,
    ) -> InventoryResult {
        if resolver.revision() != self.resolver_revision {
            return Err(ProviderSourceUnavailable::ResolverChanged);
        }
        if self.proposal_selected.as_ref().is_some_and(|selected| {
            self.rows.get(path).is_some_and(|row| {
                row.uri == selected.uri
                    || matches!(&row.source, ProviderSource::Indexed { file, .. } if *file == selected.file)
            }) || selected.admission.path().is_some_and(|selected_path| selected_path.as_os_str() == path.as_os_str())
        }) {
            let refused = Err(ProviderSourceUnavailable::ProposalSelectedSource);
            self.inventories.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((path.to_owned(), target), refused.clone());
            return refused;
        }
        if self.proposal_selected.is_some()
            && self
                .rows
                .get(path)
                .is_some_and(|row| matches!(row.source, ProviderSource::Disk))
        {
            let refused = Err(ProviderSourceUnavailable::ProposalDiskSource);
            self.inventories
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .insert((path.to_owned(), target), refused.clone());
            return refused;
        }
        let key = (path.to_owned(), target);
        if let Some(inventory) = self
            .inventories
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return match inventory {
                Ok(inventory) if !inventory.is_current() => {
                    Err(ProviderSourceUnavailable::SourceCurrency)
                }
                result => result.clone(),
            };
        }
        let result = self.analyse(path, target);
        self.inventories
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key, result.clone());
        result
    }

    fn analyse(&self, path: &Path, target: Option<tcl_dialect::TclVersion>) -> InventoryResult {
        let row = self
            .rows
            .get(path)
            .ok_or(ProviderSourceUnavailable::MissingMapping)?;
        let mut analyser = row
            .settings
            .analyser(path)
            .with_workspace_class_factories(row.factories.clone())
            .with_workspace_subclass_methods(row.subclasses.clone());
        let (text, dialect, input) = match &row.source {
            ProviderSource::Indexed {
                text,
                dialect,
                input,
                ..
            } => (
                Arc::clone(text),
                dialect.clone(),
                input.clone().map_err(ProviderSourceUnavailable::Overlay)?,
            ),
            ProviderSource::Disk => {
                let (text, _) = self
                    .store
                    .read_source(path)
                    .map_err(|_| ProviderSourceUnavailable::MissingSource)?;
                let text: Arc<str> = Arc::from(tcl_lexer::normalise_lone_cr(&text).as_ref());
                let dialect = self.dialects.dialect(&row.uri, &text, target);
                let input = Arc::new(
                    analyser
                        .prepare_analysis_input(&dialect)
                        .map_err(ProviderSourceUnavailable::Overlay)?,
                );
                (text, dialect, input)
            }
        };
        let analysis = analyser
            .with_resolved_input(input.as_ref().clone())
            .structure_only()
            .analyse(&text, &dialect);
        if analysis.resolved_input.as_ref() != Some(input.as_ref())
            || !analysis.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&text),
                input.lexer_config(),
            )
        {
            return Err(ProviderSourceUnavailable::SourceCurrency);
        }
        Ok(Arc::new(ProviderSourceInventory {
            text,
            input,
            analysis,
        }))
    }
}

pub(super) enum ProviderSourceAccess<'a> {
    Supplied(&'a ConfiguredProviderSources),
    #[cfg(test)]
    Standalone(&'a dyn vfs::SourceStore),
}

impl ProviderSourceAccess<'_> {
    pub(super) fn inventory(
        &self,
        path: &Path,
        target: Option<tcl_dialect::TclVersion>,
        resolver: &PackageResolver,
    ) -> InventoryResult {
        match self {
            Self::Supplied(sources) => sources.inventory(path, target, resolver),
            #[cfg(test)]
            Self::Standalone(store) => {
                let (text, _) = store
                    .read_source(path)
                    .map_err(|_| ProviderSourceUnavailable::MissingSource)?;
                let target = target.ok_or(ProviderSourceUnavailable::MissingMapping)?;
                let analysis = Analyser::new()
                    .structure_only()
                    .analyse(&text, target.dialect_name());
                let input = analysis
                    .resolved_input
                    .clone()
                    .ok_or(ProviderSourceUnavailable::SourceCurrency)?;
                Ok(Arc::new(ProviderSourceInventory {
                    text: Arc::from(text),
                    input: Arc::new(input),
                    analysis,
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests;
