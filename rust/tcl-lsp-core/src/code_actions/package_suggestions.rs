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

//! `Add 'package require <pkg>'` suggestions for an unresolved command head.

use super::context_diagnostics::ContextDiagnostic;
use super::{ActionKind, CodeAction, diagnostic_context, ranges_overlap};
use crate::definition::LspRange;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lexer::LineIndex;

/// Reviewed package-loading suggestions for an original unresolved,
/// namespace-qualified command head touched by the request. The actual full
/// context owns the catalogue, and matching a namespace to package metadata
/// supplies a suggestion only: it establishes neither installation nor which
/// commands that package will provide.
///
/// Native advice requires the genuine emitting W123 subject and its original
/// static lookup. Explicit Logical advice requires its positively sealed whole
/// source vector, shared positioned absence and whole-program declarations.
/// Dynamic providers, unsupported original names and existing selected source
/// requirements/provisions withdraw advice. External diagnostic prose supplies
/// no substitute for the original owner. Insertion geometry comes from the same
/// root-source owner as W120, preserving split commands and input grammar.
#[must_use]
pub fn package_require_actions(
    source: &str,
    range: LspRange,
    registry: &tcl_registry::CommandRegistry,
    analysis: Option<&AnalysisResult>,
    context_diagnostics: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    package_require_actions_in_program(
        source,
        range,
        crate::definition::CallResolution::document_only().with_registry(registry),
        analysis,
        context_diagnostics,
    )
}

/// Explicit new-source declaration, without package installation or execution identity.
pub struct PackageRequireSource {
    /// Complete proposed source, or the unchanged source for an existing requirement.
    pub source: String,
    /// Whether a genuine current source reference already names the intended value.
    pub already_required: bool,
}

/// Propose one requested declaration under the complete current source input.
/// Explicit Unicode source-value comparison remains separate from Native keys.
#[must_use]
pub fn package_require_source(
    source: &str,
    analysis: &AnalysisResult,
    package: &str,
) -> Option<PackageRequireSource> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let word = tcl_syntax::word_rules::lexical_unicode_source_word(package, current.config())?;
    let command = format!("package require {word}\n");
    // This fresh proposed-source owner grants authoring metadata, not a Native entry.
    let owner =
        tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
            &command,
            analysis.resolved_input.as_ref()?.clone(),
        );
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        &command,
        0,
        current.config(),
    );
    let [segment] = commands.as_slice() else {
        return None;
    };
    let words = owner.words(segment)?;
    let reference = words.package_reference(&current.context())?;
    if reference.kind() != tcl_registry::source_navigation::SourcePackageReferenceKind::Require
        || reference.matches_unicode_source_value(
            &tcl_lexer::SourceImage::document(&command),
            current.config(),
            package,
        ) != Some(true)
    {
        return None;
    }
    let census = crate::source_structure::SourcePackagePresenceCensus::capture(source, analysis)?;
    let already_required = census.already_names_unicode_source_value(source, analysis, package)?;
    if already_required {
        return Some(PackageRequireSource {
            source: source.to_owned(),
            already_required,
        });
    }
    let offset = tcl_compiler::registry_invocation::source_structure::original_package_require_insert_offset(source, analysis)?;
    let offset = usize::try_from(offset).ok()?;
    let (before, after) = (source.get(..offset)?, source.get(offset..)?);
    let separator = if !before.is_empty() && !before.ends_with('\n') {
        "\n"
    } else {
        ""
    };
    Some(PackageRequireSource {
        source: format!("{before}{separator}{command}{after}"),
        already_required,
    })
}

/// Package suggestions under the caller's whole-program declaration view.
/// Workspace declarations may withdraw source advice; they cannot establish a
/// Native slot, original source producer or package installation.
#[must_use]
pub fn package_require_actions_in_program(
    source: &str,
    range: LspRange,
    resolution: crate::definition::CallResolution<'_>,
    analysis: Option<&AnalysisResult>,
    context_diagnostics: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    // No analysis means no evidence, and evidence is the whole gate.
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    if analysis.has_dynamic_providers
        || analysis.body_lexer_config.is_none_or(|config| {
            !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        })
    {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    let Some(package) = missing_package_for_head_at(
        source,
        range,
        resolution,
        analysis,
        context_diagnostics,
        &line_index,
    ) else {
        return Vec::new();
    };
    let Some(insert_offset) =
        tcl_compiler::registry_invocation::source_structure::original_package_require_insert_offset(
            source, analysis,
        )
    else {
        return Vec::new();
    };
    let insertion = line_index.position_at_utf16(insert_offset, source);
    let separator = if usize::try_from(insert_offset).ok() == Some(source.len())
        && !source.is_empty()
        && !source.ends_with('\n')
    {
        "\n"
    } else {
        ""
    };
    vec![CodeAction {
        title: format!("Add 'package require {package}'"),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: insertion.line,
                start_character: insertion.character.get(),
                end_line: insertion.line,
                end_character: insertion.character.get(),
            },
            new_text: format!("{separator}package require {package}\n"),
        }],
        kind: ActionKind::QuickFix,
        command: None,
        data_group_definition: None,
        disabled: None,
    }]
}

/// The package a command head the request range touches appears to need, or
/// `None` when any of [`package_require_actions`]'s gates fails.
fn missing_package_for_head_at(
    source: &str,
    range: LspRange,
    resolution: crate::definition::CallResolution<'_>,
    analysis: &AnalysisResult,
    _context_diagnostics: &[ContextDiagnostic],
    line_index: &LineIndex,
) -> Option<String> {
    resolution.registry?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let ownership = context.namespace_package_ownership();
    for invocation in &analysis.command_invocations {
        let start = line_index.position_at_utf16(invocation.range.start(), source);
        let end = line_index.position_at_utf16(invocation.range.end(), source);
        let head_range = LspRange {
            start_line: start.line,
            start_character: start.character.get(),
            end_line: end.line,
            end_character: end.character.get(),
        };
        // Gate 1: the range must touch this head.
        if !ranges_overlap(head_range, range) {
            continue;
        }
        if let Some(input) = invocation.original_name_input.as_ref() {
            let Some(key) = input.original_word_key() else {
                continue;
            };
            let Some(lookup) = invocation.original_lookup.as_ref().filter(|lookup| {
                lookup.name_input() == input
                    && lookup.site().source.source_image() == key.source_image()
                    && lookup.site().offset == invocation.range.start()
            }) else {
                continue;
            };
            if !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                input,
                invocation.range,
            ) || invocation.resolved_command_reference.is_some()
                || !analysis.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == DiagCode::W123
                        && diagnostic.span == invocation.range
                        && diagnostic.unresolved_command().is_some_and(|subject| {
                            subject.name_input() == key
                                && subject
                                    .invocation()
                                    .original_static_command_lookup(key)
                                    .as_ref()
                                    == Some(lookup)
                        })
                })
            {
                continue;
            }
            let Some(package) = package_owned_by_original_input(input, &ownership) else {
                continue;
            };
            let selected = tcl_registry::native_package::NativePackageNameKey::from_native_units(
                package.as_bytes(),
                input.policy(),
            );
            if original_package_already_required(source, analysis, &selected) != Some(false)
                || selected.manifest_atom(analysis.body_lexer_config?) != Some(package.clone())
            {
                continue;
            }
            return Some(package);
        }
        // A missing Native producer cannot enter explicit Logical advice.
        let Some(offset) = invocation.lookup.offset(invocation.range) else {
            continue;
        };
        let Some(words) =
            tcl_compiler::registry_invocation::source_structure::original_logical_source_words_at(
                source, analysis, offset,
            )
        else {
            continue;
        };
        let Some(head) = words.first().and_then(diagnostic_context::source_literal) else {
            continue;
        };
        if !is_static_command_name(head) {
            continue;
        }
        let Some(package) = package_owned_by_logical_namespace(head, &ownership) else {
            continue;
        };
        if !head_is_unresolved(source, analysis, resolution, invocation, head)
            || original_source_package_already_named(source, analysis, &package) != Some(false)
            || !package_source_atom(&package)
        {
            continue;
        }
        return Some(package);
    }
    None
}

// Actual selected descriptor ownership supplies a package hint only. The
// independently retained input/absence/source/grammar gates remain above.
fn package_owned_by_original_input(
    input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    ownership: &tcl_registry::model::NamespacePackageOwnership,
) -> Option<String> {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};
    match input.policy().recipe() {
        protocol @ NativeNameProtocol::C(_) => {
            let slot = protocol
                .command_lookup_slot(NativeNameContext::root(), input.bytes())
                .ok()?;
            // Only an analytical presentation for descriptor ownership. It
            // cannot recreate the original slot or grant Native namespace use.
            let namespace = tcl_syntax::naming::native_namespace_full_name_bytes(&slot.namespace);
            let namespace = std::str::from_utf8(&namespace).ok()?;
            ownership
                .package_for_namespace(namespace)
                .map(str::to_owned)
        }
        protocol @ NativeNameProtocol::Jim084 => {
            // Retain Jim's actual flat key; no C namespace path is fabricated.
            let keys = protocol
                .jim_command_lookup_keys(NativeNameContext::root(), input.bytes())
                .ok()?;
            ownership
                .package_for_flat_key(keys.first()?.as_bytes())
                .map(str::to_owned)
        }
    }
}

fn package_owned_by_logical_namespace(
    head: &str,
    ownership: &tcl_registry::model::NamespacePackageOwnership,
) -> Option<String> {
    let written = tcl_syntax::naming::canonical_written_command(head);
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(&written);
    ownership.package_for_namespace(holder).map(str::to_owned)
}

fn original_package_already_required(
    source: &str,
    analysis: &AnalysisResult,
    selected: &tcl_registry::native_package::NativePackageNameKey,
) -> Option<bool> {
    let metadata = std::str::from_utf8(selected.bytes()).ok()?;
    metadata.is_ascii().then_some(())?;
    original_source_package_already_named(source, analysis, metadata)
}

/// Authentic selected package syntax, not line prefixes or signature labels.
pub(super) fn original_source_package_already_named(
    source: &str,
    analysis: &AnalysisResult,
    package: &str,
) -> Option<bool> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let census = crate::source_structure::SourcePackagePresenceCensus::capture(source, analysis)?;
    crate::source_structure::SourcePackagePresenceCensus::already_names(
        &census, source, analysis, package,
    )
}

fn package_source_atom(package: &str) -> bool {
    package.is_ascii()
        && !package.is_empty()
        && !package.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte.is_ascii_whitespace()
                || matches!(
                    byte,
                    b';' | b'$' | b'[' | b']' | b'{' | b'}' | b'"' | b'\\' | b'#'
                )
        })
}

/// `true` when `name` is a command name written literally in the source —
/// the only shape a package suggestion can be matched against.
///
/// A head built at run time (`$cmd`, `[pick]`, an `{*}`-expanded word) is
/// still recorded as an invocation, but its *name* is whatever the caller
/// wrote, not the command that will run.
fn is_static_command_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('$')
        && !name.contains('[')
        && !name.contains('{')
        && !name.contains(char::is_whitespace)
}

/// Match the first written qualifier to a catalogue package. This fixture
/// projection supplies no package availability or command identity.
#[cfg(test)]
pub(super) fn package_named_by_namespace(head: &str, catalogue: &[String]) -> Option<String> {
    let parts = tcl_syntax::naming::written_command_parts(head.as_bytes());
    let namespace = std::str::from_utf8(parts.qualifiers.first()?).ok()?;
    catalogue
        .iter()
        .find(|package| package.eq_ignore_ascii_case(namespace))
        .cloned()
}

/// Positioned absence from the shared source owner, followed by the same
/// whole-program declaration resolver used by definition navigation. Registry
/// descriptor names and external diagnostic prose cannot establish absence.
fn head_is_unresolved(
    source: &str,
    analysis: &AnalysisResult,
    resolution: crate::definition::CallResolution<'_>,
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    head: &str,
) -> bool {
    let Some(offset) = invocation.lookup.offset(invocation.range) else {
        return false;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return false;
    };
    if realm.diagnostic_slot_presence_at(offset).knowledge != tcl_core_types::Knowledge::Absent {
        return false;
    }
    let namespace = crate::definition::namespace_context_at(
        &analysis.global_scope,
        offset,
        &analysis.namespace_overrides,
    );
    crate::definition::resolve_called_proc(analysis, source, &namespace, head, offset, resolution)
        .is_none()
}
