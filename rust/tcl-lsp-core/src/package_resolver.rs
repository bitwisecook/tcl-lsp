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

//! Tcl package + auto-load resolution, aligned to C Tcl's own loading
//! machinery.
//!
//! # What C Tcl actually does (the reference this module mirrors)
//!
//! * **`package require Name ?ver?`** — when `Name` isn't already present, the
//!   `package unknown` handler [`tclPkgUnknown`] walks `$auto_path` (each
//!   directory *and its immediate subdirectories*, `*/pkgIndex.tcl`),
//!   `source`-ing every `pkgIndex.tcl`. Each runs
//!   `package ifneeded NAME VERSION SCRIPT`, registering the version→script
//!   map. `package require` then evaluates the best-matching version's script,
//!   which `load`s a shared library or `source`s Tcl files — *that* is what
//!   defines the package's commands. See `library/package.tcl:467`
//!   (`tclPkgUnknown`).
//!
//! * **auto-loading an unknown command** — `unknown` → `auto_load` consults the
//!   `auto_index` array, populated by [`auto_load_index`] from `tclIndex` files
//!   found on `$auto_path`. A `tclIndex` maps a (qualified) command name to the
//!   file that `source`s its definition. Name lookup is qualified through
//!   [`auto_qualify`]. See `library/init.tcl:353` (`auto_load`), `:422`
//!   (`auto_load_index`), `:488` (`auto_qualify`).
//!
//! # What this module provides
//!
//! [`parse_pkg_index`] and [`parse_tcl_index`] extract, respectively, the
//! `package ifneeded` source targets and the `auto_index` proc→file mappings
//! from index-file *content* (tokenised with the real Tcl lexer, so nested
//! `[file join $dir x]` / quoted / braced forms are handled structurally, not
//! by regex). [`auto_qualify`] delegates name qualification to the shared
//! naming owner. [`PackageResolver`] ties them together over a set of search paths
//! (an `auto_path`), resolving `package require` and auto-loaded command names
//! to candidate implementation files.
//!
//! The differential tests in this module verify the output against a real
//! `tclsh` (`auto_qualify`, and `pkg_mkIndex` / `auto_mkindex`-generated index
//! files).
//!
//! [`tclPkgUnknown`]: https://www.tcl-lang.org/man/tcl/TclCmd/package.htm
//! [`auto_load_index`]: https://www.tcl-lang.org/man/tcl/TclCmd/library.htm

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::vfs::{NativeStore, SourceStore};

use tcl_lexer::Token;

pub use tcl_dialect::PackagePrefer;

/// The `package prefer` mode in force at `at` in `analysis` — the
/// interpreter-global selection rule [`PackageResolver::resolve_require`]
/// needs.
///
/// `package prefer` is a monotone latch: it starts at `default`, a
/// `package prefer latest` raises it, and
/// nothing lowers it again (`package prefer stable` afterwards is silently
/// ineffective — transcript on
/// [`tcl_compiler::signature_scan::types::SignaturePackagePrefer`]). So the
/// whole state is "has a raise already run at this point", and the answer
/// needs no fold over an ordered log — just one existence test.
///
/// Ordering is [`tcl_compiler::analyser::indirection::in_effect`], the rule
/// every other "had this statement already run?" question in the resolver
/// family uses: a top-level raise counts for a `package require` written
/// anywhere in a proc body of the same file (the file loads before any body
/// runs), while a raise inside a body does not reach back to a top-level
/// require.
///
/// A **conditional** raise (inside `if` / `catch` / `try`) is ignored — it may
/// not run, and flipping the selection rule would move go-to-definition onto a
/// different release of the package.  That abstention is toward `default`.
///
/// # What `default` carries
///
/// The interpreter's own starting mode is not always `stable`:
/// `TCL_PKG_PREFER_LATEST` in the environment makes it `latest`, and on 9.0+
/// so does running an unstable build of Tcl itself (the registry's `prefer`
/// subcommand detail documents both).  Neither is a fact about the source
/// tree, so it is a caller-supplied input rather than an inference —
/// [`PackagePrefer::Stable`] unless the workspace says otherwise.
///
/// It is also where a raise in **another document** arrives.  The state is
/// interpreter-global, so a `package prefer latest` in a file that runs first
/// really does change this file's answer; along the `source` graph "runs
/// first" is a static fact, and
/// [`WorkspaceIndex::source_ancestor_prefers_latest`](crate::workspace_index::WorkspaceIndex::source_ancestor_prefers_latest)
/// answers it.  A raise reaching this file some *other* way (a load order no
/// `source` edge records) is still unseen — the abstention the import family
/// records for every cross-file event
/// (`docs/design/analysis/import-order-source-graph.md`).
#[must_use]
pub fn package_prefer_at(
    analysis: &tcl_compiler::analyser::AnalysisResult,
    at: u32,
    default: PackagePrefer,
) -> PackagePrefer {
    // Monotone latch: once raised, nothing lowers it, so an inherited
    // `latest` default is the answer whatever this document writes.
    if default == PackagePrefer::Latest {
        return PackagePrefer::Latest;
    }
    let raised = analysis.package_prefer_latest.iter().any(|p| {
        !p.conditional
            && tcl_compiler::analyser::indirection::in_effect(analysis, p.range.start(), at)
    });
    if raised {
        PackagePrefer::Latest
    } else {
        default
    }
}

/// Metadata for one `package ifneeded` declaration discovered in a
/// `pkgIndex.tcl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    /// Package name (the `NAME` in `package ifneeded NAME VERSION …`).
    pub name: String,
    /// Declared version.
    pub version: String,
    /// Candidate Tcl implementation files suggested by the deferred script
    /// or its directory. This is source advice, not evidence that a loader ran
    /// or that a file was sourced. Original providers expose each candidate
    /// provenance through [`OriginalPackageInfo::file_candidates`].
    ///
    /// **Empty** means "declared, but its command set is not statically
    /// enumerable" — a binary-only C extension whose `ifneeded` body just
    /// `load`s a shared object, with no companion `.tcl` in its directory.
    /// The package genuinely exists ([`PackageResolver::provides`] answers
    /// `true`, so a `package require` of it is not an unknown package), but
    /// nothing can be said about which commands it installs, so a consumer
    /// must neither claim its commands are missing nor claim they exist.
    pub source_files: Vec<PathBuf>,
    /// The `pkgIndex.tcl` that declared this entry.
    pub pkg_index_path: PathBuf,
    /// What must hold for the interpreter to reach this declaration at all —
    /// empty for the ordinary unguarded index file.  See
    /// [`reachability`] for the guard shapes modelled.
    pub conditions: reachability::Conditions,
}

impl PackageInfo {
    /// Whether a Tcl release running this index registers the declaration.
    ///
    /// `target` is the release the workspace targets; `None` (an unversioned
    /// `tcl` dialect, or a non-Tcl one) leaves every version guard undecided,
    /// so a guarded entry reports [`reachability::Availability::Conditional`]
    /// rather than being guessed either way.
    #[must_use]
    pub fn availability(
        &self,
        target: Option<tcl_dialect::TclVersion>,
    ) -> reachability::Availability {
        self.conditions.availability(target)
    }
}

/// One `proc → file` mapping from a `tclIndex` auto-load index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoIndexEntry {
    /// The (possibly namespace-qualified) command name, e.g. `::http::geturl`.
    pub proc_name: String,
    /// Absolute path to the file that defines it.
    pub source_file: PathBuf,
}

pub(super) fn walk_command_words_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<Vec<Vec<Token>>> {
    let Ok(script) = tcl_compiler::segmenter::segment_script_image_with_offset_and_config(
        &tcl_lexer::SourceImage::document(text),
        0,
        config,
    ) else {
        return Vec::new();
    };
    script
        .commands
        .into_iter()
        .map(|command| {
            command
                .word_fragments
                .into_iter()
                .map(|word| word.into_iter().map(|fragment| fragment.token).collect())
                .collect()
        })
        .collect()
}

/// The verbatim source slice spanning `word` within `text`.
///
/// For a `[...]` command-substitution word the lexer's `Cmd` token excludes
/// the closing `]` from its span, so this returns `[list …` (no trailing
/// bracket).
pub(super) fn word_raw<'t>(text: &'t str, word: &[Token]) -> &'t str {
    let start = word[0].span.start() as usize;
    let end = word[word.len() - 1].span.end() as usize;
    text.get(start..end).unwrap_or("")
}

/// Fixture-only legacy syntax projection for retained parser boundary controls.
#[cfg(test)]
fn source_filename_with_config(
    text: &str,
    arg: &[Token],
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let raw = word_raw(text, arg);

    // `[file join $dir X]` — a single command-substitution word.
    if arg.len() == 1 && arg[0].kind == tcl_lexer::TokenType::Cmd {
        let start = arg[0].span.start() as usize + arg[0].content_offset as usize;
        let end = arg[0].span.end() as usize;
        let inner = text.get(start..end)?;
        let cmds = walk_command_words_with_config(
            inner,
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
        );
        if cmds.len() != 1 {
            return None;
        }
        let words = &cmds[0];
        if words.len() < 4 {
            return None;
        }
        if word_raw(inner, &words[0]) != "file" || word_raw(inner, &words[1]) != "join" {
            return None;
        }
        if !matches!(word_raw(inner, &words[2]), "$dir" | "${dir}") {
            return None;
        }
        // `file join $dir a b c` joins path *components* with the directory
        // separator, not a space: the tail is `a/b/c`, so
        // `[file join $dir src impl.tcl]` resolves `src/impl.tcl` (issue 177).
        // Each component's surrounding quotes/braces are stripped individually
        // so a quoted component with no separator survives intact.
        let tail = words[3..]
            .iter()
            .map(|w| {
                word_raw(inner, w)
                    .trim()
                    .trim_matches(|c| c == '"' || c == '{' || c == '}')
            })
            .collect::<Vec<_>>()
            .join("/");
        return Some(tail);
    }

    // `$dir/X` — bare variable substitution immediately followed by `/tail`.
    for prefix in ["$dir/", "${dir}/"] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            return Some(rest.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Cap on `[...]`/`{...}`/`"..."` wrapper-word nesting depth
/// [`collect_source_targets`] will descend into — pathologically deep
/// nesting in a `pkgIndex.tcl` `package ifneeded` body could otherwise
/// blow the native stack when the workspace indexer scans it. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_SOURCE_TARGET_SCAN_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Walk `commands` collecting `source <arg>` targets that exist under
/// `pkg_dir` (per `exists`), descending into `[...]` / `{...}` / `"..."`
/// wrapper words.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_SOURCE_TARGET_SCAN_DEPTH`] this stops descending into wrapper
/// words rather than recursing further.
fn collect_source_targets(
    text: &str,
    commands: &[Vec<Vec<Token>>],
    pkg_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    out: &mut Vec<PathBuf>,
    depth: u32,
) {
    let config = tcl_lexer::LexerConfig::default();
    let Ok(length) = u32::try_from(text.len()) else {
        return;
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(text),
        tcl_lexer::Span::new(0, length),
        config,
    ) else {
        return;
    };
    let loader = original_names::index_loader::IndexLoader::standalone(text, config);
    let mut candidates = Vec::new();
    for tokens in commands.iter().flatten() {
        let mut matching = plan
            .commands
            .iter()
            .flat_map(|command| &command.words)
            .filter(|word| word.tokens() == tokens.as_slice());
        let Some(word) = matching.next() else {
            continue;
        };
        if matching.next().is_none() {
            loader.collect_word(word, pkg_dir, &mut candidates, depth);
        }
    }
    out.extend(candidates.into_iter().filter(|path| exists(path)));
}

/// A literal advertisement must be a version in at least one supported C Tcl
/// release. The selected profile filters advertisements before selection;
/// discovery never assigns an unspecified source a default Tcl release.
fn is_version_word(word: &str) -> bool {
    tcl_dialect::TclVersion::ALL
        .into_iter()
        .any(|release| tcl_dialect::validate_version_for(word, release))
}

// Public parsers.

/// Parse a `pkgIndex.tcl`'s `content`, returning a [`PackageInfo`] per literal
/// `package ifneeded NAME VERSION …` declaration.
///
/// `pkg_dir` is the directory the index lives in (`$dir` in the script);
/// `exists` decides whether a candidate implementation file is real (pass
/// `|p| p.is_file()` for the filesystem; a closure for tests). When an
/// `ifneeded` body names no source file, this falls back to every `*.tcl`
/// (other than `pkgIndex.tcl`) reported by `list_tcl_files` — matching C
/// Tcl-era `pkg_mkIndex` output, where the index always sources concrete
/// files but hand-written indices sometimes don't.  When *that* finds nothing
/// either the declaration is still returned, with an empty `source_files` —
/// see [`PackageInfo::source_files`] for what that state means and why the
/// declaration must not be dropped.
///
/// Declarations are found through the [`reachability`] scan, so a declaration
/// nested inside an `if` branch is found (the TEA "pick a Tcl 8 or Tcl 9
/// library" shape) and each one carries the guard conditions standing in
/// front of it. Nothing is filtered here: a caller that targets a specific
/// Tcl release asks [`PackageInfo::availability`], and one that does not
/// (go-to-definition, which is deliberately permissive) ignores the field.
/// This standalone survey uses the default Tcl source configuration and
/// encoding-form loader syntax. Candidates retain the supplied directory,
/// existence and fallback-listing conventions.
pub fn parse_pkg_index(
    content: &str,
    pkg_dir: &Path,
    pkg_index_path: &Path,
    exists: &dyn Fn(&Path) -> bool,
    list_tcl_files: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Vec<PackageInfo> {
    // A `pkgIndex.tcl` is plain Tcl whatever dialect the workspace's own
    // documents are in — `tclPkgUnknown` sources it in the ordinary
    // interpreter — so the guard vocabulary (`package`, `if`, `return`) comes
    // from the core registry, not the document's.  The lookup is cached and
    // returns a `&'static`, so this costs nothing per file.
    let registry = crate::registry_for_dialect("tcl");
    let mut out = Vec::new();
    reachability::scan_selected_registrations(
        content,
        registry,
        tcl_lexer::LexerConfig::default(),
        &mut |selected| {
            let registration = selected.registration;
            let reachability::Reached {
                text,
                words,
                conditions,
                ..
            } = selected.reached;
            let Some((name, version, body)) = registration::Registration::advertisement(
                &registration,
                text,
                words,
                registry,
                tcl_lexer::LexerConfig::default(),
            ) else {
                return;
            };
            if !is_version_word(version) {
                return;
            }
            let Some(body) = words.get(body) else {
                return;
            };
            let mut source_files = Vec::new();
            collect_source_targets(
                text,
                &[vec![body.clone()]],
                pkg_dir,
                exists,
                &mut source_files,
                0,
            );
            if source_files.is_empty() {
                for f in list_tcl_files(pkg_dir) {
                    if f.file_name().is_some_and(|n| n != "pkgIndex.tcl") {
                        source_files.push(f);
                    }
                }
            }
            // A declaration with no implementation *file* is still a declaration.
            // The `load`-only C extension (`package ifneeded pix 0.8 [list apply
            // {dir {… load [file join $dir $os $lib] Pix}} $dir]`, whose directory
            // holds nothing but `pkgIndex.tcl` and the shared object) is the
            // shape that must not be dropped: parsing the name and version and
            // then throwing them away makes `provides("pix")` false for a
            // package plainly declared in the workspace, and every W120 in any
            // document requiring it is then suppressed as "unknowable".
            //
            // Recording it with an empty `source_files` is the honest state: the
            // package **exists**, its command set is **not statically
            // enumerable**.  Consumers already distinguish the two — `resolve`
            // answers an empty file list, so nothing tries to read a shared object
            // as Tcl, while `provides` answers `true`.
            out.push(PackageInfo {
                name: name.to_owned(),
                version: version.to_owned(),
                source_files,
                pkg_index_path: pkg_index_path.to_owned(),
                conditions,
            });
        },
    );
    out
}

/// Parse a `tclIndex` auto-load index's `content` into proc→file mappings.
///
/// Handles both formats `auto_load_index` reads (`library/init.tcl:446`):
///
/// * **version 2.0** — `set auto_index(NAME) [list source [file join $dir F]]`
///   (including the standalone `-encoding utf-8` grammar), and
/// * **old format** — `# Tcl autoload index file: each line identifies a Tcl`
///   followed by `NAME FILE` lines.
///
/// `index_dir` is the directory the index lives in; `exists` gates referenced
/// files (pass `|p| p.is_file()`).
/// This standalone survey uses the default Tcl source configuration and
/// encoding-form loader syntax, independently of a document analysis or runtime.
pub fn parse_tcl_index(
    content: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<AutoIndexEntry> {
    let mut entries = Vec::new();
    let first_line = content.lines().next().unwrap_or("");
    let old_format = first_line == "# Tcl autoload index file: each line identifies a Tcl";

    if old_format {
        for line in content.lines().skip(1) {
            if line.trim_start().starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let Ok(parts) = tcl_syntax::word_rules::WordValueRules::from_config(
                &tcl_lexer::LexerConfig::default(),
            )
            .split_list(line) else {
                continue;
            };
            let [name, filename] = parts.as_slice() else {
                continue;
            };
            push_auto_entry(name, filename, index_dir, exists, &mut entries);
        }
        return entries;
    }

    let config = tcl_lexer::LexerConfig::default();
    let Ok(length) = u32::try_from(content.len()) else {
        return entries;
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(content),
        tcl_lexer::Span::new(0, length),
        config,
    ) else {
        return entries;
    };
    let loader = original_names::index_loader::IndexLoader::standalone(content, config);
    for command in plan.commands {
        let [head, name, value] = command.words.as_slice() else {
            continue;
        };
        if tcl_syntax::word_rules::original_static_word_unicode_value(head).as_deref()
            != Some(b"set")
        {
            continue;
        }
        let Some(name) = tcl_syntax::word_rules::original_static_word_unicode_value(name) else {
            continue;
        };
        let Some(proc_name) = std::str::from_utf8(&name).ok().and_then(auto_index_key) else {
            continue;
        };
        let Some(file) = loader.filename(value, index_dir) else {
            continue;
        };
        if exists(&file) {
            entries.push(AutoIndexEntry {
                proc_name: proc_name.to_owned(),
                source_file: file,
            });
        }
    }
    entries
}

/// Extract `NAME` from an `auto_index(NAME)` array-element word.
fn auto_index_key(raw: &str) -> Option<&str> {
    let (root, index) = tcl_syntax::naming::split_element_ref(raw)?;
    (root == "auto_index").then_some(index)
}

fn push_auto_entry(
    proc_name: &str,
    filename: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    out: &mut Vec<AutoIndexEntry>,
) {
    let full = index_dir.join(filename);
    if exists(&full) {
        out.push(AutoIndexEntry {
            proc_name: proc_name.to_owned(),
            source_file: full,
        });
    }
}

/// Package-name candidates from literal `package require NAME` (with optional
/// `-exact`) and version-bearing `package provide NAME VERSION` declarations.
///
/// Used to survey transitive source dependencies: a literal `package require Tk`
/// contributes `Tk` as a candidate even when the containing command may never
/// run. A non-literal name (`package require $x`) is skipped; the survey does
/// not establish the complete provided set or successful package loading.
/// This explicit standalone survey retains the default source configuration;
/// selected Registry source ordinals and decoded literal data grant no package
/// lookup, installation or successful loading authority.
#[must_use]
pub fn package_requires_in(content: &str) -> Vec<String> {
    let config = tcl_lexer::LexerConfig::default();
    let Ok(length) = u32::try_from(content.len()) else {
        return Vec::new();
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(content),
        tcl_lexer::Span::new(0, length),
        config,
    ) else {
        return Vec::new();
    };
    if plan.fatal_tail.is_some() {
        return Vec::new();
    }
    let context = tcl_registry::model::ingress::static_context_for("tcl");
    let mut dialect =
        tcl_registry::InvocationDialect::of_profile(tcl_dialect::DialectProfile::plain_tcl());
    dialect.lexer_grammar = config.grammar_over(dialect.lexer_grammar);
    dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let mut names = Vec::new();
    for command in plan.commands {
        let values = command
            .words
            .iter()
            .map(|word| {
                String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
                    word,
                )?)
                .ok()
            })
            .collect::<Vec<_>>();
        if values.iter().flatten().any(|value| value.contains('\0')) {
            continue;
        }
        let words = command
            .words
            .iter()
            .zip(&values)
            .map(|(word, value)| {
                if word.group().expand {
                    tcl_registry::InvocationWord::Expanded
                } else {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::Literal,
                    )
                }
            })
            .collect::<Vec<_>>();
        let Some((&head, arguments)) = words.split_first() else {
            continue;
        };
        let Some(schema) =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(head, arguments).with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::InterpreterRuntime,
            )
            .resolved()
        else {
            continue;
        };
        let Some(reference) = schema.authored_source_package_reference() else {
            continue;
        };
        if let Some(Some(name)) = values.get(reference.argument + 1) {
            names.push(name.clone());
        }
    }
    names
}

fn candidate_package_infos<'a>(
    infos: &[&'a PackageInfo],
    requirements: &[&str],
    exact: bool,
    prefer: PackagePrefer,
    target: Option<tcl_dialect::TclVersion>,
) -> Vec<&'a PackageInfo> {
    let releases: Vec<_> = target.map_or_else(
        || tcl_dialect::TclVersion::ALL.to_vec(),
        |version| vec![version],
    );
    let mut candidates: Vec<&PackageInfo> = Vec::new();
    for release in releases {
        let available: Vec<_> = infos
            .iter()
            .copied()
            .filter(|info| {
                info.availability(Some(release)) != reachability::Availability::Unavailable
            })
            .collect();
        let certain: Vec<_> = available
            .iter()
            .copied()
            .filter(|info| {
                info.availability(Some(release)) == reachability::Availability::Available
            })
            .collect();
        let versions: Vec<_> = certain.iter().map(|info| info.version.as_str()).collect();
        let selected = if exact {
            requirements
                .first()
                .filter(|_| requirements.len() == 1)
                .and_then(|requested| {
                    tcl_dialect::select_package_version_exact_for(&versions, requested, release)
                })
        } else {
            tcl_dialect::select_package_version_for(&versions, requirements, prefer, release)
        };
        let selected_version = selected.map(|index| versions[index]);
        for info in available {
            let accepts = if exact {
                requirements
                    .first()
                    .filter(|_| requirements.len() == 1)
                    .is_some_and(|requested| {
                        tcl_dialect::version_matches_exact_for(&info.version, requested, release)
                    })
            } else {
                tcl_dialect::select_package_version_for(
                    &[info.version.as_str()],
                    requirements,
                    prefer,
                    release,
                )
                .is_some()
            };
            if accepts
                && (info.availability(Some(release)) == reachability::Availability::Conditional
                    || selected_version.is_some_and(|version| {
                        tcl_dialect::compare_versions_for(&info.version, version, release)
                            == core::cmp::Ordering::Equal
                    }))
                && !candidates.iter().any(|candidate| **candidate == *info)
            {
                candidates.push(info);
            }
        }
    }
    candidates
}

/// Shared C Tcl autoload keys for a command in its canonical namespace.
/// Global keys omit `::`; relative names try the namespace before global scope.
#[must_use]
pub fn auto_qualify(cmd: &str, namespace: &str) -> Vec<String> {
    tcl_syntax::naming::autoload_command_candidates(cmd, namespace)
}

// PackageResolver — ties the parsers to a set of search paths (auto_path).

/// Resolves `package require` and auto-loaded command names to the files C
/// Tcl would load, by scanning `pkgIndex.tcl` / `tclIndex` files across a set
/// of search paths (an `auto_path`).
///
/// Every provider discovered for a name is kept, in discovery order (each
/// search path in turn, its subdirectories sorted by name). Which one a
/// `package require` picks is decided by [`PackageResolver::resolve_require`]
/// with C Tcl's own selection rule — the highest acceptable version, not the
/// first discovered — and *ties are broken by discovery order*, so listing a
/// workspace-local copy first still shadows an equally-versioned installed
/// one.
#[derive(Debug)]
pub struct PackageResolver {
    packages: HashMap<String, Vec<PackageInfo>>,
    auto_index: HashMap<String, Vec<PathBuf>>,
    original_packages: HashMap<
        tcl_registry::native_package::NativePackageNameKey,
        Vec<original_names::OriginalPackageInfo>,
    >,
    original_auto_index: HashMap<
        (
            tcl_syntax::native_string::NativeStringProtocol,
            tcl_core_types::NameBytes,
        ),
        Vec<original_names::OriginalAutoIndexRegistration>,
    >,
    scanned_dirs: Vec<PathBuf>,
    revision: u64,
}

/// Source of [`PackageResolver::revision`] values.
///
/// Drawn from one process-wide counter (rather than a per-resolver `+= 1`) so a
/// revision identifies *which* database state, not just how many mutations it
/// has seen: the server replaces the resolver wholesale on a rescan
/// (`*package_resolver.write().await = resolver`), and a per-instance counter
/// would let a fresh resolver's low revision collide with an older, already-
/// cached one. Every value handed out here is unique for the process lifetime,
/// so "same revision" means "same resolver contents" for any consumer caching
/// a derived view.
static RESOLVER_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_resolver_revision() -> u64 {
    RESOLVER_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Default for PackageResolver {
    fn default() -> Self {
        Self {
            packages: HashMap::new(),
            auto_index: HashMap::new(),
            original_auto_index: HashMap::new(),
            original_packages: HashMap::new(),
            scanned_dirs: Vec::new(),
            revision: next_resolver_revision(),
        }
    }
}

impl PackageResolver {
    /// A new, empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An opaque token identifying this resolver's current contents.
    ///
    /// Bumped by every mutation of the package database
    /// ([`Self::add_pkg_index`] and [`Self::add_tcl_index`], including the
    /// calls [`Self::scan_path`] / [`Self::scan_tree`] make when they discover
    /// index files) and unique across resolver instances, so a consumer
    /// caching something derived from the package database (the recovery
    /// known-command widening in `tcl-lsp-server`) can tell whether its cache
    /// is still valid without diffing the database. A scan that discovers no
    /// index files records the directory as visited but leaves the revision
    /// unchanged — nothing a consumer can derive differs, so its caches stay
    /// valid.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Note a mutation: take a fresh, process-unique revision.
    fn invalidate(&mut self) {
        self.revision = next_resolver_revision();
    }

    /// Record a parsed `pkgIndex.tcl`'s declarations, appended in discovery
    /// order (selection among them is
    /// [`Self::resolve_require`]'s job, not this one's).
    pub fn add_pkg_index(&mut self, infos: Vec<PackageInfo>) {
        self.invalidate();
        for info in infos {
            self.packages
                .entry(info.name.clone())
                .or_default()
                .push(info);
        }
    }

    /// Record a parsed `tclIndex`'s declarations.
    pub fn add_tcl_index(&mut self, entries: Vec<AutoIndexEntry>) {
        self.invalidate();
        for entry in entries {
            self.auto_index
                .entry(entry.proc_name)
                .or_default()
                .push(entry.source_file);
        }
    }

    /// Scan one directory and its **immediate subdirectories** for
    /// `pkgIndex.tcl` and `tclIndex` files, mirroring `tclPkgUnknown`'s
    /// `glob -directory $dir -join * pkgIndex.tcl` plus the `$dir/pkgIndex.tcl`
    /// case (`library/package.tcl:486-534`). Idempotent per directory.
    ///
    /// Reads the real filesystem; [`Self::scan_path_in`] is the same scan over
    /// a caller-supplied [`SourceStore`].
    pub fn scan_path(&mut self, dir: &Path) {
        self.scan_path_in(&NativeStore, dir);
    }

    /// [`Self::scan_path`] against `store` rather than `std::fs`, so a host
    /// that has the bytes but no filesystem (the browser worker) builds the
    /// same package database from them. See [`crate::vfs`].
    pub fn scan_path_in(&mut self, store: &dyn SourceStore, dir: &Path) {
        if !store.is_dir(dir) {
            return;
        }
        // The directory itself, then each immediate subdirectory in name
        // order — `read_dir` order is filesystem-dependent, and discovery
        // order decides which provider an unconstrained `resolve` answers, so
        // it must not vary across machines.
        self.scan_single_dir(store, dir);
        for path in sorted_subdirs(store, dir) {
            self.scan_single_dir(store, &path);
        }
    }

    /// Recursively scan a directory tree (bounded to `max_dirs` directories)
    /// for `pkgIndex.tcl` / `tclIndex` files.
    ///
    /// Unlike [`Self::scan_path`] — which follows C Tcl's `auto_path` rule of
    /// scanning a directory and its *immediate* subdirectories — this walks
    /// the whole tree, the right behaviour for an IDE workspace root where a
    /// project may nest its packages arbitrarily deep (a full recursive walk).
    /// The `max_dirs` cap keeps a huge tree from stalling the scan.
    ///
    /// Reads the real filesystem; [`Self::scan_tree_in`] is the same walk over
    /// a caller-supplied [`SourceStore`].
    pub fn scan_tree(&mut self, root: &Path, max_dirs: usize) {
        self.scan_tree_in(&NativeStore, root, max_dirs);
    }

    /// [`Self::scan_tree`] against `store` rather than `std::fs`. See
    /// [`Self::scan_path_in`].
    pub fn scan_tree_in(&mut self, store: &dyn SourceStore, root: &Path, max_dirs: usize) {
        let mut stack = vec![root.to_path_buf()];
        let mut visited = 0usize;
        while let Some(dir) = stack.pop() {
            if visited >= max_dirs {
                break;
            }
            if !store.is_dir(&dir) {
                continue;
            }
            visited += 1;
            self.scan_single_dir(store, &dir);
            // Reverse-sorted push so the LIFO pop visits children in name
            // order, keeping discovery order machine-independent.
            for path in sorted_subdirs(store, &dir).into_iter().rev() {
                stack.push(path);
            }
        }
    }

    fn scan_single_dir(&mut self, store: &dyn SourceStore, dir: &Path) {
        let dir_buf = dir.to_path_buf();
        if self.scanned_dirs.contains(&dir_buf) {
            return;
        }
        self.scanned_dirs.push(dir_buf);
        let pkg_index = dir.join("pkgIndex.tcl");
        if store.is_file(&pkg_index)
            && let Ok(content) = store.read_to_string(&pkg_index)
        {
            let infos = parse_pkg_index(&content, dir, &pkg_index, &|p| store.is_file(p), &|d| {
                list_tcl_files(store, d)
            });
            self.add_original_pkg_index(&content, dir, &pkg_index, &|p| store.is_file(p), &|d| {
                list_tcl_files(store, d)
            });
            self.add_pkg_index(infos);
        }
        // `tclIndex` is matched case-insensitively (`tclIndex` / `tclindex`).
        if let Ok(entries) = store.read_dir(dir) {
            for entry in entries {
                let is_tcl_index = entry
                    .path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("tclindex"));
                if is_tcl_index && let Ok(content) = store.read_to_string(&entry.path) {
                    let dir_owned = entry.path.parent().unwrap_or(dir).to_path_buf();
                    self.add_original_tcl_index(&content, &dir_owned, &|p| store.is_file(p));
                    self.add_tcl_index(parse_tcl_index(&content, &dir_owned, &|p| {
                        store.is_file(p)
                    }));
                }
            }
        }
    }

    /// Resolve a `package require NAME ?VERSION?` to its implementation files.
    ///
    /// Shorthand for [`Self::resolve_require`] with `exact` false and the
    /// interpreter's default `package prefer` mode; see there for the
    /// selection rule and its oracle.
    #[must_use]
    pub fn resolve(&self, name: &str, version: Option<&str>) -> Vec<PathBuf> {
        self.resolve_require(name, version, false, PackagePrefer::default())
    }

    /// Resolve a `package require ?-exact? NAME ?VERSION?` to its
    /// implementation files, empty when nothing acceptable is on the scanned
    /// paths (which includes the "constraint no provider satisfies" case —
    /// never a silent fallback to some other release).
    ///
    /// The selection is [`tcl_dialect::select_package_version`], a port of C
    /// Tcl's `SelectPackage`: among the providers satisfying the requirement,
    /// the **highest** version wins, preferring the highest *stable* one
    /// unless `prefer` says otherwise ([`PackagePrefer::Stable`] is the
    /// interpreter default on 8.6.14 and 9.0.4 alike; the caller derives the
    /// mode in force at its own `package require` with
    /// [`package_prefer_at`]).
    ///
    /// | Written | Means | picks, from 1.5 + 2.3 |
    /// |---------|-------|-----------------------|
    /// | *(none)* | `0-` — every version is acceptable | 2.3 |
    /// | `1.2` | `[1.2, 2)` — up to but excluding the next major | 1.5 |
    /// | `2.0` | `[2.0, 3)` | 2.3 |
    /// | `2.0-` | `[2.0, ∞)` | 2.3 |
    /// | `2.0-2.2` | `[2.0, 2.2)` | *nothing* |
    /// | `-exact 2.0` | the degenerate range `2.0-2.0` | *nothing* |
    ///
    /// `-exact` is the requirement `VERSION-VERSION`
    /// ([`tcl_dialect::exact_requirement`]) — the same shape `tclPkg.c` builds
    /// — so it accepts a trailing-zero spelling of the same release (`2.0.0`)
    /// and rejects both a later release and that release's alpha (`2.0a1`).
    /// `-exact` with no version is `package require -exact NAME`, which real
    /// Tcl rejects as a syntax error; it is treated here as unconstrained.
    ///
    /// **Equal-comparing duplicate providers contribute every one of their
    /// files**. C Tcl collapses two `package ifneeded`
    /// registrations whose versions compare equal (`1.0` / `1.0.0`, `5` /
    /// `0005`) into one entry, keeping the *first* registration's version
    /// string and the *last* one's script — and the registration order is
    /// `glob` order, i.e. filesystem order, so which script that is varies by
    /// machine and no oracle can pin it. [`Self::select_provider`] keeps the
    /// first-discovered provider, which is deterministic and is also the one
    /// whose version string C Tcl reports; this returns the union of the
    /// equal-comparing providers' files, in discovery order, because the
    /// question it answers is "which files may hold this package's
    /// definitions" and the honest answer names all of them rather than
    /// betting on a filesystem order. Navigation then finds a symbol in
    /// whichever copy really loaded, instead of silently missing it half the
    /// time.
    #[must_use]
    pub fn resolve_require(
        &self,
        name: &str,
        version: Option<&str>,
        exact: bool,
        prefer: PackagePrefer,
    ) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        for info in self.select_providers(name, version, exact, prefer) {
            for file in &info.source_files {
                if !files.contains(file) {
                    files.push(file.clone());
                }
            }
        }
        files
    }

    /// A representative assistance candidate, in discovery order.
    ///
    /// This carries the first registered version spelling for navigation; it
    /// does not identify the currently installed loader. Equal-comparing
    /// registrations retain the first key spelling but replace its script,
    /// while registration guards and an unpinned release can leave distinct
    /// candidate versions. Semantic callers must establish execution order and
    /// provider provenance separately.
    #[must_use]
    pub fn select_provider(
        &self,
        name: &str,
        version: Option<&str>,
        exact: bool,
        prefer: PackagePrefer,
    ) -> Option<&PackageInfo> {
        self.select_providers(name, version, exact, prefer)
            .into_iter()
            .next()
    }

    /// Assistance candidates across supported C releases, preserving discovery
    /// order. This does not promote unknown guards to registered loaders.
    #[must_use]
    fn select_providers(
        &self,
        name: &str,
        version: Option<&str>,
        exact: bool,
        prefer: PackagePrefer,
    ) -> Vec<&PackageInfo> {
        let requirements: Vec<&str> = version.into_iter().collect();
        self.candidate_providers_for_require(name, &requirements, exact, prefer, None)
    }

    /// Candidate loader declarations under an explicit C Tcl target.
    /// Unknown release or guards return alternatives, never an executing-loader proof.
    #[must_use]
    pub fn candidate_providers_for_require(
        &self,
        name: &str,
        requirements: &[&str],
        exact: bool,
        prefer: PackagePrefer,
        target: Option<tcl_dialect::TclVersion>,
    ) -> Vec<&PackageInfo> {
        let Some(infos) = self.packages.get(name) else {
            return Vec::new();
        };
        candidate_package_infos(
            &infos.iter().collect::<Vec<_>>(),
            requirements,
            exact,
            prefer,
            target,
        )
    }

    /// Assistance source candidates using the document's requirement grammar.
    /// This does not prove a loader ran or that its commands remain installed.
    #[must_use]
    pub fn resolve_require_for_profile(
        &self,
        name: &str,
        requirements: &[&str],
        exact: bool,
        prefer: PackagePrefer,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Vec<PathBuf> {
        if profile.is_some_and(|profile| {
            !profile
                .grammar_union
                .contains(&tcl_dialect::model::SpecProvider::Core(
                    tcl_dialect::model::Family::Tcl,
                ))
                && profile.runtime_version().is_none()
        }) {
            return Vec::new();
        }
        let mut files = Vec::new();
        for provider in self.candidate_providers_for_require(
            name,
            requirements,
            exact,
            prefer,
            profile.and_then(tcl_dialect::DialectProfile::runtime_version),
        ) {
            for file in &provider.source_files {
                if !files.contains(file) {
                    files.push(file.clone());
                }
            }
        }
        files
    }

    /// Whether the scanned paths know how to provide `name`.
    #[must_use]
    pub fn provides(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Transitive package-name candidates from indexed implementation sources.
    ///
    /// Starts from `roots` (a document's own `package require` names) and, for
    /// each indexed package, reads its implementation files (`read_file`) and
    /// folds in the literal names they `require` / `provide`, to a fixpoint.
    /// This source survey can include commands that never run and establishes
    /// neither successful `ifneeded` evaluation nor installed packages. The
    /// returned set includes `roots`; roots with no readable implementation
    /// contribute only themselves, without proving their full provided set.
    #[must_use]
    pub fn transitive_available_packages(
        &self,
        roots: &[String],
        read_file: &dyn Fn(&Path) -> Option<String>,
    ) -> HashSet<String> {
        let mut available: HashSet<String> = HashSet::new();
        let mut work: Vec<String> = roots.to_vec();
        while let Some(pkg) = work.pop() {
            if !available.insert(pkg.clone()) {
                continue;
            }
            for file in self.resolve(&pkg, None) {
                if let Some(content) = read_file(&file) {
                    for req in package_requires_in(&content) {
                        if !available.contains(&req) {
                            work.push(req);
                        }
                    }
                }
            }
        }
        available
    }

    /// Resolve an auto-loaded command `cmd` (used in `namespace`) to the files
    /// that would define it, trying each [`auto_qualify`] candidate in order —
    /// the same lookup `auto_load` performs against `auto_index`.
    #[must_use]
    pub fn resolve_auto_command(&self, cmd: &str, namespace: &str) -> Vec<PathBuf> {
        for candidate in auto_qualify(cmd, namespace) {
            if let Some(files) = self.auto_index.get(&candidate) {
                return files.clone();
            }
        }
        Vec::new()
    }

    /// Every package name known to the scanned paths.
    #[must_use]
    pub fn package_names(&self) -> Vec<&str> {
        self.packages.keys().map(String::as_str).collect()
    }

    /// All registered package implementation paths, before version selection.
    /// This inventory lets callers capture each file's own source configuration;
    /// it does not establish that any provider is selected or a loader ran.
    #[must_use]
    pub fn provider_source_files(&self) -> Vec<PathBuf> {
        let mut paths = self
            .packages
            .values()
            .flatten()
            .flat_map(|provider| provider.source_files.iter().cloned())
            .chain(
                self.original_packages
                    .values()
                    .flatten()
                    .flat_map(|provider| provider.metadata().source_files.iter().cloned()),
            )
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    /// Every auto-loadable command name known to the scanned paths.
    #[must_use]
    pub fn auto_command_names(&self) -> Vec<&str> {
        self.auto_index.keys().map(String::as_str).collect()
    }

    /// Whether the scanned `auto_path` can auto-load the bare command `cmd`
    /// (used in `namespace`) — i.e. some `tclIndex` on the path maps one of its
    /// [`auto_qualify`] candidates to a defining file.
    ///
    /// This is the package-database analogue of the built-in command registry:
    /// a command an installed library makes available through its auto-load
    /// index is as *resolvable* as a built-in, so the unknown-command (W123)
    /// check must treat it as known rather than flag it. The
    /// decision is data-driven — it never matches on a specific command name.
    #[must_use]
    pub fn auto_loads_command(&self, cmd: &str, namespace: &str) -> bool {
        !self.resolve_auto_command(cmd, namespace).is_empty()
    }

    /// The union of command names defined by the implementation files of every
    /// package in `available`, each resolved to its source files (via
    /// [`Self::resolve`]) and then read + parsed by the caller-supplied
    /// `defined_commands` extractor.
    ///
    /// The extractor is injected (rather than parsing here) so the command-name
    /// discovery can run through the compiler's registry-driven symbol-definer
    /// machinery — the same `proc` / `oo::class` / `interp alias` set the
    /// analyser records — instead of a hand-rolled `proc`-name scan. Keeping it
    /// out of the resolver also keeps this crate's package database free of a
    /// compile dependency on the analyser and makes the method unit-testable
    /// with a stub extractor.
    ///
    /// Used by the W123 refinement for a command provided by a `pkgIndex.tcl`
    /// package with no `tclIndex` (so [`Self::auto_loads_command`] can't see it)
    /// that the document's requires — or the requires it inherits from a
    /// `source` ancestor / entry point — pull in.
    #[must_use]
    pub fn package_defined_commands(
        &self,
        available: &[String],
        target: Option<tcl_dialect::TclVersion>,
        defined_commands: &dyn Fn(&Path) -> Vec<String>,
    ) -> HashSet<String> {
        let mut out = HashSet::new();
        for pkg in available {
            for file in self.reachable_files(pkg, target) {
                out.extend(defined_commands(&file));
            }
        }
        out
    }

    /// The implementation files of `name` that a Tcl release `target` would
    /// actually load, skipping any `package ifneeded` its `pkgIndex.tcl`
    /// guards out ([`reachability`]).
    ///
    /// A declaration whose guards cannot be decided
    /// ([`reachability::Availability::Conditional`]) counts as loadable: the
    /// scan does not know it *won't* run, and treating "unsure" as "absent"
    /// would resurrect exactly the false unknown-command reports this
    /// modelling exists to remove.
    #[must_use]
    pub fn reachable_files(
        &self,
        name: &str,
        target: Option<tcl_dialect::TclVersion>,
    ) -> Vec<PathBuf> {
        let Some(infos) = self.packages.get(name) else {
            return Vec::new();
        };
        infos
            .iter()
            .filter(|i| i.availability(target) != reachability::Availability::Unavailable)
            .flat_map(|i| i.source_files.iter().cloned())
            .collect()
    }
}

/// The immediate subdirectories of `dir`, sorted by name — `read_dir` yields
/// filesystem order, and everything downstream of a scan (which provider wins
/// a version tie, the order of a package's source files) must be identical on
/// every machine.
///
/// The kind test is [`SourceStore::is_dir`] on the entry path, not the entry's
/// own type: a `lib -> ../lib` symlink is a directory C Tcl's `auto_path` walk
/// descends into, and the pre-store code asked `path.is_dir()` for exactly that
/// reason.
fn sorted_subdirs(store: &dyn SourceStore, dir: &Path) -> Vec<PathBuf> {
    let mut subdirs = Vec::new();
    if let Ok(entries) = store.read_dir(dir) {
        for entry in entries {
            if store.is_dir(&entry.path) {
                subdirs.push(entry.path);
            }
        }
    }
    subdirs.sort();
    subdirs
}

/// List the `*.tcl` files directly in `dir` (the `pkg_mkIndex` fallback),
/// sorted by name for machine-independent output.
fn list_tcl_files(store: &dyn SourceStore, dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = store.read_dir(dir) {
        for entry in entries {
            if entry.path.extension().is_some_and(|e| e == "tcl") {
                files.push(entry.path);
            }
        }
    }
    files.sort();
    files
}

mod original_names;
pub mod reachability;
mod registration;
pub use original_names::{
    OriginalPackageFileCandidate, OriginalPackageFileCandidateKind, OriginalPackageInfo,
    PackageRequirementAdvice, PackageRequirementAdviceKey,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod standalone_filename_data_tests {
    use super::*;

    #[test]
    fn standalone_indices_keep_actual_filename_values_and_default_source_grammar() {
        let directory = Path::new("/library");
        for (loader, relative) in [
            (
                "[list source [file join $dir { spaced.tcl }]]",
                " spaced.tcl ",
            ),
            (
                "[list source [file join $dir {\"quoted\".tcl}]]",
                "\"quoted\".tcl",
            ),
            (
                "[list source [file join $dir {cash$name.tcl}]]",
                "cash$name.tcl",
            ),
            ("[list source [file join $dir {} empty.tcl]]", "empty.tcl"),
            (
                "{source [file join $dir {a b} {part\".tcl}]}",
                "a b/part\".tcl",
            ),
            ("\"source $dir/plain.tcl\"", "plain.tcl"),
            (
                "[list source -encoding utf-8 [file join $dir -literal.tcl]]",
                "-literal.tcl",
            ),
        ] {
            let expected = directory.join(relative);
            let source = format!("set auto_index(p) {loader}");
            let entries = parse_tcl_index(&source, directory, &|path| path == expected);
            assert_eq!(entries.len(), 1, "{source}");
            assert_eq!(entries[0].source_file, expected, "{source}");
            assert_eq!(entries[0].proc_name, "p");
            let source = format!("package ifneeded p 1.0 {loader}");
            let packages = parse_pkg_index(
                &source,
                directory,
                &directory.join("pkgIndex.tcl"),
                &|path| path == expected,
                &|_| Vec::new(),
            );
            assert_eq!(packages.len(), 1, "{source}");
            assert_eq!(packages[0].source_files, vec![expected], "{source}");
        }
        for observed in [
            include_str!("../tests/data/standalone_index_keys/8.4.20.stdout"),
            include_str!("../tests/data/standalone_index_keys/8.5.19.stdout"),
            include_str!("../tests/data/standalone_index_keys/8.6.18.stdout"),
            include_str!("../tests/data/standalone_index_keys/9.0.4.stdout"),
            include_str!("../tests/data/standalone_index_keys/9.1.0.stdout"),
            include_str!("../tests/data/standalone_index_keys/jim.stdout"),
        ] {
            assert_eq!(
                observed.lines().collect::<Vec<_>>(),
                vec![
                    "KEY|207020|5350414345",
                    "KEY||454d505459",
                    "KEY|3a3a4e3a3a70|5155414c4946494544",
                    "KEY|612962|434c4f5345"
                ]
            );
        }
        for (name, expected) in [
            ("auto_index( p )", " p "),
            ("auto_index()", ""),
            ("auto_index(::N::p)", "::N::p"),
            ("auto_index(a)b)", "a)b"),
        ] {
            let source = format!("set {{{name}}} [list source [file join $dir p.tcl]]");
            let entries = parse_tcl_index(&source, directory, &|_| true);
            assert_eq!(entries.len(), 1, "{source}");
            assert_eq!(entries[0].proc_name, expected, "{source}");
        }
        let source = "# Tcl autoload index file: each line identifies a Tcl\n{ spaced command } { spaced.tcl }\nq {\"quoted\".tcl}\nr spaced\\ \n";
        let entries = parse_tcl_index(source, directory, &|_| true);
        assert_eq!(entries[0].proc_name, " spaced command ");
        assert_eq!(entries[0].source_file, directory.join(" spaced.tcl "));
        assert_eq!(entries[1].source_file, directory.join("\"quoted\".tcl"));
        assert_eq!(entries[2].source_file, directory.join("spaced "));
    }

    #[test]
    fn standalone_loader_candidates_refuse_unknown_expanded_and_malformed_data() {
        let directory = Path::new("/library");
        for loader in [
            "[mystery source [file join $dir p.tcl]]",
            "[concat source [file join $dir p.tcl]]",
            "[list source {*}$files]",
            "[list source [file join $dir $tail]]",
            "[list source [file join $dir p.tcl] extra]",
            "[list source -encoding utf-8 [file join $dir p.tcl] extra]",
            "[list source -nopkg [file join $dir p.tcl]]",
            "[list ::tcl::Pkg::source [file join $dir p.tcl]]",
        ] {
            let source = format!("set auto_index(p) {loader}");
            assert!(
                parse_tcl_index(&source, directory, &|_| true).is_empty(),
                "{source}"
            );
            let source = format!("package ifneeded p 1.0 {loader}");
            let packages = parse_pkg_index(
                &source,
                directory,
                &directory.join("pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
            );
            assert_eq!(packages.len(), 1, "{source}");
            assert!(packages[0].source_files.is_empty(), "{source}");
        }
        let source = "package ifneeded p 1.0 {puts source [file join $dir p.tcl]}";
        let packages = parse_pkg_index(
            source,
            directory,
            &directory.join("pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
        );
        assert!(packages[0].source_files.is_empty());
    }
}
