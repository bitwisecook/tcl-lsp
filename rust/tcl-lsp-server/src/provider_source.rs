// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Each package implementation keeps its own configured source owner.

use super::*;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_registry::model::OverlayMiss;

#[derive(Clone)]
pub(super) struct ProviderDialectInputs {
    document_overrides: Arc<HashMap<String, String>>,
    folders: Vec<(Uri, String)>,
    default: String,
}

impl ProviderDialectInputs {
    pub(super) async fn capture(backend: &Backend) -> Self {
        Self {
            document_overrides: backend.document_dialect_override_snapshot().await,
            folders: backend.folder_dialect_overrides().await,
            default: backend.session_dialect().await,
        }
    }

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
    pub dialects: &'a ProviderDialectInputs,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ProviderSourceUnavailable {
    MissingMapping,
    MissingSource,
    Overlay(OverlayMiss),
    SourceCurrency,
    ResolverChanged,
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
}

impl ProviderCaptureInputs<'_> {
    pub(super) async fn capture(
        &self,
        resolver: &Arc<RwLock<PackageResolver>>,
        needed: bool,
    ) -> ConfiguredProviderSources {
        if !needed {
            return ConfiguredProviderSources {
                resolver_revision: 0,
                rows: HashMap::new(),
                store: Arc::clone(self.store),
                dialects: self.dialects.clone(),
                inventories: std::sync::Mutex::new(HashMap::new()),
            };
        }
        let (resolver_revision, paths) = {
            let resolver = resolver.read().await;
            (resolver.revision(), resolver.provider_source_files())
        };
        // Configuration handles are copied before taking the DB lock. Source
        // walks and disk reads retain none of these locks or a Salsa snapshot.
        let folders = self.folders.lock().await.clone();
        let global = *self.global.lock().await;
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
                    Some((
                        path,
                        ProviderRow {
                            uri,
                            settings,
                            factories,
                            subclasses,
                            source,
                        },
                    ))
                })
                .collect()
        };
        ConfiguredProviderSources {
            resolver_revision,
            rows,
            store: Arc::clone(self.store),
            dialects: self.dialects.clone(),
            inventories: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl ConfiguredProviderSources {
    pub(super) fn inventory(
        &self,
        path: &Path,
        target: Option<tcl_dialect::TclVersion>,
        resolver: &PackageResolver,
    ) -> InventoryResult {
        if resolver.revision() != self.resolver_revision {
            return Err(ProviderSourceUnavailable::ResolverChanged);
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
                    .as_ref()
                    .cloned()
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
