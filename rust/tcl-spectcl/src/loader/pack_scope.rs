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

//! Pack-scope statements: environments, dialects and surface rosters, the package, special-variable and file-extension rows, `include` resolution and the pack-level tables.

use super::dialect_block::PackDialect;
use super::environment_block::PackEnvironment;
use super::notices::{NewerVocabulary, Notice};
use super::statement_reader::{Stmt, block};
use super::value_parsers::{
    checked_lifecycle, lifecycle_flag, list_words, next_text, parse_dialects, parse_flag,
    parse_version,
};
use super::vocabulary_class::VocabularyClass;
use super::{
    AmbientPackage, CoProvides, FileExtension, IncludeContext, KNOWN_VOCABULARY_VERSIONS, Log,
    NEWEST_SUPPORTED_MAJOR, NEWEST_VOCABULARY_VERSION, Pack, PackProvides, PackSpecialVar,
    PackTables, available, dialect_block, environment_block, leak_one, leak_str, surface_roster,
};
use tcl_dialect::model::SpecSurface;
use tcl_registry::hover::ArgValue;
use tcl_source::spectcl::IncludeNameUnavailable;
use tcl_source::spectcl::include_file_name;
use tcl_source::spectcl::include_file_path;
use tcl_source::spectcl::pack_source_lexer_config;
use tcl_source::spectcl::unsupported_pack_major;

/// Apply one pack-scope statement — every arm of the pack vocabulary except
/// `command`, which the caller collects for the second pass. Returns `true`
/// exactly when the statement is a `command` declaration.
///
/// The one reader for a pack-level word: the evaluation loader's replay is
/// its only caller, whether the row was written literally or computed.
/// Read one `environment NAME { … }` block, keeping the first of a
/// redeclared pair (a declaration and its `-extend` are different rows,
/// so the identity is the pair).
fn read_environment(pack: &mut Pack, stmt: &Stmt, log: &mut Log) {
    log.v20(stmt.line, "environment");
    push_environment(pack, environment_block::parse(stmt, log), stmt.line, log);
}

/// [`read_environment`] from an already-read row list — the seam the
/// evaluation loader enters through, having run the block body as a script.
///
/// A pack is a Tcl program, so `environment NAME { … ambient Tk $v … }`
/// substitutes exactly as a `command` body does. Both entries meet the same
/// parser, the same validation, and the same redeclare rule; only where the
/// rows came from differs.
pub(super) fn read_environment_rows(
    pack: &mut Pack,
    stmt: &Stmt,
    rows: Option<&[Stmt]>,
    log: &mut Log,
) {
    log.v20(stmt.line, "environment");
    push_environment(
        pack,
        environment_block::parse_rows(stmt, rows, log),
        stmt.line,
        log,
    );
}

/// Keep an accepted environment block, or report the redeclaration.
fn push_environment(
    pack: &mut Pack,
    environment: Option<PackEnvironment>,
    line: u32,
    log: &mut Log,
) {
    let Some(environment) = environment else {
        return;
    };
    if pack
        .environments
        .iter()
        .any(|prior| prior.id == environment.id && prior.extends == environment.extends)
    {
        log.say(
            line,
            format!(
                "`environment {}` redeclared; the first is kept",
                environment.id
            ),
        );
    } else {
        pack.environments.push(environment);
    }
}

/// Read one `dialect NAME { … }` declaration from its already-read rows.
///
/// The dialect twin of [`read_environment_rows`], and reached the same two
/// ways: the literal reader splits the braced body, the evaluation loader
/// runs it as a script.
pub(super) fn read_dialect_rows(
    pack: &mut Pack,
    stmt: &Stmt,
    rows: Option<&[Stmt]>,
    log: &mut Log,
) {
    log.v20(stmt.line, "dialect");
    push_dialect(
        pack,
        dialect_block::parse_rows(stmt, rows, log),
        stmt.line,
        log,
    );
}

/// Keep an accepted dialect block, or report the redeclaration.
fn push_dialect(pack: &mut Pack, dialect: Option<PackDialect>, line: u32, log: &mut Log) {
    let Some(dialect) = dialect else {
        return;
    };
    if pack.dialects.iter().any(|prior| prior.name == dialect.name) {
        log.say(
            line,
            format!("`dialect {}` redeclared; the first is kept", dialect.name),
        );
    } else {
        pack.dialects.push(dialect);
    }
}

/// Read an `include` row's **surface** form (`include from … into …`,
/// design Q6), reporting whether the row was one.
///
/// The surface form composes no files, reads nothing off disk, and so has
/// none of the determinism problem the file form has — which is why it is
/// the one `include` shape that may reach this reader *computed*. A
/// literal row arrives here too: the capture layer routes it straight
/// through, so both spellings meet the one parser.
fn read_surface_roster(pack: &mut Pack, stmt: &Stmt, log: &mut Log) -> bool {
    let words: Vec<&str> = stmt
        .words
        .iter()
        .skip(1)
        .map(|word| word.text.as_str())
        .collect();
    if !surface_roster::is_surface_row(&words) {
        return false;
    }
    if let Some(row) = surface_roster::from_statement(stmt, log) {
        pack.surface_rosters.push(row);
    }
    true
}

pub(super) fn apply_pack_stmt(
    pack: &mut Pack,
    tables: &mut PackTables,
    stmt: &Stmt,
    log: &mut Log,
) -> bool {
    match stmt.word_text(0) {
        "values" => tables.add_values(stmt, log),
        "descriptor" => tables.add_descriptor(stmt, log),
        "hook" => tables.add_hook(stmt, log),
        "default" => tables.add_default(stmt, log),
        "display_name" => {
            let name = stmt.word_text(1);
            if name.is_empty() {
                log.say(stmt.line, "`display_name` needs a name");
            } else {
                if pack.display_name.is_some() {
                    log.say(stmt.line, "`display_name` redeclared; last wins");
                }
                pack.display_name = Some(name.to_owned());
            }
        }
        "file_extension" => {
            if let Some(row) = file_extension_row(stmt, log) {
                if pack
                    .file_extensions
                    .iter()
                    .any(|prior| prior.extension == row.extension)
                {
                    log.say(
                        stmt.line,
                        format!("`file_extension {}` redeclared; first wins", row.extension),
                    );
                } else {
                    pack.file_extensions.push(row);
                }
            }
        }
        "ambient_package" => {
            log.v12(stmt.line, "ambient_package");
            if let Some(row) = ambient_package_row(stmt, log) {
                pack.ambient_packages.push(row);
            }
        }
        "special_var" => {
            if let Some(row) = special_var_row(stmt, tables.defaults.surface, log) {
                if pack
                    .special_vars
                    .iter()
                    .any(|prior| prior.spec.name == row.spec.name)
                {
                    log.say(
                        stmt.line,
                        format!("`special_var {}` redeclared; first wins", row.spec.name),
                    );
                } else {
                    pack.special_vars.push(row);
                }
            }
        }
        "provides" => {
            log.v20(stmt.line, "provides");
            if let Some(row) = provides_row(stmt, log) {
                // §6.2: commands default their provider to the pack's
                // `provides`. The first `provides` name is the fallback; an
                // explicit `default required_package` / `default available`
                // always wins over it (see `command_from_parts`).
                if tables.defaults.provides_package.is_none() {
                    tables.defaults.provides_package = Some(row.name);
                }
                pack.provides.push(row);
            }
        }
        "co_provides" => {
            log.v20(stmt.line, "co_provides");
            if let Some(row) = co_provides_row(stmt, log) {
                pack.co_provides.push(row);
            }
        }
        "include" => {
            // A literal pack-scope `include` is consumed by the capture layer
            // (`stage_include`) before any row reaches this reader, so the
            // only way here is a *computed* include — a templated pack
            // building the row, which the determinism contract does not
            // follow.
            log.v20(stmt.line, "include");
            if read_surface_roster(pack, stmt, log) {
                return false;
            }
            log.say_classified(
                stmt.line,
                VocabularyClass::Semantic,
                "`include` must be a literal pack-scope row (the determinism contract \
                 forbids a computed include); the row is dropped and its declarations \
                 are not loaded",
            );
        }
        "environment" => read_environment(pack, stmt, log),
        "dialect" => {
            log.v20(stmt.line, "dialect");
            push_dialect(pack, dialect_block::parse(stmt, log), stmt.line, log);
        }
        "command" => return true,
        _ => log.unknown_property(stmt),
    }
    false
}

/// Vocabulary consistency: words newer than the pack's declaration.
/// Additions never gate, so THIS loader read them fine — but a loader
/// speaking only the declared vocabulary drops each one silently, and
/// raising the declaration is how a pack says it needs them. One notice
/// per site, so an editor can mark every offending row.
///
/// A pack with no `speclib` line declares nothing to be inconsistent with,
/// and one declaring a vocabulary at or above the word's own is correct;
/// both are skipped.
pub(super) fn finish_newer_words(pack: &Pack, log: &mut Log) {
    if pack.dsl_version.is_empty() {
        log.newer_words.clear();
        return;
    }
    let declared = pack.dsl_version.clone();
    let name = pack.name.clone();
    for (context, line, word, since) in std::mem::take(&mut log.newer_words) {
        if tcl_registry::version::compare(&declared, since).is_ge() {
            continue;
        }
        log.notices.push(Notice {
            subject: None,
            context,
            line,
            class: VocabularyClass::Presentation,
            message: format!(
                "`{word}` is SpecTcl {since} vocabulary, but this pack declares \
                 vocabulary {declared}; a {declared} loader drops the word — declare \
                 `speclib {name} {since}`"
            ),
            newer_vocabulary: Some(NewerVocabulary {
                word,
                since,
                declared: declared.clone(),
            }),
        });
    }
}

/// Resolve every `environment … { core DIALECT RELEASE }` row against the
/// pack's own `dialect` blocks, once the whole file has been read.
///
/// A `core` row naming neither a compiled family nor a dialect this pack
/// declares is §6.1's semantic class — the row says what language the
/// environment's documents are — so the environment block is **rejected**,
/// not degraded. Run after both passes because a `dialect` block may be
/// written after the `environment` that rides it, and a pack's statement
/// order is the author's business.
pub(super) fn finish_pack_cores(pack: &mut Pack, log: &mut Log) {
    if pack.environments.iter().all(|e| e.pack_core.is_none()) {
        return;
    }
    let dialects: Vec<(String, Vec<String>)> = pack
        .dialects
        .iter()
        .map(|dialect| {
            (
                dialect.name.clone(),
                dialect
                    .releases
                    .iter()
                    .map(|release| release.release.clone())
                    .collect(),
            )
        })
        .collect();
    let mut rejected: Vec<String> = Vec::new();
    for environment in &pack.environments {
        let Some(core) = &environment.pack_core else {
            continue;
        };
        let found = dialects.iter().find(|(name, _)| *name == core.dialect);
        let message = match found {
            None => Some(format!(
                "`environment {}` names core family `{}`, which is neither a compiled \
                 family (`tcl`, `f5-tcl`, `f5-irules`, `jim`) nor a `dialect` block this \
                 pack declares; the environment block is rejected",
                environment.id, core.dialect
            )),
            Some((_, releases)) if !releases.contains(&core.release) => Some(format!(
                "`environment {}` names core `{} {}`, which is not a `release` row of \
                 `dialect {}`; the environment block is rejected",
                environment.id, core.dialect, core.release, core.dialect
            )),
            Some(_) => None,
        };
        if let Some(message) = message {
            log.say(core.line, message);
            rejected.push(environment.id.clone());
        }
    }
    pack.environments
        .retain(|environment| !rejected.contains(&environment.id));
}

/// A scoping flag for `ambient_package`, recognised
/// only so it can be **refused**.
///
/// A pack that wants a library ambient under one of its dialects and not
/// another might reach for
/// `ambient_package NAME VERSION -dialects {…}`; `SpecTcl` 2.0 instead
/// answers the same question with `environment NAME { ambient PACKAGE
/// VERSION }`, which states the placement *inside* the environment it
/// belongs to instead of flag-scoping a global claim, and a version shared
/// by several environments is an ordinary Tcl variable substituted into
/// each row.
///
/// The flag is not carried as a second spelling of that, for two reasons
/// the model does not let us wish away:
///
/// - **Desugaring it is not available.** A `-dialects` list naming a
///   compiled environment (`tcl8.6`, `f5-irules`) desugars to
///   `environment NAME -extend { ambient … }`, and §6.4's trust lattice
///   forbids exactly that for the workspace and studio tiers most packs
///   come from (`registration::untrusted_compiled_extension`). The sugar
///   would work for bundled packs and fail the whole registration for
///   everyone else.
/// - **It would fork the floor model.** A scoped row kept in the pack's own
///   ambient table reports as `PackAmbient` (a pack the workspace owns);
///   the same claim written as a placement reports as the environment's own
///   pin. One question with two answers, differing only in how it was
///   spelled, is the parallel table the redesign set out to remove.
///
/// So the word is refused, and the row goes with it. Dropping only the flag
/// would leave the *unscoped* claim standing — the pack would floor the
/// package in every dialect, including the ones it just said the package is
/// absent from, which is §6.1's fail-closed rule read backwards.
const AMBIENT_SCOPE_FLAG: &str = "-dialects";

/// `ambient_package NAME VERSION` — one package the pack's dialect provides
/// without a `package require`.
///
/// Both words are required. A row naming no version is dropped rather than
/// defaulted: an ambient package with no version would floor at nothing, which
/// is what the row exists to stop being the case. A word that is not a package
/// version is no version either, and the row is dropped with it: the version is
/// a floor, and a floor the comparison cannot read orders against the profile's
/// pin as it happens to.
///
/// The row is **unscoped by construction**: it floors its package for every
/// document the pack is active in. Scoping the claim to some of a pack's
/// environments is what `environment NAME { ambient PACKAGE VERSION }`
/// (`SpecTcl` 2.0 §6.2) says, and [`AMBIENT_SCOPE_FLAG`] is refused here
/// rather than read — see that constant for why.
fn ambient_package_row(stmt: &Stmt, log: &mut Log) -> Option<AmbientPackage> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`ambient_package` needs a package name");
        return None;
    }
    let version = stmt.word_text(2);
    if version.is_empty() {
        log.say(
            stmt.line,
            format!("`ambient_package {name}` needs the version the runtime provides; dropped"),
        );
        return None;
    }
    if !tcl_dialect::validate_version(version) {
        log.say(
            stmt.line,
            format!("`ambient_package {name}` names `{version}`, which is not a package version; dropped"),
        );
        return None;
    }
    if stmt
        .words
        .iter()
        .skip(3)
        .any(|word| word.text == AMBIENT_SCOPE_FLAG)
    {
        log.say_classified(
            stmt.line,
            VocabularyClass::Semantic,
            format!(
                "`ambient_package {name}` uses `{AMBIENT_SCOPE_FLAG}`, which is not \
                 SpecTcl vocabulary; the whole row is dropped rather than applied \
                 to every dialect, because a reader that ignored the scoping would \
                 floor {name} where the pack says it is absent (design §6.1). Write \
                 `environment NAME {{ ambient {name} {version} }}` for each \
                 environment the package is ambient in"
            ),
        );
        return None;
    }
    for extra in stmt.words.iter().skip(3) {
        log.unknown_flag("ambient_package", stmt.line, extra);
    }
    Some(AmbientPackage {
        name: leak_str(name),
        version: leak_str(version),
        line: stmt.line,
    })
}

/// `special_var NAME -kind K -access A -origin O ?-dialects {…}? ?-startup B?`
/// — one interpreter-provided global the pack's dialect has, as the
/// special-variable registry states one: its value shape (`Scalar`, `Array`,
/// `Namespace`), whether user code writes it (`ReadOnly`, `ReadWrite`), where
/// it comes from (`Interpreter`, `AutoLoader`, `Platform`, `Environment`,
/// `Dialect`), the dialects that provide it — the pack's `default dialects`
/// when the row names none, else every Tcl release — and the lifecycle event
/// that makes it readable before user code (`None`, the default;
/// `Interpreter`, `TclInit`, `TclMain`, `AppInit`; or `ReadTrace`, a core read
/// trace materialising it on first read).
///
/// The three descriptors are required: a row missing one is dropped rather
/// than defaulted, since a guessed kind or access would state a fact the pack
/// never made. The row's remaining facts — known array keys, a runtime-observed
/// write, a write's interpreter effect, a read's taint, the hover summary —
/// take the shipped table's empty values.
fn special_var_row(
    stmt: &Stmt,
    default_surface: Option<&'static [SpecSurface]>,
    log: &mut Log,
) -> Option<PackSpecialVar> {
    use tcl_registry::{SpecialVarKind, SpecialVarSpec, StartupBinding, VarAccess};
    let name = stmt.word_text(1);
    if name.is_empty() || name.starts_with('-') {
        log.say(stmt.line, "`special_var` needs a variable name");
        return None;
    }
    let (mut kind, mut access, mut origin, mut surface) = (None, None, None, None);
    let mut startup = Some(StartupBinding::None);
    // A `-dialects` naming nothing this build knows narrows to nothing it
    // can honour: the row goes rather than widening to every dialect.
    let mut dialects_unread = false;
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        let flag = words[i].text.clone();
        if !["-kind", "-access", "-origin", "-dialects", "-startup"].contains(&flag.as_str()) {
            // An unknown flag consumes no value.
            log.unknown_flag("special_var", stmt.line, &rejected_flag_word);
            i += 1;
            continue;
        }
        let value = next_text(words, &mut i);
        let recognised = match flag.as_str() {
            "-kind" => {
                kind = match value.as_str() {
                    "Scalar" => Some(SpecialVarKind::Scalar),
                    "Array" => Some(SpecialVarKind::Array),
                    "Namespace" => Some(SpecialVarKind::Namespace),
                    _ => None,
                };
                kind.is_some()
            }
            "-access" => {
                access = match value.as_str() {
                    "ReadOnly" => Some(VarAccess::ReadOnly),
                    "ReadWrite" => Some(VarAccess::ReadWrite),
                    _ => None,
                };
                access.is_some()
            }
            "-origin" => {
                origin = special_var_origin(&value);
                origin.is_some()
            }
            "-dialects" => {
                surface = parse_dialects(&value, stmt.line, log);
                dialects_unread |= surface.is_none();
                surface.is_some()
            }
            _ => {
                startup = startup_binding(&value);
                startup.is_some()
            }
        };
        if !recognised {
            log.say(
                stmt.line,
                format!("`special_var {name} {flag} {value}` names no such value; dropped"),
            );
        }
        i += 1;
    }
    let (Some(kind), Some(access), Some(origin), Some(startup), false) =
        (kind, access, origin, startup, dialects_unread)
    else {
        log.say(
            stmt.line,
            format!(
                "`special_var {name}` needs `-kind`, `-access` and `-origin`, and every \
                 flag it writes must name a value this build reads; dropped rather than \
                 widened"
            ),
        );
        return None;
    };
    let surface = surface.or(default_surface).unwrap_or(SpecSurface::ALL_TCL);
    let (initially_bound, lazily_readable): (&'static [SpecSurface], &'static [SpecSurface]) =
        match startup {
            StartupBinding::None => (&[], &[]),
            StartupBinding::ReadTrace => (&[], surface),
            _ => (surface, &[]),
        };
    Some(PackSpecialVar {
        spec: leak_one(SpecialVarSpec {
            name: leak_str(name),
            kind,
            access,
            origin,
            surface,
            initially_bound,
            lazily_readable,
            startup_binding: startup,
            runtime_hook: None,
            keys: &[],
            externally_read: false,
            cmp_unsafe: false,
            write_effect: None,
            read_taint: None,
            summary: "",
        }),
        line: stmt.line,
    })
}

/// A `special_var -origin` value.
fn special_var_origin(value: &str) -> Option<tcl_registry::VarOrigin> {
    use tcl_registry::VarOrigin;
    Some(match value {
        "Interpreter" => VarOrigin::Interpreter,
        "AutoLoader" => VarOrigin::AutoLoader,
        "Platform" => VarOrigin::Platform,
        "Environment" => VarOrigin::Environment,
        "Dialect" => VarOrigin::Dialect,
        _ => return None,
    })
}

/// A `special_var -startup` value: the lifecycle event that makes the
/// variable readable before user code.
pub(super) fn startup_binding(value: &str) -> Option<tcl_registry::StartupBinding> {
    use tcl_registry::StartupBinding;
    Some(match value {
        "None" => StartupBinding::None,
        "Interpreter" => StartupBinding::Interpreter,
        "TclInit" => StartupBinding::TclInit,
        "TclMain" => StartupBinding::TclMain,
        "AppInit" => StartupBinding::AppInit,
        "ReadTrace" => StartupBinding::ReadTrace,
        _ => return None,
    })
}

/// `provides NAME ?VERSION…?` — one package train this pack describes.
fn provides_row(stmt: &Stmt, log: &mut Log) -> Option<PackProvides> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`provides` needs a package name");
        return None;
    }
    let versions: Vec<&'static str> = stmt
        .words
        .iter()
        .skip(2)
        .map(|word| leak_str(&word.text))
        .collect();
    Some(PackProvides {
        name: leak_str(name),
        versions,
        line: stmt.line,
    })
}

/// `co_provides NAME ?-requires-exact PACKAGE? ?-when PREDICATE?`.
fn co_provides_row(stmt: &Stmt, log: &mut Log) -> Option<CoProvides> {
    let name = stmt.word_text(1);
    if name.is_empty() {
        log.say(stmt.line, "`co_provides` needs the co-provided name");
        return None;
    }
    let mut row = CoProvides {
        name: leak_str(name),
        requires_exact: None,
        when: None,
        line: stmt.line,
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-requires-exact" => {
                let package = next_text(words, &mut i);
                if package.is_empty() {
                    log.say(stmt.line, "`-requires-exact` needs a package name");
                } else {
                    row.requires_exact = Some(leak_str(&package));
                }
            }
            "-when" => {
                let predicate = next_text(words, &mut i);
                if predicate.is_empty() {
                    log.say(stmt.line, "`-when` needs a predicate");
                } else {
                    row.when = Some(leak_str(&predicate));
                }
            }
            _ => log.unknown_flag("co_provides", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(row)
}

/// `file_extension EXT ?-name {…}? ?-dialect DIALECT?` — one extension the
/// pack's language is written under. The extension is normalised to
/// lower-case without a leading dot; `-dialect` must name a canonical
/// profile (the routing consumer needs an interned name), and a typo drops
/// only the routing, not the row.
pub(super) fn file_extension_row(stmt: &Stmt, log: &mut Log) -> Option<FileExtension> {
    let raw = stmt.word_text(1);
    if raw.is_empty() {
        log.say(stmt.line, "`file_extension` needs an extension");
        return None;
    }
    let extension = raw.trim_start_matches('.').to_ascii_lowercase();
    if extension.is_empty() || extension.contains('.') || extension.contains(char::is_whitespace) {
        log.say(
            stmt.line,
            format!("`file_extension {raw}` is not a single extension"),
        );
        return None;
    }
    let mut row = FileExtension {
        extension,
        display_name: None,
        dialect: None,
        line: stmt.line,
        // Filled in by the merge, the only layer that knows the path.
        file: std::path::PathBuf::new(),
    };
    let words = &stmt.words;
    let mut i = 2;
    while i < words.len() {
        let rejected_flag_word = words[i].clone();
        match words[i].text.as_str() {
            "-name" => {
                let name = next_text(words, &mut i);
                if name.is_empty() {
                    log.say(stmt.line, "`-name` needs a value");
                } else {
                    row.display_name = Some(name);
                }
            }
            "-dialect" => {
                let dialect = next_text(words, &mut i);
                match crate::environment::catalogue_profile_for_dialect(&dialect) {
                    Some(profile) => row.dialect = Some(profile.name),
                    None => log.say(
                        stmt.line,
                        format!(
                            "`-dialect {dialect}` names no dialect profile; \
                             the extension is kept without routing"
                        ),
                    ),
                }
            }
            _ => log.unknown_flag("file_extension", stmt.line, &rejected_flag_word),
        }
        i += 1;
    }
    Some(row)
}

/// One actual include resolver observation, before BOM removal/evaluation.
pub(crate) struct IncludeSourceObservation {
    pub(crate) path: std::path::PathBuf,
    pub(crate) origin: Option<tcl_lsp_core::vfs::SourcePathOrigin>,
    pub(crate) image: Result<String, String>,
}

impl IncludeContext {
    /// A context over an arbitrary resolver — the seam tests and the
    /// studio use.
    pub fn new(resolver: impl Fn(&str) -> Result<String, String> + 'static) -> Self {
        Self {
            resolver: Box::new(resolver),
            reads: std::cell::Cell::new(0),
        }
    }

    /// How many includes evaluations through this context have resolved.
    pub(crate) fn reads(&self) -> u32 {
        self.reads.get()
    }

    /// The file-system context for a pack at `path`: an include name
    /// resolves against the pack's **own directory** only. Name
    /// validation (no separators, no `..`) happens in the loader before
    /// the resolver is asked, so this cannot be steered outside the
    /// directory even by a hostile name.
    #[must_use]
    pub fn for_file(path: &std::path::Path) -> Self {
        let including_file = path.to_path_buf();
        Self::new(move |name| {
            let Some(path) = include_file_path(&including_file, name) else {
                return Err("the including pack has no parent directory".to_owned());
            };
            std::fs::read_to_string(path).map_err(|e| e.to_string())
        })
    }

    pub(crate) fn for_source_store(
        path: &std::path::Path,
        store: std::sync::Arc<dyn tcl_lsp_core::vfs::SourceStore>,
        observations: std::rc::Rc<std::cell::RefCell<Vec<IncludeSourceObservation>>>,
    ) -> Self {
        let including_file = path.to_path_buf();
        Self::new(move |name| {
            let path = include_file_path(&including_file, name)
                .ok_or_else(|| "the including pack has no parent directory".to_owned())?;
            let origin = store.source_path_origin(&path).ok().filter(|origin| {
                origin.path().as_os_str() == path.as_os_str()
                    && store.validate_source_path_origin(origin).is_ok()
            });
            let image = store
                .read_to_string(&path)
                .map_err(|error| format!("{error:?}"));
            observations.borrow_mut().push(IncludeSourceObservation {
                path,
                origin,
                image: image.clone(),
            });
            image
        })
    }

    pub(super) fn resolve(&self, name: &str) -> Result<String, String> {
        self.reads.set(self.reads.get().saturating_add(1));
        (self.resolver)(name)
    }
}

/// The most deeply nested chain of `include`s the loader follows. The
/// cap is a determinism bound, not a feature: a chain this deep is a
/// mistake, and an unbounded walk over a resolver is not a loader.
pub(crate) const INCLUDE_DEPTH_LIMIT: usize = 8;

/// Whether `source`'s first pack body carries a pack-scope `include` row —
/// the trigger [`crate::pack`] uses to load through a file-system
/// [`IncludeContext`] instead of the compiled cache, whose key cannot see
/// an included file's bytes. The substring test keeps the common
/// include-free pack on the cached path with no second parse.
#[must_use]
pub(crate) fn uses_include(source: &str) -> bool {
    if !source.contains("include") {
        return false;
    }
    let config = pack_source_lexer_config(tcl_lexer::SourceSegmentationPurpose::WholeInput);
    let Some(body) = tcl_source::spectcl::first_pack_source_body(source, config) else {
        return false;
    };
    body.statements()
        .iter()
        .any(tcl_source::spectcl::is_pack_file_include_row)
}

/// Why one `include NAME` word is unusable, or `Ok` with the name.
///
/// The one rule for what a valid include row is, whichever route captured
/// it: exactly two words, a non-empty literal name, and no path structure —
/// the resolver decides what a name means, but a name is never a path.
pub(crate) fn include_name(words: &[&str], line: u32) -> Result<String, Box<Notice>> {
    let reject = |message: String| {
        Box::new(Notice {
            subject: None,
            newer_vocabulary: None,
            context: "pack".to_owned(),
            line,
            message,
            class: VocabularyClass::Semantic,
        })
    };
    match include_file_name(words) {
        Ok(name) => Ok(name.to_owned()),
        Err(IncludeNameUnavailable::PathStructure) => {
            let name = words[0];
            Err(reject(format!(
                "`include {name}` is not a plain file name (no path separators, no `..`); \
                 the row is dropped and its declarations are not loaded"
            )))
        }
        Err(IncludeNameUnavailable::Arity) => Err(reject(
            "`include` takes exactly one file name; the row is dropped and its \
             declarations are not loaded"
                .to_owned(),
        )),
    }
}

/// Warn when `speclib`'s version word is not a vocabulary this build knows,
/// and report whether the pack must fail closed.
///
/// `true` means the declaration named a **major** past this loader
/// (`speclib probe 3.0`). §6.1: that is a load error, not a notice — every
/// word in the file may have been redefined, so loading the ones this build
/// recognises would publish confident answers derived from a vocabulary it
/// does not speak. An unknown *minor* is the additive case and keeps loading.
pub(super) fn check_vocabulary_version(declared: &str, line: u32, log: &mut Log) -> bool {
    if declared.is_empty() || KNOWN_VOCABULARY_VERSIONS.contains(&declared) {
        return false;
    }
    if unsupported_pack_major(declared) {
        log.say(
            line,
            format!(
                "pack declares SpecTcl vocabulary {declared}; this loader supports major \
                 {NEWEST_SUPPORTED_MAJOR} at most, and a new major may redefine words this \
                 loader thinks it knows — nothing is loaded (design §6.1)"
            ),
        );
        return true;
    }
    // "Newer words may be dropped" is only true of a vocabulary this loader
    // postdates. Anything else in the slot — most often a pack that wrote its
    // *library's* version there (`speclib tclinterp 0.15`) — never named a
    // vocabulary at all, and the notice says so instead of implying the
    // loader is behind.
    let msg = if tcl_registry::version::compare(declared, NEWEST_VOCABULARY_VERSION).is_gt() {
        format!(
            "pack declares SpecTcl vocabulary {declared}; this loader knows \
             {NEWEST_VOCABULARY_VERSION} — newer words may be dropped"
        )
    } else {
        format!(
            "`{declared}` is not a SpecTcl vocabulary version (this loader \
             knows {NEWEST_VOCABULARY_VERSION}); if it is the library's own \
             version, it belongs in `introduced_version`, not the `speclib` \
             slot"
        )
    };
    log.say(line, msg);
    false
}

impl PackTables {
    fn add_values(&mut self, stmt: &Stmt, log: &mut Log) {
        let name = stmt.word_text(1).to_owned();
        let Some(body) = stmt.arg(2) else {
            log.say(stmt.line, "`values` needs a name and a block");
            return;
        };
        if self.values.contains_key(&name) {
            log.say(stmt.line, format!("`values {name}` redeclared; last wins"));
        }
        self.values.insert(name, value_rows(&block(body), log));
    }

    fn add_descriptor(&mut self, stmt: &Stmt, log: &mut Log) {
        let key = stmt.word_text(1).to_owned();
        let name = stmt.word_text(2).to_owned();
        let Some(body) = stmt.arg(3) else {
            log.say(stmt.line, "`descriptor` needs a key, a name, and a block");
            return;
        };
        self.descriptors.insert((key, name), block(body));
    }

    fn add_hook(&mut self, stmt: &Stmt, log: &mut Log) {
        let name = stmt.word_text(1).to_owned();
        let (Some(params), Some(body)) = (stmt.arg(2), stmt.arg(3)) else {
            log.say(
                stmt.line,
                "`hook` needs a name, a parameter list, and a body",
            );
            return;
        };
        self.hooks
            .insert(name, (list_words(&params.text), body.text.clone()));
    }

    fn add_default(&mut self, stmt: &Stmt, log: &mut Log) {
        let key = stmt.word_text(1);
        let value = stmt.word_text(2);
        match key {
            "dialects" => self.defaults.surface = parse_dialects(value, stmt.line, log),
            "available" => {
                log.v20(stmt.line, "available");
                let availability = available::from_statement(stmt, 2, log);
                if availability.surface().is_some() {
                    self.defaults.surface = availability.surface();
                }
                if let Some(package) = availability.required_package {
                    self.defaults.required_package = Some(package);
                }
            }
            "required_package" => self.defaults.required_package = Some(leak_str(value)),
            "tcllib_package" => self.defaults.tcllib_package = Some(leak_str(value)),
            "introduced_version" => self.defaults.introduced_version = Some(leak_str(value)),
            "deprecated_version" => self.defaults.deprecated_version = Some(leak_str(value)),
            "retired_version" => self.defaults.retired_version = Some(leak_str(value)),
            "warn_missing_import" => {
                self.defaults.warn_missing_import = Some(parse_flag(stmt.tail()));
            }
            "is_namespace_exported" => {
                self.defaults.is_namespace_exported = Some(parse_flag(stmt.tail()));
            }
            other => log.say(
                stmt.line,
                format!("`default {other}` is not an availability key; dropped"),
            ),
        }
    }

    pub(super) fn descriptor(&self, key: &str, name: &str) -> Option<&[Stmt]> {
        self.descriptors
            .get(&(key.to_owned(), name.to_owned()))
            .map(Vec::as_slice)
    }
}

/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?` rows, plus the three
/// lifecycle flags.
///
/// The two version axes are independent: `-min-tcl` is the Tcl-core rung the
/// value needs, `-introduced` / `-deprecated` / `-retired` are the owning
/// package's releases.
fn value_rows(stmts: &[Stmt], log: &mut Log) -> Vec<ArgValue> {
    let mut out = Vec::new();
    for stmt in stmts {
        if stmt.word_text(0) != "value" {
            log.unknown_property(stmt);
            continue;
        }
        let mut row = ArgValue {
            value: leak_str(stmt.word_text(1)),
            ..ArgValue::DEFAULT
        };
        let mut i = 2;
        let words = &stmt.words;
        while i < words.len() {
            let rejected_flag_word = words[i].clone();
            match words[i].text.as_str() {
                "-detail" => {
                    row.detail = leak_str(&next_text(words, &mut i));
                }
                "-min-tcl" => {
                    let raw = next_text(words, &mut i);
                    match parse_version(&raw) {
                        Some(version) => row.min_tcl = Some(version),
                        None => log.say(stmt.line, format!("unknown Tcl version `{raw}` dropped")),
                    }
                }
                "-code" => {
                    let raw = next_text(words, &mut i);
                    match raw.parse::<i64>() {
                        Ok(code) => row.code = Some(code),
                        Err(_) => log.say(stmt.line, format!("`-code {raw}` is not an integer")),
                    }
                }
                other => {
                    if lifecycle_flag(&mut row.lifecycle, other, words, &mut i) {
                        log.v11(stmt.line, other);
                    } else {
                        log.unknown_flag("value", stmt.line, &rejected_flag_word);
                    }
                }
            }
            i += 1;
        }
        row.lifecycle = checked_lifecycle(
            row.lifecycle,
            &format!("value `{}`", row.value),
            stmt.line,
            log,
        );
        out.push(row);
    }
    out
}
