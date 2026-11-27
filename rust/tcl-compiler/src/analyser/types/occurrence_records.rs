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

//! Records of single source occurrences: package provides and `ifneeded` scripts, `auto_path` entries, qualified variable and namespace references, instance bindings and regex patterns.

use crate::signature_scan::scope::SignatureNamespaceChild;
use tcl_lexer::Span;

/// `package provide NAME ?VERSION?` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageProvide {
    /// Authentic original package operand and selected native package key.
    pub original_name: Option<crate::signature_scan::original_name::SourcePackageName>,
    /// Provided package name.
    pub name: String,
    /// Version string when present.
    pub version: Option<String>,
    /// Span of the originating ``package`` token.
    pub range: Span,
    /// ``true`` when the call is inside a guarded branch (an
    /// ``if``/``elseif``/``else`` body, a ``catch`` script, or a
    /// ``try``/``on``/``trap``/``finally`` clause), matching
    /// [`SignaturePackageRequire::conditional`].
    ///
    /// The shim idiom makes this load-bearing: tcllib's
    /// ``doctools2idx/import_json.tcl`` writes ``package provide dict 1``
    /// inside ``if {[package vcompare …] < 0} { if {[catch {package
    /// require dict}]} { … } }`` — a fake ``dict`` package supplied only
    /// on an old interpreter.  Reading that as "this document provides
    /// ``dict``" would name the wrong file as a package's provider on
    /// every other interpreter.
    pub conditional: bool,
}

/// `package ifneeded NAME VERSION ?SCRIPT?` record.
///
/// The script argument is deliberately **not** kept.  It is evaluated later,
/// in the global namespace, by whichever `package require` first needs this
/// version, and it may do anything at all — `source` one file, `load` a shared
/// library, branch on the platform, or provide the package inline.  A consumer
/// asking "which statements does a `package require NAME` run?" therefore
/// cannot answer it from here; what this record supplies is the *fact that the
/// question is open*, which is exactly the abstention a package-derived load
/// order needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIfneeded {
    /// Authentic original package operand and selected native package key.
    pub original_name: Option<crate::signature_scan::original_name::SourcePackageName>,
    /// Package the load script is registered for.
    pub name: String,
    /// Version the script is registered for.
    pub version: String,
    /// Span of the originating ``package`` token.
    pub range: Span,
}

/// One **namespace-qualified** variable occurrence — a read or a write whose
/// *written* name carries a `::` qualifier (`$::tomato::helper::TolEquals`,
/// `set app::colors::palette …`).
///
/// Recorded regardless of whether the named cell is declared in this document,
/// which is exactly what makes it useful: the declaring `namespace eval NS {
/// variable v }` routinely lives in a sibling file, so the occurrence resolves
/// to nothing locally and leaves no trace in the scope tree
/// ([`Scope::variables`] / [`VarDef::references`] only ever record a *resolved*
/// use).  The workspace index lifts these into the cross-document variable
/// reference set.
///
/// Deliberately **qualified-only**: an unqualified `$v` names whichever cell
/// the local scope chain supplies, which is a per-document question, so
/// indexing those would be whole-program variable indexing with no
/// statically-sound cross-file meaning.  A qualified name, by contrast, names
/// one cell in one namespace no matter where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedVarRef {
    /// The `::`-rooted cell the occurrence names (`::tomato::helper::TolEquals`),
    /// resolved against the occurrence's own namespace for a relative
    /// qualifier (`app::colors::palette` at global scope → `::app::colors::palette`).
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
    /// Original lexical root or retained evaluated variable-name operand. The
    /// producer kind selects neither a compiler primary nor an executed cell.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    /// Independently retained namespace geometry at this occurrence. Missing
    /// geometry cannot be reconstructed from `qualified_name`.
    pub original_namespace: Option<tcl_names::namespace::SignatureNamespaceScope>,
}

/// One occurrence of a word that **names a namespace** — every argument the
/// registry marks [`tcl_registry::ArgRole::NamespaceName`] (`namespace
/// children ::tomato`, `namespace exists ns`, `namespace delete ::a ::b`,
/// `namespace eval NS body`, `namespace inscope NS script`, `namespace upvar
/// NS v local`).
///
/// The namespace analogue of [`QualifiedVarRef`], and it exists for the same
/// reason: the declaring `namespace eval` block routinely lives elsewhere —
/// another block in the same file, or a sibling document — so the occurrence
/// leaves no trace in the scope tree, which only records a namespace as a
/// *container* ([`Scope`] with [`ScopeKind::Namespace`]) and never as a
/// referenceable symbol.
///
/// Unlike the variable table this one records **relative** names too, because
/// there is nothing per-document about them: a namespace word roots against
/// the command-resolution namespace in effect at the occurrence, and that
/// namespace is a lexical fact this document already knows. Pinned on tclsh
/// 9.0.4 and 8.6.16, byte-identically: inside `namespace eval ::outer`,
/// `namespace exists inner` answers `1` for `::outer::inner` while the same
/// words at global scope answer `0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceRef {
    /// Original namespace operand. List children and evaluated values remain
    /// readonly; their source lineage cannot acquire complete-word geometry.
    pub original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    /// Exact namespace context before interpreting the original operand.
    pub source_context: Option<tcl_names::namespace::SignatureNamespaceScope>,
    /// Original selected word value, retained before any report rendering.
    pub original_name: String,
    /// Candidate literal value range. Consumers verify these exact original bytes
    /// against source before projecting editable subranges; escaped/computed words decline.
    pub source_span: Option<Span>,
    /// Exact original namespace selection, independent of its rendered report.
    pub source_namespace: Option<tcl_names::namespace::SignatureNamespaceScope>,
    /// Independently selected naming policy for the original namespace operand.
    pub name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    /// The `::`-rooted namespace the occurrence names, with a relative
    /// spelling already rooted against the occurrence's own namespace.
    pub qualified_name: String,
    /// Byte span of the name token as written.
    pub span: Span,
    /// `true` when this occurrence is also a **declaring** site — the name
    /// word of a [`tcl_registry::Traits::DECLARES_NAMESPACE`] command
    /// (`namespace eval`).  Go-to-definition answers with these; the
    /// reference set is everything else.
    pub declares: bool,
}

impl NamespaceRef {
    /// Select the original namespace operand using independently retained context.
    /// The report fallback supplies presentation only when that context is absent.
    #[must_use]
    pub(crate) fn from_original(
        source_context: Option<tcl_names::namespace::SignatureNamespaceScope>,
        name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
        original_name: &str,
        span: Span,
        content_offset: u16,
        declares: bool,
        report_fallback: String,
    ) -> Self {
        // A native policy cannot interpret the reporting String as original
        // native units. The genuine input producer is joined separately below.
        let source_namespace = name_policy
            .is_none()
            .then(|| {
                source_context
                    .as_ref()
                    .and_then(|current| current.child(original_name, None))
            })
            .flatten();
        let qualified_name = source_namespace
            .as_ref()
            .and_then(tcl_names::namespace::SignatureNamespaceScope::display)
            .unwrap_or(report_fallback);
        let source_span = span
            .start()
            .checked_add(u32::from(content_offset))
            .and_then(|start| {
                Some(Span::new(
                    start,
                    start.checked_add(u32::try_from(original_name.len()).ok()?)?,
                ))
            });
        Self {
            original_name_input: None,
            source_context,
            original_name: original_name.to_owned(),
            source_span,
            source_namespace,
            name_policy,
            qualified_name,
            span,
            declares,
        }
    }

    pub(crate) fn with_original_input(
        mut self,
        input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    ) -> Self {
        if let Some(input) = &input {
            self.name_policy = Some(input.policy());
            self.source_namespace = self
                .source_context
                .as_ref()
                .and_then(|scope| scope.child_from_input(input));
            if let Some(report) = self
                .source_namespace
                .as_ref()
                .and_then(tcl_names::namespace::SignatureNamespaceScope::display)
            {
                self.qualified_name = report;
            }
        }
        self.original_name_input = input;
        self
    }

    pub(super) fn original_extent_span(
        &self,
        source: &str,
        wanted: &tcl_names::namespace::SignatureNamespaceScope,
        member: bool,
    ) -> Option<Span> {
        let input = self.original_name_input.as_ref()?;
        let key = input.original_word_key()?;
        let word = key.original_word();
        if word.image() != &tcl_lexer::SourceImage::document(source) {
            return None;
        }
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(word),
            input.policy().string_protocol(),
        )
        .ok()?;
        if captured.literal(0)? != input.bytes() {
            return None;
        }
        let protocol = input.policy().recipe();
        let current = self.source_context.as_ref()?.context()?;
        let wanted = wanted.context()?;
        let extent = if member {
            tcl_syntax::naming::native_written_namespace_member_extent(
                protocol,
                current,
                input.bytes(),
                wanted,
            )?
        } else {
            tcl_syntax::naming::native_written_namespace_prefix_extent(
                protocol,
                current,
                input.bytes(),
                wanted,
            )?
        };
        captured.original_literal_extent(0, extent)
    }

    /// Covering original prefix for an exact retained ancestor namespace.
    #[must_use]
    pub fn written_ancestor_span(
        &self,
        source: &str,
        wanted: &tcl_names::namespace::SignatureNamespaceScope,
    ) -> Option<Span> {
        self.original_extent_span(source, wanted, false)
    }
    /// Original component range for an exact retained namespace.
    #[must_use]
    pub fn written_member_span(
        &self,
        source: &str,
        wanted: &tcl_names::namespace::SignatureNamespaceScope,
    ) -> Option<Span> {
        self.original_extent_span(source, wanted, true)
    }
}

/// An original variable-name argument containing substitutions.
/// The resolved value requires dominating Logical declaration and source provenance;
/// an unresolved site cannot narrow the set of variable names it might reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicVariableNameSite {
    /// Byte span of the name word as written.
    pub span: Span,
    /// The name the word evaluates to when every substitution in it has a
    /// value that **dominates** this site — the straight-line constant
    /// retained original declaration and value provenance proves.
    ///
    /// [`None`] when it does not: a branch-dependent binding, a parameter, a
    /// command substitution, or any variable the constant lattice does not
    /// track.  A consumer must treat `None` as "could be anything".
    pub resolved: Option<String>,
    /// `true` when the position **writes** the named variable
    /// ([`tcl_registry::ArgRole::VarWrite`]), `false` when it reads it.
    pub writes: bool,
}

/// One `CLASS create NAME` object-command binding, namespace-qualified.
///
/// See [`AnalysisResult::instance_command_bindings`] for why the bare-name
/// set it accompanies is not enough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceCommandBinding {
    /// The object command's `::`-rooted qualified name — the creation site's
    /// own namespace applied to `NAME` as written (an already-qualified
    /// `NAME` is used as written).
    pub qualified_name: String,
    /// The `::`-rooted qualified name of the class whose `create` bound it.
    pub class_q: String,
}

/// Which `auto_path` mutation wrote an [`AutoPathEntry`].
///
/// The two forms differ in **arity**, so a consumer must not read them alike
/// (verified against `tclsh8.6` / `tclsh9.0`):
///
/// | Written | `llength $auto_path` gains |
/// |---------|----------------------------|
/// | `lappend auto_path a b` | 2 — one element per argument *word* |
/// | `lappend auto_path {p q}` | 1 — the word `p q`, spaces and all |
/// | `set auto_path {/o/p1 /o/p2}` | 2 — the right-hand side is a *list* |
/// | `set auto_path {/o/a {/o/w s} /o/b}` | 3 — the braced element stays one |
///
/// Retained-analysis consumers use
/// [`crate::auto_path_eval::capture_source_auto_path_expression`] to join the
/// row to its original selected mutation and apply the actual list grammar.
/// [`crate::auto_path_eval::evaluate_auto_path_entry`] is the separate
/// standalone authoring simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPathForm {
    /// `lappend auto_path WORD …` — one record per argument word, and that
    /// word is exactly one directory however many spaces it contains.
    Append,
    /// `set auto_path LIST` — one record holding the *whole* right-hand side,
    /// which is a Tcl list and may therefore name several directories.
    Assign,
}

/// `auto_path` mutation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoPathEntry {
    /// Raw path text as written.  One directory for [`AutoPathForm::Append`];
    /// a Tcl *list* of them for [`AutoPathForm::Assign`].
    pub raw_path: String,
    /// Span of the path argument.
    pub range: Span,
    /// Which mutation wrote this record — see [`AutoPathForm`].
    pub form: AutoPathForm,
}

/// `regexp` / `regsub` / `switch -regexp` literal-pattern record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexPattern {
    /// Span of the literal pattern token in source.
    pub range: Span,
    /// Raw pattern text.
    pub pattern: String,
    /// Originating command name (``"regexp"`` / ``"regsub"`` /
    /// ``"switch"``).
    pub command: String,
}
