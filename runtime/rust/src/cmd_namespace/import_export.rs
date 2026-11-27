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

//! `namespace export`, `import` and `forget`, and the pattern words they share.

use super::set_list_bytes;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{error_code_list, Code, Command, Interp};
use crate::namespace::NsId;
use crate::obj::TclObj;

// export / import / forget

/// `namespace export ?-clear? ?pattern ...?` — query / append (or clear+set)
/// the current namespace's export patterns.
fn namespace_pattern(
    interp: &Interp,
    arguments: &OriginalArguments,
    index: usize,
    purpose: tcl_syntax::naming::NativeNamePurpose,
) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
    let original = arguments.bytes(index)?;
    let policy = interp.name_policy_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace pattern issuer"),
    )?;
    policy
        .recipe()
        .namespace_pattern_input(&original, purpose)
        .map(|projection| projection.selected().to_vec())
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace pattern purpose")
                .into()
        })
}

pub(super) fn ns_export(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    let cur = interp.current_ns();
    if argv.len() == 2 {
        let patterns = interp.namespaces().exports(cur).to_vec();
        set_list_bytes(interp, &patterns);
        return Code::Ok;
    }
    let mut first = match namespace_pattern(
        interp,
        arguments,
        2,
        tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
    ) {
        Ok(pattern) => Some(pattern),
        Err(error) => return interp.report_cmd_error(error),
    };
    let clear = first.as_deref() == Some(b"-clear");
    let start = if clear {
        interp.namespaces_mut().clear_exports(cur);
        first = None;
        3
    } else {
        2
    };
    // Original C drivers commit clear and each accepted pattern before the next
    // String getter. A failed pattern leaves earlier effects and later originals untouched.
    for index in start..argv.len() {
        let pattern = match first.take().map(Ok).unwrap_or_else(|| {
            namespace_pattern(
                interp,
                arguments,
                index,
                tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
            )
        }) {
            Ok(pattern) => pattern,
            Err(error) => return interp.report_cmd_error(error),
        };
        if tcl_syntax::naming::is_qualified(&pattern) {
            let mut message = b"invalid export pattern \"".to_vec();
            message.extend_from_slice(&pattern);
            message.extend_from_slice(b"\": pattern can't specify a namespace");
            return interp.set_error(&message);
        }
        interp.namespaces_mut().export(cur, &pattern);
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace import ?-force? pattern ?pattern ...?` — install transparent
/// redirects in the current ns for the exported commands matching each pattern.
pub(super) fn ns_import(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_native_model::native_jim_lookup::NativeJimNamespaceHelperKind::Import,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
    if argv.len() > 2 {
        interp.invalidate_namespace_structure_guards();
    }
    let dest = interp.current_ns();
    // The introspection form is `objc == 1` — literally no words after
    // `import` — and C tests it *before* looking at `-force`. So a bare
    // `namespace import -force` is NOT a query: it takes the flag, imports
    // nothing, and yields "".
    if argv.len() == 2 {
        let names = interp.imported_command_tails(dest);
        set_list_bytes(interp, &names);
        return Code::Ok;
    }
    let mut first = match namespace_pattern(
        interp,
        arguments,
        2,
        tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
    ) {
        Ok(pattern) => Some(pattern),
        Err(error) => return interp.report_cmd_error(error),
    };
    let force = first.as_deref() == Some(b"-force");
    let start = if force {
        first = None;
        3
    } else {
        2
    };
    for index in start..argv.len() {
        let pattern = match first.take().map(Ok).unwrap_or_else(|| {
            namespace_pattern(
                interp,
                arguments,
                index,
                tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
            )
        }) {
            Ok(pattern) => pattern,
            Err(error) => return interp.report_cmd_error(error),
        };
        let pat = pattern.as_slice();
        let destination = tcl_runtime_api::Namespaces::current(interp);
        let validated = match tcl_cmd_core::namespace::import_pattern(interp, destination, pat) {
            Ok(validated) => validated,
            Err(problem @ tcl_cmd_core::namespace::ImportPatternError::Unknown(_)) => {
                // A namespace being deleted is absent from the public tree but
                // its command table remains token-addressable during delete
                // callbacks. Tcl_Import can therefore import a callback-created
                // command from that dying table even though `namespace exists`
                // is false. Keep the shared qualifier/tail parser and only
                // substitute this lifecycle-specific namespace lookup.
                let qualifier = match tcl_cmd_core::namespace::qualifier(pat) {
                    tcl_cmd_core::namespace::Qualifier::Absolute(prefix)
                    | tcl_cmd_core::namespace::Qualifier::Relative(prefix) => prefix,
                    tcl_cmd_core::namespace::Qualifier::Unqualified => {
                        return interp.set_error(&problem.message());
                    }
                };
                let Some(source) = interp.namespaces().dying_namespace(dest, qualifier) else {
                    return interp.set_error(&problem.message());
                };
                tcl_cmd_core::namespace::ImportPattern {
                    source: tcl_runtime_api::NsId(source as u32),
                    tail: tcl_cmd_core::namespace::tail(pat).to_vec(),
                }
            }
            Err(problem) => return interp.set_error(&problem.message()),
        };
        let src_ns = validated.source.0 as usize;
        let tail_pat = validated.tail;
        // Collect the matching, exported source commands first (borrow ends).
        let mut to_import: Vec<Vec<u8>> = Vec::new();
        for name in interp.visible_command_names_in(src_ns) {
            if glob_match_bytes(&tail_pat, &name) && interp.namespaces().is_exported(src_ns, &name)
            {
                to_import.push(name);
            }
        }
        for simple in to_import {
            let Some((source, source_generation, ensemble)) =
                interp.import_metadata_in(src_ns, &simple)
            else {
                continue;
            };
            // Without `-force`, re-importing the *same* command from the *same*
            // source is a silent no-op (C's `TclGetOriginalCommand` reimport
            // check, tclNamesp.c) — common when a file and its sourced helper
            // both import `::tcltest::*`. `-force` deliberately replaces even
            // that same-origin import with a fresh command token.
            let existing_import = match interp.namespaces().command_in(dest, &simple) {
                Some(Command::Imported {
                    source_generation, ..
                }) => Some(source_generation),
                _ => None,
            };
            if !force && existing_import == Some(source_generation) {
                continue;
            }
            // Reject clobbering an existing (different) command unless -force.
            if !force && (existing_import.is_some() || dest_has_own(interp, dest, &simple)) {
                let mut m = b"can't import command \"".to_vec();
                m.extend_from_slice(&simple);
                m.extend_from_slice(b"\": already exists");
                return interp.error_with_code(&m, b"TCL IMPORT OVERWRITE");
            }
            // Follow immediate import origins before mutating the destination.
            // If the source chain already reaches the command being replaced,
            // this new edge would close Tcl's ImportRef graph into a cycle.
            let destination_generation = interp.namespaces().command_generation(dest, &simple);
            if destination_generation
                .is_some_and(|needle| interp.import_chain_contains(source_generation, needle))
            {
                let destination_fqn = interp.namespaces().command_fqn_at(dest, &simple);
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(pat);
                message.extend_from_slice(b"\" would create a loop containing command \"");
                message.extend_from_slice(&destination_fqn);
                message.push(b'"');
                let error_code = error_code_list(&[b"TCL", b"IMPORT", b"LOOP"]);
                return interp.error_with_code(&message, &error_code);
            }
            interp.bind_command_replacement(
                dest,
                &simple,
                Command::Imported {
                    source,
                    source_generation,
                    ensemble,
                    identity: std::rc::Rc::new(crate::interp::ImportToken),
                },
            );
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace forget ?pattern ...?` — remove matching imported bindings.
pub(super) fn ns_forget(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    use tcl_syntax::naming::{NativeNameContext, NativeNamespacePatternSource};
    use tcl_syntax::value::ValueError;

    interp.invalidate_namespace_structure_guards();
    let dest = interp.current_ns();
    for index in 2..argv.len() {
        let original = match arguments.bytes(index) {
            Ok(original) => original,
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(policy) = interp.name_policy_protocol() else {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("namespace forget pattern issuer").into(),
            );
        };
        let path = interp.namespaces().native_context_path(dest);
        let Some(path) = path else {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("namespace forget current namespace").into(),
            );
        };
        let parts = match policy.recipe().namespace_pattern_parts(
            NativeNameContext::new(&path),
            &original,
            tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern,
        ) {
            Ok(parts) => parts,
            Err(_) => {
                return interp.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("namespace forget pattern purpose")
                        .into(),
                );
            }
        };
        let source = match &parts.source {
            Some(NativeNamespacePatternSource::C(path)) => {
                let source = interp.namespaces().namespace_at_path(path);
                let Some(source) = source else {
                    let mut message = b"unknown namespace in namespace forget pattern \"".to_vec();
                    message.extend_from_slice(parts.selected.as_bytes());
                    message.push(b'"');
                    if interp.runtime_version() >= tcl_dialect::TclVersion::V8_5 {
                        let error_code = error_code_list(&[
                            b"TCL",
                            b"LOOKUP",
                            b"NAMESPACE",
                            parts.selected.as_bytes(),
                        ]);
                        return interp.error_with_code(&message, &error_code);
                    }
                    return interp.set_error(&message);
                };
                Some(source)
            }
            None => None,
            Some(NativeNamespacePatternSource::Jim(_)) => {
                return interp.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("Jim namespace forget pattern").into(),
                );
            }
        };
        let victims = interp.namespaces().imported_forget_bindings(
            dest,
            source,
            parts.tail.as_bytes(),
            tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy),
        );
        let victims = match victims {
            Ok(victims) => victims,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        for (simple, generation) in victims {
            if interp.uses_original_c_command_retirement() {
                interp.finish_original_command_deletion(generation, true);
            } else if interp.namespaces().command_generation(dest, &simple) == Some(generation) {
                interp.namespaces_mut().remove_in(dest, &simple);
            }
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Does the dest namespace hold a command of this name?
fn dest_has_own(interp: &Interp, dest: NsId, simple: &[u8]) -> bool {
    interp.namespaces().command_names(dest).contains(&simple)
}

/// `namespace qualifiers` text op: everything before the last `::` separator,
/// with the whole trailing colon-run trimmed (`foo:::` → `foo`, `:::::` → ``);
/// empty if unqualified.
/// `string match` over bytes (UTF-8 → shared [`tcl_syntax::glob`]; a non-UTF-8
/// pattern or text can only match byte-identically, handled by the equality
/// fallback).
fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    tcl_syntax::glob::string_match_bytes(pattern, text)
}
