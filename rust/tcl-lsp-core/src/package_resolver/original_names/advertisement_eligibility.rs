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

//! Negative-only static registration eligibility before full package analysis.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_compiler::signature_scan::scope::SignatureSourceNameKey;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage, Span, WordKind};
use tcl_registry::model::binding::PackageTransition;
use tcl_registry::state_transition::CommandBindingTransition;
use tcl_registry::{InvocationWord, InvocationWords, StateTransition};

/// A false answer excludes only this consumer's static advertisement shape.
/// It supplies no package inventory, original name, execution or analysis.
/// Uncertain source ownership and captured bindings keep the full path.
pub(super) fn may_advertise(content: &str, version: tcl_dialect::TclVersion) -> bool {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let profile =
        tcl_dialect::DialectProfile::find(version.dialect_name()).expect("C loader release");
    // Those directives modify the full analyser's supplied source context.
    // This narrower entry does not reconstruct that modified owner.
    let (commands, expressions) =
        tcl_compiler::analyser::utils::document_stub_declarations(content, None, profile.name);
    if !commands.is_empty()
        || !expressions.is_empty()
        || !tcl_compiler::analyser::utils::parse_supports_directives(content).is_empty()
        || !tcl_compiler::analyser::utils::parse_provides_directives(content).is_empty()
    {
        return true;
    }
    let Some(vectors) = original_vectors(content, config) else {
        return true;
    };
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
    let generation = crate::context_for_dialect_profile(profile);
    let loader_context =
        Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    let input = ResolvedAnalysisInput::new(profile, profile, loader_context, config);
    let mut queries = 64 * 1024;
    vectors
        .iter()
        .any(|original| vector_may_advertise(&input, original, version, &mut queries))
}

fn vector_may_advertise(
    input: &ResolvedAnalysisInput,
    original: &[NativeWord],
    version: tcl_dialect::TclVersion,
    remaining_queries: &mut usize,
) -> bool {
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let policy = dialect
        .authored_name_policy()
        .expect("C loader naming recipe");
    let config = input.lexer_config();
    let context = input.context_registry();
    let registry = context.commands();
    let keys = super::package_entry_word_keys(original, config, policy);
    let Some(head_key) = keys.first().and_then(Option::as_ref) else {
        return false;
    };
    let Some(head) = head_key.display() else {
        return true;
    };
    let arguments = super::package_entry_invocation_arguments(&keys);
    // Nominal metadata can only preserve full analysis, including prefix
    // aliases whose baked version is not a written invocation operand.
    if registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        )
        .resolved()
        .is_some_and(|schema| {
            schema.state_transitions().facts().iter().any(|fact| {
                matches!(
                    fact.transition,
                    StateTransition::CommandBinding(CommandBindingTransition::Alias { .. })
                )
            })
        })
    {
        return true;
    }
    if !keys
        .iter()
        .skip(1)
        .flatten()
        .filter_map(SignatureSourceNameKey::display)
        .any(|word| tcl_dialect::validate_version_for(word, version))
    {
        return false;
    }
    // The full consumer accepts only a Registry-backed canonical target.
    // Quantify the complete roster; no source lookup or nominal selection is
    // inferred when every possible target rejects the required output shape.
    for command in registry.command_names() {
        if *remaining_queries == 0 {
            return true;
        }
        *remaining_queries -= 1;
        let outcome = registry.resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(command), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let Some(schema) = outcome.resolved() else {
            continue;
        };
        if schema.state_transitions().facts().iter().any(|fact| {
            let StateTransition::Package(PackageTransition::Ifneeded {
                package,
                version: advertised,
                script: Some(script),
                script_provided: true,
            }) = &fact.transition
            else {
                return false;
            };
            super::package_entry_name_input(&keys, package).is_some()
                && super::package_entry_advertised_version(&keys, advertised, version).is_some()
                && super::package_entry_loader_word(original, script).is_some()
        }) {
            return true;
        }
    }
    false
}

/// Inspect a superset of contiguous source script regions. Data is deliberately
/// included; a false positive only preserves full analysis. Original words and
/// the complete source image/configuration remain the shared lexer's owners.
fn original_vectors(content: &str, config: LexerConfig) -> Option<Vec<Vec<NativeWord>>> {
    const MAX_SOURCE_BYTES: usize = 256 * 1024;
    const MAX_REGIONS: usize = 4096;
    const MAX_DEPTH: u32 = 32;
    if content.len() > MAX_SOURCE_BYTES {
        return None;
    }
    let image = SourceImage::document(content);
    let length = u32::try_from(image.len()).ok()?;
    let mut queue = VecDeque::from([(Span::new(0, length), 0)]);
    let mut visited = BTreeSet::new();
    let mut remaining_bytes = 4 * MAX_SOURCE_BYTES;
    let mut vectors = Vec::new();
    while let Some((region, depth)) = queue.pop_front() {
        if depth > MAX_DEPTH || visited.len() >= MAX_REGIONS {
            return None;
        }
        if !visited.insert((region.start(), region.end())) {
            continue;
        }
        remaining_bytes = remaining_bytes.checked_sub(region.as_range().len())?;
        let plan = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        for command in plan.commands {
            for word in &command.words {
                if word.group().expand || word.image() != &image || word.config() != config {
                    return None;
                }
                for part in word.executable_parts().all_parts() {
                    match part.part {
                        ExecutablePart::Command { body } => queue.push_back((body, depth + 1)),
                        ExecutablePart::ParseError(_) => return None,
                        _ => {}
                    }
                }
                if matches!(word.group().kind, WordKind::Braced | WordKind::Quoted) {
                    let body = word.content_span().ok()?;
                    // Cooked/materialised script bytes have a separate owner.
                    // This negative survey never substitutes them for source.
                    if tcl_syntax::word_rules::original_static_word_source_bytes(word)?.as_slice()
                        != image.bytes().get(body.as_range())?
                    {
                        return None;
                    }
                    queue.push_back((body, depth + 1));
                }
            }
            vectors.push(command.words);
        }
    }
    Some(vectors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_and_nested_advertisements_preserve_the_full_consumer() {
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "package ifneeded demo 1.0 {source demo.tcl}",
                "if {$condition} {package ifneeded demo 1.0 {source demo.tcl}}",
                "proc register {} {package ifneeded demo 1.0 {source demo.tcl}}",
                "set value [package ifneeded demo 1.0 {source demo.tcl}]",
                "set value $array([package ifneeded demo 1.0 {source demo.tcl}])",
                "interp alias {} register {} package ifneeded demo 1.0; register {source demo.tcl}",
                "set version 1.0; interp alias {} register {} package ifneeded demo $version; register {source demo.tcl}",
                "package ifneeded {*}{demo 1.0 {source demo.tcl}}",
                r"packa\ge ifneeded demo 1.0 {source demo.tcl}",
                r"package ifneeded demo 1\.0 {source demo.tcl}",
                r#"eval "package ifneeded demo 1.0 {source demo.tcl}\n""#,
                "package ifneeded demo 1.0 {source demo.tcl",
                "# tcl-lsp: supports tcl 8.6-9.0\npackage ifneeded demo $version {source demo.tcl}",
            ] {
                assert!(may_advertise(source, version), "{version:?}: {source}");
            }
        }
    }

    #[test]
    fn cooked_bare_body_matches_full_builder_without_losing_static_word_keys() {
        let cooked_body = r"eval package\ ifneeded\ demo\ 1.0\ \{source\ demo.tcl\}";
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "package ifneeded demo 1.0 {source demo.tcl}",
                r"packa\ge ifneeded demo 1.0 {source demo.tcl}",
                r"package ifneeded demo 1\.0 {source demo.tcl}",
            ] {
                assert!(may_advertise(source, version), "{version:?}: {source}");
                let entries = super::super::original_package_entries_full(
                    source,
                    std::path::Path::new("/library"),
                    std::path::Path::new("/library/pkgIndex.tcl"),
                    &|_| true,
                    &|_| Vec::new(),
                    version,
                );
                assert_ne!(
                    entries,
                    Vec::<super::super::OriginalPackageInfo>::new(),
                    "{version:?}: {source}"
                );
            }
            assert!(!may_advertise(cooked_body, version), "{version:?}");
            let entries = super::super::original_package_entries_full(
                cooked_body,
                std::path::Path::new("/library"),
                std::path::Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            assert_eq!(
                entries,
                Vec::<super::super::OriginalPackageInfo>::new(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn excluded_dynamic_advertisement_matches_the_full_original_builder() {
        let source = r"proc Loader {version dir} {
            if {[package vcompare [info tclversion] 8.4] < 0} {set suffix old} else {set suffix new}
            load [file join $dir $suffix]
            }
            set version 2.5
            set patchlevel 2.5.3
            package ifneeded BLT $patchlevel [list Loader $version $dir]
";
        for version in tcl_dialect::TclVersion::ALL {
            assert!(!may_advertise(source, version), "{version:?}");
            let positive = super::super::original_package_entries_full(
                "package ifneeded demo 1.0 {source demo.tcl}",
                std::path::Path::new("/library"),
                std::path::Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            assert_ne!(
                positive,
                Vec::<super::super::OriginalPackageInfo>::new(),
                "{version:?}: positive prerequisite"
            );
            let entries = super::super::original_package_entries_full(
                source,
                std::path::Path::new("/library"),
                std::path::Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            assert_eq!(
                entries,
                Vec::<super::super::OriginalPackageInfo>::new(),
                "{version:?}"
            );
        }
    }
}
