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

//! Internal scan-state types used by the `signature_scan` walker.
//!
//! These are crate-private — they accumulate intermediate results
//! during the walk and are consumed by the second-pass factory
//! resolver (see [`super::factory::resolve_factory_defs`]) before
//! the public [`SignatureScanResult`] is returned.
//!
//! - [`ScanCtx`] is threaded as `&mut` through the walker and every
//!   handler; it owns the public `result` accumulator plus the two
//!   first-pass vectors `candidates` and `proc_bodies`.
//! - [`FactoryCandidate`] records each four-token call (`HEAD NAME
//!   ARGS BODY`) the walker spotted, deferring binding to a real
//!   factory until pass two.
//! - [`ProcBodyInfo`] records each proc body's text + params +
//!   home namespace so the factory detector can spot the canonical
//!   `proc $name $args $body` shape.
//! - [`FACTORY_SKIP_HEADS`] is the negative list of built-in heads
//!   that incidentally take the same four-token shape but are
//!   definitely not factory wrappers.
//!
//! [`SignatureScanResult`]: super::types::SignatureScanResult

use tcl_lexer::Token;

use super::scope::{SignatureNamespaceScope, SignatureSourceCommand};
use super::types::{SignatureProc, SignatureScanResult};

/// A factory-wrapper call captured during the first scan pass.
///
/// `signature_scan` recognises tcllib-style factory wrappers (e.g.
/// `DEFC name args body`) by their canonical four-token shape and
/// defers binding to a real factory until after the full source has
/// been scanned. `FactoryCandidate` records the call-site data the
/// second pass needs to attribute the synthetic proc to the right
/// namespace.
#[derive(Debug, Clone)]
pub(super) struct FactoryCandidate {
    /// The command head as written at the call site (e.g. `"DEFC"`,
    /// `"::foo::DEF"`).
    pub(super) head: String,
    /// The proc-name argument as written.
    pub(super) name: String,
    /// Token of the name argument (used for the synthetic proc's
    /// `name_range`).
    pub(super) name_tok: Token,
    /// Token of the body argument (used for the synthetic proc's
    /// `body_range`).
    pub(super) body_tok: Token,
    /// Effective rooted constructed namespace key at the call site.
    pub(super) ns_prefix: String,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
}

/// First-pass record of a proc body, used to identify factory
/// wrappers by their `proc $p1 $p2 $p3` body shape.
///
/// The wrapper's home namespace is the namespace any factory-emitted
/// procs end up in at runtime — `proc $name …` executed inside a
/// proc creates the command in the caller's current namespace, which
/// for a factory-wrapper INIT path is unambiguously the wrapper's
/// own home.
#[derive(Debug, Clone)]
pub(super) struct ProcBodyInfo {
    /// Fully-qualified proc name with leading `::`.
    pub(super) qname: String,
    /// Parameter names (in declaration order) — the factory body
    /// detector matches these against the `proc $a $b $c` body
    /// shape.
    pub(super) params: Vec<String>,
    /// Verbatim proc body text.
    pub(super) body_text: String,
    /// Namespace any synthetic procs created by this wrapper live
    /// in, as a rooted constructed namespace key.
    pub(super) ns_prefix: String,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
    pub(super) source_name: Option<SignatureSourceCommand>,
}

/// Mutable scan context threaded through the walker.
#[derive(Debug, Default)]
pub(super) struct ScanCtx<'r> {
    /// Public result accumulator.
    pub(super) result: SignatureScanResult,
    /// Factory-call candidates collected during pass 1.
    pub(super) candidates: Vec<FactoryCandidate>,
    /// Proc-body records collected during pass 1, used to identify
    /// real factory wrappers in pass 2.
    pub(super) proc_bodies: Vec<ProcBodyInfo>,
    /// Command heads that match the factory-wrapper token shape but
    /// are not factories — sourced from the registry's
    /// `NOT_PROC_FACTORY` trait plus [`FACTORY_SKIP_NONCOMMAND_HEADS`],
    /// built once by `extract_signatures`.  Empty in `Default`
    /// (used only by focused unit tests).
    pub(super) skip_heads: std::collections::HashSet<String>,
    /// The command registry: drives the walker's definer dispatch (class
    /// definers via their `definition_body` grammar family, procedure
    /// definers via `Traits::DEFINES_PROCEDURE`) and resolves
    /// `ArgRole::CommandPrefix` callback positions + arities so
    /// background-scanned files record callback heads (`lsort -command cb`)
    /// as command invocations.  `None` in `Default` (focused unit tests
    /// that bypass registry dispatch).
    pub(super) registry: Option<&'r tcl_registry::CommandRegistry>,
    /// The document dialect's word-value rules — how a braced word's
    /// `\<newline>` folds and how list text divides — used by every
    /// re-parse of a scanned word (a proc's parameter list, an OO member's).
    /// Threaded from the scan's entry point so the scanner reads a word the
    /// way the document's own runtime reads it, rather than re-deriving C
    /// Tcl's answer at each site.
    pub(super) rules: tcl_syntax::word_rules::WordValueRules,
    /// The document dialect's lexer config — every segmentation the walk
    /// performs (the top-level stream, a recursed body, a factory-wrapper
    /// body) reads the source under this grammar rather than the default
    /// one.  Threaded from the scan's entry point, which derives it from
    /// the registry's own profile.
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) namespace_scope: Option<SignatureNamespaceScope>,
    pub(super) ambiguous_proc_names: std::collections::HashSet<String>,
}

impl ScanCtx<'_> {
    /// Pure source assistance; the policy never supplies a runtime lookup receipt.
    pub(super) fn name_policy(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        self.registry?.profile().and_then(|profile| {
            tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
        })
    }

    pub(super) fn current_namespace(&self, compatibility: &str) -> Option<SignatureNamespaceScope> {
        self.namespace_scope.clone().or_else(|| {
            let root = SignatureNamespaceScope::root(self.name_policy());
            root.child(compatibility, self.name_policy())
        })
    }

    pub(super) fn procedure_name_in_context(
        &self,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<(
        String,
        String,
        SignatureNamespaceScope,
        Option<SignatureSourceCommand>,
    )> {
        let Some(policy) = self.name_policy() else {
            let qualified = crate::naming::qualify(&namespace.display()?, written);
            let (holder, simple) = crate::naming::key_holder_and_tail(&qualified);
            return Some((
                qualified.clone(),
                simple.to_owned(),
                SignatureNamespaceScope::Symbolic(holder.to_owned()),
                None,
            ));
        };
        let recipe = policy.recipe();
        let context = namespace.context()?;
        if let tcl_syntax::naming::NativeNameProtocol::C(version) = recipe {
            let selected = recipe
                .command_lookup_slot(context, written.as_bytes())
                .ok()?;
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                selected.namespace.is_root(),
                selected.simple.as_bytes(),
            )?
            .ok()?;
        }
        let slot = recipe
            .command_publication_slot(context, written.as_bytes())
            .ok()?;
        let qualified = match recipe {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(&slot)).ok()?
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                let reported = recipe
                    .jim_namespace_canonical_input(context, written.as_bytes())
                    .ok()?;
                format!("::{}", std::str::from_utf8(reported.selected()).ok()?)
            }
        };
        let source_name = SignatureSourceCommand::new(policy, slot);
        let body_scope = source_name.body_scope()?;
        let simple = source_name.simple_name()?;
        Some((qualified, simple, body_scope, Some(source_name)))
    }

    pub(super) fn publication_name(
        &self,
        namespace: &SignatureNamespaceScope,
        written: &str,
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Option<(String, Option<SignatureSourceCommand>)> {
        let Some(policy) = self.name_policy() else {
            return Some((crate::naming::qualify(&namespace.display()?, written), None));
        };
        let recipe = policy.recipe();
        let context = namespace.context()?;
        let slot = match purpose {
            tcl_syntax::naming::NativeNamePurpose::CommandPublication => {
                recipe.command_publication_slot(context, written.as_bytes())
            }
            tcl_syntax::naming::NativeNamePurpose::RenameDestination => {
                recipe.rename_destination_slot(context, written.as_bytes())
            }
            tcl_syntax::naming::NativeNamePurpose::AliasPublication => {
                recipe.alias_publication_slot(context, written.as_bytes())
            }
            _ => return None,
        }
        .ok()?;
        let reported = if recipe.is_jim084()
            && purpose != tcl_syntax::naming::NativeNamePurpose::AliasPublication
        {
            let reported = recipe
                .jim_namespace_canonical_input(context, written.as_bytes())
                .ok()?;
            format!("::{}", std::str::from_utf8(reported.selected()).ok()?)
        } else {
            String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(&slot)).ok()?
        };
        Some((reported, Some(SignatureSourceCommand::new(policy, slot))))
    }

    pub(super) fn namespace_context(
        &self,
        namespace: &str,
        written: &str,
    ) -> Option<SignatureNamespaceScope> {
        self.current_namespace(namespace)?
            .child(written, self.name_policy())
    }

    pub(super) fn record_proc(&mut self, declaration: SignatureProc) {
        let name = declaration.qualified_name.clone();
        if self
            .result
            .procs
            .get(&name)
            .is_some_and(|previous| previous.source_name != declaration.source_name)
        {
            self.result.procs.remove(&name);
            self.ambiguous_proc_names.insert(name.clone());
        }
        if !self.ambiguous_proc_names.contains(&name) {
            self.result.procs.insert(name, declaration.clone());
        }
        self.result.procedure_declarations.push(declaration);
    }

    #[cfg(test)]
    pub(super) fn command_keys(&self, namespace: &str, written: &str) -> Vec<String> {
        let local = crate::naming::qualify(namespace, written);
        let global = crate::naming::qualify("::", written);
        if local == global || written.starts_with("::") {
            vec![local]
        } else {
            vec![local, global]
        }
    }
}

/// Project source declarations with an explicitly authored policy. The returned
/// analytical key carries no entered command identity or execution authority.
pub(super) fn authored_publication_key(
    namespace: &str,
    written: &str,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    purpose: tcl_syntax::naming::NativeNamePurpose,
) -> Option<String> {
    use tcl_syntax::naming::{NativeNameContext, NativeNamePurpose};
    let Some(policy) = policy else {
        return Some(crate::naming::qualify(namespace, written));
    };
    let path =
        tcl_core_types::ByteNamespacePath::from_segments(crate::naming::key_segments(namespace));
    let context = NativeNameContext::with_jim_namespace(
        &path,
        namespace.strip_prefix("::").unwrap_or(namespace).as_bytes(),
    );
    let recipe = policy.recipe();
    let slot = match purpose {
        NativeNamePurpose::RenameDestination => {
            recipe.rename_destination_slot(context, written.as_bytes())
        }
        NativeNamePurpose::AliasPublication => {
            recipe.alias_publication_slot(context, written.as_bytes())
        }
        _ => return None,
    }
    .ok()?;
    String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(&slot)).ok()
}

/// Factory-skip heads that are **not** registered commands and so
/// cannot carry the registry's `NOT_PROC_FACTORY` trait: the `TclOO`
/// definition keywords `method` / `classmethod` and the
/// (unregistered) itcl class-definition heads.  `extract_signatures`
/// unions these with the registry-stamped heads for factory exclusion.
pub(super) const FACTORY_SKIP_NONCOMMAND_HEADS: &[&str] =
    &["method", "classmethod", "itcl::class", "::itcl::class"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noncommand_skip_heads_are_the_unregistered_four() {
        assert_eq!(FACTORY_SKIP_NONCOMMAND_HEADS.len(), 4);
        for head in ["method", "classmethod", "itcl::class", "::itcl::class"] {
            assert!(FACTORY_SKIP_NONCOMMAND_HEADS.contains(&head));
        }
    }

    #[test]
    fn default_ctx_is_empty() {
        let ctx = ScanCtx::default();
        assert_eq!(ctx.candidates.len(), 0);
        assert_eq!(ctx.proc_bodies.len(), 0);
        assert_eq!(ctx.result.procs.len(), 0);
        assert_eq!(
            ctx.result.command_invocations,
            [] as [crate::signature_scan::types::SignatureCommandInvocation; 0]
        );
    }
}
