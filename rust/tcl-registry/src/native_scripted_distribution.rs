// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native scripted libraries, separate from core registration and lookup.

use crate::InvocationDialect;

/// Independently selected library roster at a source-distribution boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeScriptedLibrary {
    /// Original Jim dictionary procedures.
    Dictionary,
    /// Original Jim namespace-ensemble helper and its ensemble constructor.
    NamespaceEnsemble,
    /// Original Jim namespace-aware info helper, independent of core inventory.
    NamespaceInfo,
}

impl NativeScriptedLibrary {
    /// Explicit distribution bootstrap roster; each library retains its own binding owners.
    pub const ALL: &'static [Self] = &[
        Self::Dictionary,
        Self::NamespaceInfo,
        Self::NamespaceEnsemble,
    ];
}

/// Audited declaration source; this supplies no live command-binding authority.
#[derive(Debug, Clone, Copy)]
pub struct NativeScriptedProcedure {
    /// Original command spelling, including multiword names.
    pub command: &'static str,
    /// Original formal declaration, including defaults and reference formals.
    pub parameters: &'static str,
    /// Original body bytes with unchanged indentation and boundary newlines.
    pub body: &'static str,
}

const JIM_NAMESPACE_INFO: &[NativeScriptedProcedure] = &[NativeScriptedProcedure {
    command: "namespace info",
    parameters: "cmd {pattern *}",
    body: include_str!("native_scripts/jim_namespace_info.tcl"),
}];

const JIM_NAMESPACE_ENSEMBLE: &[NativeScriptedProcedure] = &[
    NativeScriptedProcedure {
        command: "ensemble",
        parameters: "command args",
        body: include_str!("native_scripts/jim_ensemble.tcl"),
    },
    NativeScriptedProcedure {
        command: "namespace ensemble",
        parameters: "subcommand args",
        body: include_str!("native_scripts/jim_namespace_ensemble.tcl"),
    },
];

/// Select original source for explicit library bootstrap. Core construction
/// remains independent; an invocation must resolve the current binding.
pub fn procedures(
    dialect: InvocationDialect,
    library: NativeScriptedLibrary,
) -> impl Iterator<Item = NativeScriptedProcedure> {
    let dictionary = if library == NativeScriptedLibrary::Dictionary {
        crate::dictionary_scope::stock_scripted_wrappers(dialect)
    } else {
        &[]
    };
    let namespace = if library == NativeScriptedLibrary::NamespaceEnsemble
        && dialect
            .native_name_protocol()
            .is_some_and(|recipe| recipe.is_jim084())
    {
        JIM_NAMESPACE_ENSEMBLE
    } else {
        &[]
    };
    let namespace_info = if matches!(
        library,
        NativeScriptedLibrary::NamespaceInfo | NativeScriptedLibrary::NamespaceEnsemble
    ) && dialect
        .native_name_protocol()
        .is_some_and(|recipe| recipe.is_jim084())
    {
        JIM_NAMESPACE_INFO
    } else {
        &[]
    };
    dictionary
        .iter()
        .map(|procedure| NativeScriptedProcedure {
            command: procedure.command,
            parameters: procedure.parameters,
            body: procedure.body,
        })
        .chain(namespace_info.iter().copied())
        .chain(namespace.iter().copied())
}

impl InvocationDialect {
    /// Jim's namespace extension forms a literal helper name from the original
    /// selector's native C-string extent. It does not substitute the canonical
    /// table entry or grant a C ensemble-configuration protocol.
    #[must_use]
    pub fn native_namespace_scripted_helper(self, selector: &[u8]) -> Option<Vec<u8>> {
        let protocol = self.native_name_protocol()?;
        if !protocol.is_jim084() {
            return None;
        }
        let projection = protocol.namespace_subcommand_input(selector);
        let mut command = b"namespace ".to_vec();
        command.extend_from_slice(projection.selected());
        Some(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::{
        TclVersion,
        model::{DialectPoint, Release},
    };

    #[test]
    fn original_namespace_info_library_matches_public_declaration() {
        // Native proof: naming.info.original-namespace-helper-declaration-and-current-forwarding
        // docs/design/analysis/name-resolution-proofs/info-original-namespace-helper-declaration-and-current-forwarding.md
        let captured = include_str!(
            "../tests/data/native_jim_info_helper_forwarding258/jim/original-helper-formals-and-body/stdout"
        );
        let fields: Vec<_> = captured
            .lines()
            .find(|line| line.starts_with("ORIGINAL|"))
            .unwrap()
            .split('|')
            .collect();
        assert_eq!(fields[1], "0");
        let bytes: Vec<_> = fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let observed = tcl_syntax::list::split_native_list_bytes(
            &bytes,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        let dialect = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        assert_eq!(observed.len(), 2);
        let roster: Vec<_> = procedures(dialect, NativeScriptedLibrary::NamespaceInfo).collect();
        assert_eq!(roster.len(), 1);
        assert_eq!(roster[0].command, "namespace info");
        assert_eq!(roster[0].parameters.as_bytes(), observed[0].as_ref());
        assert_eq!(roster[0].body.as_bytes(), observed[1].as_ref());
        for version in TclVersion::ALL {
            assert_eq!(
                procedures(
                    InvocationDialect::for_version(version),
                    NativeScriptedLibrary::NamespaceInfo
                )
                .count(),
                0
            );
        }
    }

    #[test]
    fn original_namespace_library_metadata_matches_public_formals_and_bodies() {
        // Native proof: naming.namespace.jim-original-scripted-helper-availability-and-forwarding
        // docs/design/analysis/name-resolution-proofs/namespace-jim-original-scripted-helper-availability-and-forwarding.md
        // The exact public declarations establish source assets, not their
        // continued live binding or native core/compiler capabilities.
        let stdout = include_str!(
            "../tests/data/native_jim_namespace_distribution236/jim/original-library-formals-and-bodies/stdout"
        );
        let fields: Vec<_> = stdout
            .lines()
            .find(|row| row.starts_with("ORIGINAL|"))
            .unwrap()
            .split('|')
            .collect();
        assert_eq!(fields[1], "0");
        let tuple: Vec<_> = fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let observed = tcl_syntax::list::split_native_list_bytes(
            &tuple,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        assert_eq!(observed.len(), 4);
        let dialect = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let roster: Vec<_> =
            procedures(dialect, NativeScriptedLibrary::NamespaceEnsemble).collect();
        for (ordinal, command) in ["namespace ensemble", "ensemble"].into_iter().enumerate() {
            let procedure = roster
                .iter()
                .find(|procedure| procedure.command == command)
                .unwrap();
            assert_eq!(
                procedure.parameters.as_bytes(),
                observed[ordinal * 2].as_ref()
            );
            assert_eq!(
                procedure.body.as_bytes(),
                observed[ordinal * 2 + 1].as_ref()
            );
        }
        for version in TclVersion::ALL {
            assert_eq!(
                procedures(
                    InvocationDialect::for_version(version),
                    NativeScriptedLibrary::NamespaceEnsemble
                )
                .count(),
                0
            );
        }
        let unknown = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_79));
        assert_eq!(
            procedures(unknown, NativeScriptedLibrary::NamespaceEnsemble).count(),
            0
        );
        assert!(
            unknown
                .native_namespace_scripted_helper(b"ensemble")
                .is_none()
        );
    }
}
