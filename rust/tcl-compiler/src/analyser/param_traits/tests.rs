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

use super::*;
use tcl_registry::CommandRegistry;

mod body_scan;
mod caller_frame;
mod command_traits;
mod original_parameter_roles;
mod resolved_heads;

fn assert_trait(
    traits: &HashMap<String, HashSet<ProcArgTrait>>,
    param: &str,
    expected: ProcArgTrait,
) {
    let set = traits
        .get(param)
        .unwrap_or_else(|| panic!("no traits for {param}"));
    assert!(
        set.contains(&expected),
        "{param}: expected {expected:?}, got {set:?}",
    );
}

/// Retain an uncalled declaration with its exact formals and body source.
/// Successful own-body advice requires this receipt; no caller is invented.
fn infer(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_owned(params, body, false)
}

fn infer_owned(params: &[&str], body: &str, deep: bool) -> HashMap<String, HashSet<ProcArgTrait>> {
    with_receipt(params, body, |env| {
        if deep {
            infer_param_traits_deep(params, body, env)
        } else {
            infer_param_traits(params, body, env)
        }
    })
}

/// Run `scan` over the receipt [`infer`] retains for `body` declared with
/// exactly `params` as its formals.
fn with_receipt<R>(params: &[&str], body: &str, scan: impl FnOnce(TraitScanEnv<'_>) -> R) -> R {
    let registry = CommandRegistry::build_default();
    let formals = tcl_syntax::list::join_list(params);
    let source = tcl_syntax::list::join_list(["proc", "__trait_probe", &formals, body]);
    let identities = crate::realm::document_realm_bindings(
        &source,
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        &registry,
    );
    let command = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default())
        .pop()
        .unwrap();
    let script = identities
        .executed_script_for_word(command.argv.last().unwrap().span)
        .unwrap();
    assert_eq!(script.text.try_text().unwrap(), body);
    scan(TraitScanEnv {
        surface: DocumentCommandSurface::new(&registry, None),
        config: LexerConfig::default(),
        identities: &identities,
        executed_source: Some(script),
        original_logical_body: None,
    })
}

/// A scan environment over `registry` with no document declarations and
/// no document binding facts — what a bare-body unit test scans under.
fn env_for(registry: &CommandRegistry, config: LexerConfig) -> TraitScanEnv<'_> {
    TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, None),
        config,
        executed_source: None,
        original_logical_body: None,
        identities: if registry.profile().is_some() {
            crate::realm::CommandBindingRealm::none()
        } else {
            static ENTRY: std::sync::OnceLock<crate::realm::CommandBindingRealm> =
                std::sync::OnceLock::new();
            ENTRY.get_or_init(|| {
                crate::realm::document_realm_bindings(
                    "",
                    tcl_dialect::DialectProfile::find("tcl9.0").expect("test native core"),
                    tcl_registry::default_registry(),
                )
            })
        },
    }
}

/// [`env_for`] with the document's own declarations attached.
fn env_with_declarations<'a>(
    registry: &'a CommandRegistry,
    declared: &'a tcl_registry::model::DeclaredSurface,
) -> TraitScanEnv<'a> {
    TraitScanEnv {
        surface: DocumentCommandSurface::new(registry, Some(declared)),
        ..env_for(registry, LexerConfig::default())
    }
}

// document-declaration integration
//
// These tests pin the contract that a non-empty
// [`tcl_registry::model::DeclaredSurface`] threaded through
// `infer_param_traits` / `infer_param_traits_deep` lets user-declared
// `# tcl-lsp: stub` commands participate in role-driven trait
// inference alongside the catalogue.

use tcl_dialect::model::Provenance;
use tcl_registry::model::{DeclaredArgument, DeclaredCommand, DeclaredSurface};

fn make_declarations(commands: Vec<DeclaredCommand>) -> DeclaredSurface {
    let mut surface = DeclaredSurface::new();
    for command in commands {
        surface.declare(command);
    }
    surface
}

fn stub_sig(name: &str, args: &[(&str, ArgRole)]) -> DeclaredCommand {
    DeclaredCommand::new(
        name.to_string(),
        args.iter()
            .map(|(n, r)| DeclaredArgument {
                name: (*n).to_string(),
                role: *r,
                optional: false,
            })
            .collect(),
        Provenance::Document,
    )
}

/// Trait inference must resolve a command head's *effective identity*, not
/// its written spelling.
///
/// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} run
/// {} eval` makes `run $body` evaluate `$body`; `rename eval run` moves it
/// and leaves `eval` gone; a top-level `proc eval …` takes the name over,
/// so `eval $body` no longer evaluates anything.
///
/// The environment carries the *document's* facts while the scan reads a
/// *proc body*, which is exactly the shape the analyser threads.
fn realm_for_body(
    prelude: &str,
    body: &str,
) -> (crate::realm::CommandBindingRealm, tcl_lexer::Span) {
    let source = format!("{prelude}proc __trait_probe {{body v}} {{{body}}}");
    let registry = CommandRegistry::build_default();
    let identities = crate::realm::document_realm_bindings(
        &source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    let segments = segment_commands_with_offset_and_config(&source, 0, LexerConfig::default());
    let body_span = segments.last().unwrap().argv.last().unwrap().span;
    (identities, body_span)
}
