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

//! Object methods across files: the override family, its consumers and the navigation and rename built on them.

use tcl_compiler::analyser::AnalysisResult;

mod class_analysis;
mod consumer_scans;
mod family;
mod navigation;
mod rename;

pub(super) use class_analysis::WorkspaceClassAnalysis;
#[cfg(test)]
pub(super) use class_analysis::workspace_class_analysis_fingerprint;
pub(super) use consumer_scans::ConsumerCoverage;

/// What one pure-consumer method scan is about: the member, and how wide a
/// document set to cover.  Grouped so the scan entry points stay within the
/// argument budget now that they carry a coverage mode.
#[derive(Debug, Clone, Copy)]
pub(super) struct ConsumerTarget<'a> {
    /// The class the cursor resolved the member against.
    pub(super) seed_class: &'a str,
    /// The member name.
    pub(super) method: &'a str,
    /// Whether it lives in the class-object dispatch table.
    pub(super) is_classmethod: bool,
    /// How wide a document set to scan — see [`ConsumerCoverage`].
    pub(super) coverage: ConsumerCoverage,
}

/// The workspace facts one pure-consumer method scan needs, resolved under a
/// single index read by [`Backend::consumer_scan_plan`].
struct ConsumerScan {
    /// Qualified names of every class whose instances dispatch the method to
    /// the family — the override-family definers plus the pure inheritors.
    family: Vec<String>,
    /// Documents **declaring** a family class — the definers' and pure
    /// inheritors' own files.
    family_uris: Vec<String>,
    /// Candidate consumer documents: those invoking a family constructor.
    consumer_uris: Vec<String>,
    /// Class names valid as a bare classmethod-dispatch head; empty for an
    /// instance method.  See [`Backend::classmethod_dispatch_names`].
    classmethod_cmd_names: Vec<String>,
}

/// The document-local inputs [`family_spans_in_document`] scans one document
/// with — bundled so the cross-file references and rename passes share one
/// call shape rather than two near-identical argument lists.
#[derive(Clone, Copy)]
struct FamilySpanScan<'a> {
    pub(super) source: &'a str,
    pub(super) profile: &'static tcl_dialect::DialectProfile,
    pub(super) analysis: &'a AnalysisResult,
    method: &'a str,
    is_classmethod: bool,
    /// See [`core_references::InheritedReceiverFacts::extra_classmethod_cmd_names`].
    classmethod_cmd_names: &'a [String],
    /// The receivers the workspace proved can dispatch `method` externally —
    /// see [`MethodFamily::external_callback_receivers`].
    external_receivers: &'a std::collections::HashSet<String>,
}

/// The family classes one document declares: the **definers** that (re)define
/// the method and the pure **inheritors** that only inherit it.
#[derive(Default)]
struct FamilyClasses {
    definers: Vec<String>,
    inheritors: Vec<String>,
}

/// A method's override family grouped by declaring document, plus the class
/// names valid as a bare classmethod-dispatch head.  See
/// [`Backend::method_family_by_document`].
struct MethodFamily {
    pub(super) by_uri: Vec<(String, FamilyClasses)>,
    classmethod_cmd_names: Vec<String>,
    /// The inheriting receivers that can dispatch the method **externally**,
    /// so a captured `[self]` object command written in their bodies really
    /// reaches the family's declaration.
    ///
    /// A property of the receiver, not of the provider: a subclass can
    /// `export` / `unexport` a name it inherits without redeclaring it, and
    /// the two halves of the answer routinely live in different documents —
    /// the stub next to the subclass, the declared visibility next to the
    /// base.  The workspace dispatch chain already folds both, so it decides
    /// here, under the same index read the family itself is built from, and
    /// the per-document scans are handed a plain fact.
    ///
    /// A `my` capture is not gated on this: it keeps the current object's
    /// internal dispatch, which ignores the export flag entirely (tclsh
    /// 8.6.16 / 9.0.4).
    external_callback_receivers: std::collections::HashSet<String>,
}
