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

//! Warnings on what a definition names: a shadowed built-in (W113), a misplaced `args` (W218) and names that cannot be addressed or run (W314, W315).

use super::super::state::Analyser;
use super::qualify;
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token};

/// Tcl *library* procedures (defined in init.tcl / auto.tcl / history.tcl /
/// package.tcl / word.tcl) that are script-defined and documented as
/// user-replaceable — redefining one is the supported overlay idiom, not
/// shadowing a C built-in.  Genuine C commands that are not byte-compiled
/// but still dangerous to redefine (`clock`, `after`, `socket`, `glob`) are
/// deliberately excluded — they keep firing W113.
/// Memoised set of the redefinable Tcl library procs (`unknown`,
/// `auto_*`, `pkg_*`, `tclLog`, `tcl_findLibrary`, the word-boundary
/// helpers, …), sourced from the registry's
/// [`Traits::OVERRIDABLE_LIBRARY_PROC`] trait. Redefining one of these
/// must not fire W113. Cached once; the set is dialect-agnostic core Tcl.
fn overridable_library_procs() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        tcl_registry::CommandRegistry::build_default()
            .commands_with_trait(tcl_registry::Traits::OVERRIDABLE_LIBRARY_PROC)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

impl Analyser {
    /// **W113** — a `proc` name shadows a built-in command.
    ///
    /// The check runs against both the unqualified `raw_name` and the
    /// fully-qualified form, with the leading `::` trimmed (the registry
    /// indexes by bare command name).  The inner borrow on
    /// `self.builtin_command_names()` is dropped before the diagnostic push so
    /// `self.result` is free to mutate.
    ///
    /// Only *core global* built-ins are shadow-worthy (unqualified `set` /
    /// `dict` / …).  A namespace-qualified match (`::base64::encode`,
    /// `::snit::type`) is a library/package command living in its own namespace
    /// — its own definition or a deliberate override, not shadowing a built-in.
    /// And the overridable Tcl *library* procedures (`unknown`, `history`,
    /// `auto_*`, …) are script-defined, documented user-replaceable overlays.
    /// A third-party/package command (`argparse`, a tcllib/itcl/ticklecharts
    /// proc, …) is excluded too: `registry.command_names()`
    /// is the full package-inclusive command universe, not the set of names a
    /// bare Tcl interpreter actually starts with, so a proc named after a
    /// registered package command (e.g. `proc ::argparse {args} {...}`
    /// defining the `argparse` package's own entry point) is not shadowing
    /// anything until that package is actually loaded — and even then it is
    /// the package's own definition, not a redefinition of a core built-in.
    /// The gate reads `CommandSpec::owning_package` (`required_package` or
    /// its `tcllib_package` alias) rather than a hardcoded package-name list,
    /// so it stays correct as new package specs are added. A package a
    /// profile ships **ambient** (an F5 command pack, an EDA vendor tool
    /// surface) is the exception: that profile's real, always-present command
    /// surface, so it keeps firing W113 like any other core built-in (e.g.
    /// iRules' `pool`, which — being ambient and never `required_package`-gated
    /// in the first place — is untouched by this filter).
    pub(super) fn emit_w113_proc_shadows_builtin(
        &mut self,
        raw_name: &str,
        qualified: &str,
        name_span: Span,
        source_name: Option<&crate::signature_scan::scope::SignatureSourceCommand>,
    ) {
        // Original structured publication geometry takes precedence over
        // reporting names, including literal colons and isolated interpreters.
        let original_global = match source_name {
            Some(name) if !name.slot().namespace.is_root() => return,
            Some(name) => {
                let Ok(simple) = name.slot().simple.try_utf8() else {
                    return;
                };
                Some(simple)
            }
            None => None,
        };
        let rooted_written = qualify("::", raw_name);
        let normalised_proc = tcl_syntax::naming::unroot_rooted_key(&rooted_written)
            .unwrap_or(&rooted_written)
            .to_owned();
        let normalised_qual = tcl_syntax::naming::unroot_rooted_key(qualified)
            .unwrap_or(qualified)
            .to_owned();
        let shadow_name: Option<String> = {
            let builtins = self.builtin_command_names();
            // A `tcl::mathop` operator (`%`, `+`, `eq`, …) carries a bare-name
            // registry entry only so an `import`ed / `namespace path`-reachable
            // call resolves; the command itself lives in `::tcl::mathop`, not the
            // global namespace, so `proc %` shadows nothing reachable. Detect it
            // by its qualified spelling and treat it like the other namespaced
            // (non-core-global) commands the check already skips.
            let is_mathop =
                |n: &str| builtins.contains(&crate::naming::unrooted_member_key("tcl::mathop", n));
            if let Some(simple) = original_global {
                (builtins.contains(simple) && !is_mathop(simple)).then(|| simple.to_owned())
            } else if builtins.contains(&normalised_proc) && !is_mathop(&normalised_proc) {
                Some(normalised_proc.clone())
            } else if builtins.contains(&normalised_qual) && !is_mathop(&normalised_qual) {
                Some(normalised_qual.clone())
            } else {
                None
            }
        };
        let shadow_name = shadow_name.filter(|name| {
            !tcl_syntax::naming::is_qualified(name.as_bytes())
                && !overridable_library_procs().contains(name.as_str())
                && !self.is_package_gated_non_ambient(name)
        });
        if shadow_name.is_some() {
            // The permissive fallback profile means "no specific dialect" —
            // no parenthetical label.
            let dialect_label = if self.profile.is_fallback() {
                String::new()
            } else {
                format!(" ({})", self.dialect())
            };
            let message = format!("Procedure '{raw_name}' shadows built-in command{dialect_label}");
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W113,
                    name_span,
                    message,
                    super::super::types::Severity::Warning,
                ));
        }
    }

    /// Whether `name` (a bare or fully-qualified command name already known
    /// to be in `registry.command_names()`) resolves to a `CommandSpec` gated
    /// behind a `required_package` / `tcllib_package` (`CommandSpec::
    /// owning_package`) that this profile does **not** ship ambiently.
    ///
    /// Data-driven, not a hardcoded package-name list: the answer comes
    /// straight from the resolved spec's package attribution and the
    /// registry's own `is_ambient_package` query (the same query
    /// `emit_missing_package_require_diagnostics` / W120 already uses for the
    /// converse fact), so a new package-gated spec is covered automatically —
    /// no per-package entry to remember to add here. The registry is asked
    /// rather than the profile so a pack's `ambient_package` row counts too.
    fn is_package_gated_non_ambient(&self, name: &str) -> bool {
        // Resolution reads the un-overlaid generation's store (as the old
        // `registry_for_profile` call did); the ambient answer reads this
        // walk's own — possibly pack-overlaid — context, the same split
        // W120 uses.
        let base = crate::environment_ingress::context_for_profile(self.profile);
        base.context()
            .resolve_spec(base.commands(), name)
            .and_then(tcl_registry::CommandSpec::owning_package)
            .is_some_and(|pkg| !self.analysis_context().context().ambient_package(pkg))
    }

    /// **W314.** The definition's name has no absolute (fully-qualified)
    /// written form.  A written colon run of two or more is a
    /// namespace separator (the whole run collapses, `TclGetNamespaceForQualName`,
    /// invariant C 8.4→9.1), so an **all-colon simple name** — only `:` is
    /// writable — can never be spelled absolutely: `namespace which :` renders
    /// `:::`, which parses back to the global `{}` command, ensembles cannot
    /// dispatch an exported `:`, and `interp alias` / callback qualification
    /// break the same way.  The command remains reachable by *relative*
    /// lookup only.  (tclsh 8.6/9.0-pinned.)
    pub(in super::super) fn emit_w314_no_absolute_name(&mut self, raw_name: &str, name_span: Span) {
        // The written simple name is the last colon-run-delimited segment.  A
        // name *ending* in a run names the `{}` command, which IS addressable
        // (`::x::`); an all-colon segment can only be a lone `:` (a run of ≥2
        // is consumed as the separator).
        let unaddressable = !tcl_syntax::naming::ends_with_separator(raw_name.as_bytes())
            && crate::naming::qualifier_segments(raw_name.as_bytes())
                .last()
                .is_some_and(|seg| *seg == b":");
        if !unaddressable {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W314,
                name_span,
                format!(
                    "'{raw_name}' has no absolute (fully-qualified) name — a written colon run \
                 is a namespace separator, so this definition is reachable only by \
                 unqualified/relative lookup ([namespace which] output will not resolve, \
                 and ensembles cannot dispatch it)"
                ),
                super::super::types::Severity::Warning,
            ));
    }

    /// Logical declaration compatibility for class definition-abort reports.
    /// The source member tables retain authored order, without a selected Native
    /// definition worker or closed current own-method table. Every candidate is
    /// consumed; only positively retained Logical input permits reporting advice.
    pub(in super::super) fn emit_w315_definition_cannot_run(
        &mut self,
        class: &mut super::super::types::ClassDef,
    ) {
        let aborts = core::mem::take(&mut class.definition_aborts);
        // naming.diagnostics.original-definition-abort-admission
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
        if !self.result.allows_retained_logical_declaration_advice() || class.via_define {
            // A missing Logical admission or a cross-file extension cannot
            // justify an abort from these reporting tables. Retain its source
            // tombstone without claiming a successful Native retraction.
            return;
        }
        // Within Logical compatibility, an abort report replaces the source
        // tombstone for that same retraction. Neither reporting form describes
        // a successful Native operation or its current method table.
        if !aborts.is_empty() {
            class.retracted_members.retain(|record| {
                !aborts.iter().any(|a| {
                    a.kind == super::super::types::DefinitionAbortKind::MissingMember
                        && a.member == record.member
                })
            });
        }
        for abort in aborts {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W315,
                    abort.span,
                    abort.message(),
                    super::super::types::Severity::Warning,
                ));
        }
    }

    /// **W314**, namespace flavour: a `namespace eval` (or sibling) whose
    /// written name carries an **all-colon segment** (`namespace eval :`)
    /// creates a namespace no absolute path can spell — its entire contents
    /// are reachable only relatively (`namespace inscope : :`), and
    /// `namespace current` inside renders an unresolvable `:::`.
    pub(in super::super) fn emit_w314_unaddressable_namespace(
        &mut self,
        written_ns: &str,
        span: Span,
    ) {
        let has_all_colon_segment = crate::naming::qualifier_segments(written_ns.as_bytes())
            .into_iter()
            .any(|seg| seg == b":");
        if !has_all_colon_segment {
            return;
        }
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W314,
                span,
                format!(
                    "namespace '{written_ns}' has no absolute (fully-qualified) path — a \
                 written colon run is a namespace separator, so everything defined \
                 inside is reachable only by relative lookup"
                ),
                super::super::types::Severity::Warning,
            ));
    }

    /// **W218.** `args` declared anywhere but the final parameter position
    /// is an ordinary parameter named `args` — C Tcl sets `VAR_IS_ARGS`
    /// only on the last formal (`tclProc.c`), so the variadic
    /// collect-the-rest meaning is silently lost.  Anchors at the
    /// parameter's own name span; `fallback_tok` is used when the name
    /// span could not be recovered.  Shared by the `proc`, `apply`, and
    /// OO-method param walks.
    pub(in super::super) fn emit_w218_args_not_final(
        &mut self,
        params: &[crate::signature_scan::types::ParamDef],
        param_spans: &[tcl_lexer::Span],
        fallback_tok: Token,
    ) {
        let Some(last) = params.len().checked_sub(1) else {
            return;
        };
        for (i, p) in params.iter().enumerate() {
            // registry-axis-ok: irreducible — the variadic `args` formal is Tcl's proc grammar (`VAR_IS_ARGS` on the last formal, `tclProc.c`), no registry fact; until never
            if i == last || p.name != "args" {
                continue;
            }
            let span = param_spans.get(i).copied().unwrap_or(fallback_tok.span);
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W218,
                    span,
                    "`args` here is an ordinary parameter — it only has its special \
                     collect-the-rest meaning as the final parameter. Move it last, or \
                     rename it if a plain parameter is intended."
                        .to_string(),
                    super::super::types::Severity::Warning,
                ));
        }
    }
}
