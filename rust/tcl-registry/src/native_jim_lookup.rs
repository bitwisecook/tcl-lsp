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

//! Authentic pinned Jim original-object lookup policy.

mod namespace_subcommands;
pub use namespace_subcommands::{
    NativeJimNamespaceSubcommand, NativeJimNamespaceSubcommandKind,
    NativeJimNamespaceSubcommandTable,
};

pub(crate) fn namespace_subcommand_declarations() -> &'static [NativeJimNamespaceSubcommand] {
    namespace_subcommands::NAMESPACE_SUBCOMMANDS
}

/// Selected pinned namespace procedure family; no callable or entry authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeJimNamespaceHelperKind {
    /// Original `namespace which` distribution procedure.
    Which,
    /// Original `namespace code` distribution procedure.
    Code,
    /// Original `namespace origin` distribution procedure.
    Origin,
    /// Original `namespace inscope` distribution procedure.
    Inscope,
    /// Original namespace deletion distribution procedure.
    Delete,
    /// Original namespace variable-link distribution procedure.
    Upvar,
    /// Original private namespace-aware inventory distribution procedure.
    Info,
    /// Original namespace import distribution procedure.
    Import,
    /// Original namespace ensemble distribution procedure.
    Ensemble,
}
impl NativeJimNamespaceHelperKind {
    /// Pinned source families this owner currently supplies.
    pub const ALL: [Self; 9] = [
        Self::Delete,
        Self::Origin,
        Self::Which,
        Self::Code,
        Self::Inscope,
        Self::Import,
        Self::Info,
        Self::Upvar,
        Self::Ensemble,
    ];
}

/// Authored pinned distribution procedure source, not a published callable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJimNamespaceHelper {
    kind: NativeJimNamespaceHelperKind,
    name: &'static str,
    parameters: &'static str,
    body: &'static str,
}
/// Original static nshelper extension's source input, without a live Source or Proc role.
#[derive(Debug, Clone, Copy)]
pub struct NativeJimNamespaceHelperStaticSource {
    body: &'static str,
    first_line: i32,
}
impl NativeJimNamespaceHelperStaticSource {
    /// Exact body bytes actually evaluated by the original generated static extension.
    #[must_use]
    pub const fn body(self) -> &'static str {
        self.body
    }
    /// Original static-extension source filename, independent of a raw source-file path.
    #[must_use]
    pub const fn filename(self) -> &'static [u8] {
        b"nshelper.tcl"
    }
    /// Original body's signed Source baseline, including its leading newline.
    #[must_use]
    pub const fn first_line(self) -> i32 {
        self.first_line
    }
}

impl NativeJimNamespaceHelper {
    /// Original selected source family, independent of physical helper publication.
    #[must_use]
    pub const fn kind(self) -> NativeJimNamespaceHelperKind {
        self.kind
    }
    /// `Jim_ParseSubCmd` validates the public operands before helper-head lookup.
    #[must_use]
    pub fn accepts_public_arguments(self, count: usize) -> bool {
        self.public_entry()
            .is_some_and(|entry| entry.accepts_arguments(count))
    }
    /// Exact public-subcommand argument synopsis.
    #[must_use]
    pub fn usage(self) -> &'static str {
        self.public_entry()
            .map_or("cmd ?pattern?", |entry| entry.usage())
    }
    fn public_entry(self) -> Option<&'static NativeJimNamespaceSubcommand> {
        namespace_subcommands::NAMESPACE_SUBCOMMANDS
            .iter()
            .find(|entry| entry.scripted_helper_kind() == Some(self.kind))
    }
    /// Select the original generated static extension's exact body and Source location.
    /// This descriptor supplies no installed binding, native header or entered frame.
    #[must_use]
    pub fn static_extension_source(self) -> NativeJimNamespaceHelperStaticSource {
        use NativeJimNamespaceHelperKind as Kind;
        let body = match self.kind {
            Kind::Code => include_str!("native_scripts/jim_namespace_code_static.tcl"),
            Kind::Import => include_str!("native_scripts/jim_namespace_import_static.tcl"),
            Kind::Info => include_str!("native_scripts/jim_namespace_info.tcl"),
            Kind::Ensemble => include_str!("native_scripts/jim_namespace_ensemble.tcl"),
            _ => self.body,
        };
        let first_line = match self.kind {
            Kind::Delete => 7,
            Kind::Origin => 17,
            Kind::Which => 32,
            Kind::Code => 49,
            Kind::Inscope => 57,
            Kind::Import => 61,
            Kind::Info => 87,
            Kind::Upvar => 138,
            Kind::Ensemble => 147,
        };
        NativeJimNamespaceHelperStaticSource { body, first_line }
    }
    /// Original compound registration spelling; public lookup keeps the actual selector.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
    /// Actual original procedure formal syntax.
    #[must_use]
    pub const fn parameters(self) -> &'static str {
        self.parameters
    }
    /// Authored raw nshelper.tcl body; this does not select the static extension's input.
    #[must_use]
    pub const fn body(self) -> &'static str {
        self.body
    }
}

impl crate::InvocationDialect {
    /// Select one pinned namespace distribution procedure source.
    /// This descriptor grants no installed slot, header or Native compiler entry.
    #[must_use]
    pub fn native_jim_namespace_helper(
        self,
        kind: NativeJimNamespaceHelperKind,
    ) -> Option<NativeJimNamespaceHelper> {
        self.native_jim_lookup_protocol()?;
        Some(match kind {
            NativeJimNamespaceHelperKind::Which => NativeJimNamespaceHelper {
                kind,
                name: "namespace which",
                parameters: "{type -command} name",
                body: include_str!("native_scripts/jim_namespace_which.tcl"),
            },
            NativeJimNamespaceHelperKind::Code => NativeJimNamespaceHelper {
                kind,
                name: "namespace code",
                parameters: "arg",
                body: include_str!("native_scripts/jim_namespace_code.tcl"),
            },
            NativeJimNamespaceHelperKind::Origin => NativeJimNamespaceHelper {
                kind,
                name: "namespace origin",
                parameters: "name",
                body: include_str!("native_scripts/jim_namespace_origin.tcl"),
            },
            NativeJimNamespaceHelperKind::Inscope => NativeJimNamespaceHelper {
                kind,
                name: "namespace inscope",
                parameters: "name arg args",
                body: "\n\ttailcall namespace eval $name $arg $args\n",
            },
            NativeJimNamespaceHelperKind::Delete => NativeJimNamespaceHelper {
                kind,
                name: "namespace delete",
                parameters: "args",
                body: include_str!("native_scripts/jim_namespace_delete.tcl"),
            },
            NativeJimNamespaceHelperKind::Upvar => NativeJimNamespaceHelper {
                kind,
                name: "namespace upvar",
                parameters: "ns args",
                body: include_str!("native_scripts/jim_namespace_upvar.tcl"),
            },
            NativeJimNamespaceHelperKind::Import => NativeJimNamespaceHelper {
                kind,
                name: "namespace import",
                parameters: "args",
                body: include_str!("native_scripts/jim_namespace_import.tcl"),
            },
            NativeJimNamespaceHelperKind::Ensemble => NativeJimNamespaceHelper {
                kind,
                name: "namespace ensemble",
                parameters: "subcommand args",
                body: include_str!("native_scripts/jim_namespace_ensemble_source.tcl"),
            },
            NativeJimNamespaceHelperKind::Info => NativeJimNamespaceHelper {
                kind,
                name: "namespace info",
                parameters: "cmd {pattern *}",
                body: include_str!("native_scripts/jim_namespace_info_source.tcl"),
            },
        })
    }

    /// Actual pinned Jim lookup-cache policy; compatibility versions and C
    /// command-name caches cannot supply this original-object protocol.
    #[must_use]
    pub fn native_jim_lookup_protocol(
        self,
    ) -> Option<tcl_syntax::native_jim_lookup::NativeJimLookupProtocol> {
        (self.native_name_protocol()? == tcl_syntax::naming::NativeNameProtocol::Jim084)
            .then_some(tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::jim084())
    }
}

/// Original object operations for Jim's global namespace-origin helper.
/// Canonicalisation retains the caller namespace; all getters use the actual
/// root namespace. Implementations retain their own provider and currency gates.
pub trait NativeJimNamespaceOriginBackend {
    /// Retained original header, independently of its reporting bytes.
    type Original: Clone;
    /// The adapter's original failure and host-refusal transport.
    type Error;
    /// Construct or retain the canonical operand in the actual caller namespace.
    fn canonical(&mut self, original: &Self::Original) -> Result<Self::Original, Self::Error>;
    /// Query the root alias flag and return the same original prefix object.
    fn alias_prefix(
        &mut self,
        canonical: &Self::Original,
    ) -> Result<Option<Self::Original>, Self::Error>;
    /// Query the original operand through the root command owner.
    fn command_exists(&mut self, original: &Self::Original) -> Result<bool, Self::Error>;
    /// Report the winning operand with Jim's counted root prefix.
    fn report(&mut self, original: &Self::Original) -> Result<Vec<u8>, Self::Error>;
    /// Compare actual header identity without comparing name strings.
    fn same_original(&self, left: &Self::Original, right: &Self::Original) -> bool;
    /// Refuse an unbounded alias cycle without supplying a guest result.
    fn alias_cycle_unavailable(&mut self) -> Self::Error;
}

/// A missing origin retains the actual recursive operand for its guest error.
pub enum NativeJimNamespaceOriginOutcome<V> {
    /// Counted report of the winning original operand.
    Found(Vec<u8>),
    /// Actual recursive operand used to report a missing command.
    Missing(V),
}

/// Execute the pinned helper's alias/canonical/original query order through
/// existing original-object getters. An alias prefix remains ONE name object.
///
/// # Errors
/// Preserves adapter refusals; a repeated actual prefix header refuses rather
/// than inventing a guest result for an unbounded original alias cycle.
pub fn native_jim_namespace_origin<B: NativeJimNamespaceOriginBackend>(
    backend: &mut B,
    original: &B::Original,
) -> Result<NativeJimNamespaceOriginOutcome<B::Original>, B::Error> {
    let mut original = original.clone();
    let mut prefixes = Vec::new();
    loop {
        let canonical = backend.canonical(&original)?;
        if let Some(prefix) = backend.alias_prefix(&canonical)? {
            if prefixes
                .iter()
                .any(|seen| backend.same_original(seen, &prefix))
            {
                return Err(backend.alias_cycle_unavailable());
            }
            prefixes.push(prefix.clone());
            original = prefix;
            continue;
        }
        if backend.command_exists(&canonical)? {
            return backend
                .report(&canonical)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        if backend.command_exists(&original)? {
            return backend
                .report(&original)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        return Ok(NativeJimNamespaceOriginOutcome::Missing(original));
    }
}

#[cfg(test)]
mod namespace_helper_source_tests {
    #[test]
    fn static_extension_and_raw_source_purposes_retain_their_actual_original_bodies() {
        let observed =
            include_str!("../tests/data/native_jim_namespace_helper_sources/original-20261015.tsv");
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let decode = |hex: &str| -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        for (index, kind) in super::NativeJimNamespaceHelperKind::ALL
            .into_iter()
            .enumerate()
        {
            let helper = dialect.native_jim_namespace_helper(kind).unwrap();
            let source = helper.static_extension_source();
            for (phase, body) in [
                ("STATIC", source.body()),
                ("RAW_ONLY", helper.body()),
                ("RAW_AFTER_STATIC", helper.body()),
            ] {
                let prefix = format!("BYTES|{phase}|{index}|BODY|");
                let expected = observed
                    .lines()
                    .find_map(|line| line.strip_prefix(&prefix))
                    .unwrap();
                assert_eq!(
                    body.as_bytes(),
                    decode(expected),
                    "{phase}/{}",
                    helper.name()
                );
                let prefix = format!("HELPER|{phase}|{index}|");
                let name = observed
                    .lines()
                    .find_map(|line| line.strip_prefix(&prefix))
                    .unwrap()
                    .split('|')
                    .next()
                    .unwrap();
                assert_eq!(helper.name().as_bytes(), decode(name));
            }
            let prefix = format!("SOURCE|STATIC|{index}|BODY|");
            let expected = observed
                .lines()
                .find_map(|line| line.strip_prefix(&prefix))
                .unwrap();
            let (line, filename) = expected.split_once('|').unwrap();
            assert_eq!(source.first_line(), line.parse::<i32>().unwrap());
            assert_eq!(source.filename(), decode(filename));
        }
    }

    #[test]
    fn which_helper_descriptor_keeps_the_original_pinned_declaration() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let helper = crate::InvocationDialect::of_profile(environment.unit_profile())
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Which)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        let declaration = format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        );
        assert!(source.contains(&declaration));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Which)
                    .is_none()
            );
        }
    }
    #[test]
    fn code_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Code)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(helper.accepts_public_arguments(1));
        assert!(!helper.accepts_public_arguments(2));
        assert_eq!(helper.usage(), "arg");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Code)
                    .is_none()
            );
        }
    }
    #[test]
    fn origin_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Origin)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(helper.accepts_public_arguments(1));
        assert!(!helper.accepts_public_arguments(2));
        assert_eq!(helper.usage(), "name");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Origin)
                    .is_none()
            );
        }
    }
    #[test]
    fn inscope_helper_descriptor_keeps_original_source_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Inscope)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        assert!(!helper.accepts_public_arguments(1));
        assert!(helper.accepts_public_arguments(2));
        assert!(helper.accepts_public_arguments(3));
        assert!(helper.accepts_public_arguments(usize::MAX));
        assert_eq!(helper.usage(), "name arg ?arg ...?");
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Inscope)
                    .is_none()
            );
        }
    }
    #[test]
    fn info_helper_descriptor_keeps_original_private_procedure_without_public_dispatch() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Info)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        for count in [0, 1, 2, 3, usize::MAX] {
            assert!(!helper.accepts_public_arguments(count));
        }
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Info)
                    .is_none()
            );
        }
    }
    #[test]
    fn delete_helper_descriptor_keeps_original_variadic_procedure_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Delete)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert_eq!(helper.usage(), "?name ...?");
        for count in [0, 1, 2, usize::MAX] {
            assert!(helper.accepts_public_arguments(count));
        }
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Delete)
                    .is_none()
            );
        }
    }
    #[test]
    fn upvar_helper_descriptor_keeps_original_variadic_procedure_and_public_arity() {
        let environment = crate::model::ingress::resolve_known_environment("jim").unwrap();
        let dialect = crate::InvocationDialect::of_profile(environment.unit_profile());
        let helper = dialect
            .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Upvar)
            .unwrap();
        let source = include_str!("../tests/data/native_jim_namespace_which/nshelper.tcl");
        assert!(source.contains(&format!(
            "proc {{{}}} {{{}}} {{{}}}",
            helper.name(),
            helper.parameters(),
            helper.body()
        )));
        assert!(!helper.accepts_public_arguments(0));
        for count in [1, 2, 3, usize::MAX] {
            assert!(helper.accepts_public_arguments(count));
        }
        assert_eq!(helper.usage(), "ns ?arg ...?");
        assert!(super::NativeJimNamespaceHelperKind::ALL.contains(&helper.kind()));
        for version in tcl_dialect::TclVersion::ALL {
            let environment =
                crate::model::ingress::resolve_known_environment(version.dialect_name()).unwrap();
            assert!(
                crate::InvocationDialect::of_profile(environment.unit_profile())
                    .native_jim_namespace_helper(super::NativeJimNamespaceHelperKind::Upvar)
                    .is_none()
            );
        }
    }
}
