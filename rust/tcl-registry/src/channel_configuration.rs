// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source option relationships, independent of channel state.

use crate::{AuthoredSourceOptionBoundary, AuthoredSourceOptionScan};

/// Closed descriptor vocabulary for existing channel-configuration advice.
/// Declaring it supplies no channel, handler acceptance or runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelConfigurationSpec {
    /// Compare original encoding and translation values in a closed option run.
    EncodingTranslation,
}

impl ChannelConfigurationSpec {
    /// Complete SpecTcl and Studio authoring vocabulary.
    pub const ALL: &'static [Self] = &[Self::EncodingTranslation];

    pub(crate) fn project(
        self,
        scan: AuthoredSourceOptionScan<'_>,
    ) -> Option<AuthoredSourceChannelConfiguration> {
        if scan.boundary != AuthoredSourceOptionBoundary::End {
            return None;
        }
        let mut encoding = None;
        let mut translation = None;
        for option in scan.options {
            if !option.available {
                return None;
            }
            let values = option.values?;
            if values.len() != 1 {
                return None;
            }
            match self {
                Self::EncodingTranslation => match option.option.name {
                    "-encoding" => encoding = Some(values.start),
                    "-translation" => translation = Some(values.start),
                    _ => {}
                },
            }
        }
        Some(AuthoredSourceChannelConfiguration {
            encoding: encoding?,
            translation: translation?,
            binary_value: "binary",
        })
    }
}

/// Final original source values selected by one admitted option topology.
/// Effective ordinals retain captures; runtime configuration stays unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceChannelConfiguration {
    /// Effective post-head encoding-value ordinal.
    pub encoding: usize,
    /// Effective post-head translation-value ordinal.
    pub translation: usize,
    /// Descriptor-owned value spelling used by the existing source advice.
    pub binary_value: &'static str,
}

#[cfg(test)]
mod tests {
    use crate::InvocationWord::{Dynamic, Literal};

    #[test]
    fn original_channel_configuration_requires_its_descriptor_and_admitted_option_topology() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let current =
            crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let older = crate::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands()));
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (command, arguments, expected) in [
            (
                "fconfigure",
                vec![
                    Dynamic,
                    Literal("-enc"),
                    Literal("binary"),
                    Literal("-trans"),
                    Literal("crlf"),
                ],
                Some((2, 4)),
            ),
            (
                "chan",
                vec![
                    Literal("configure"),
                    Dynamic,
                    Literal("-encoding"),
                    Literal("binary"),
                    Literal("-translation"),
                    Literal("crlf"),
                ],
                Some((3, 5)),
            ),
            (
                "fconfigure",
                vec![
                    Dynamic,
                    Literal("-encoding"),
                    Literal("binary"),
                    Literal("-translation"),
                    Dynamic,
                ],
                Some((2, 4)),
            ),
            (
                "fconfigure",
                vec![
                    Dynamic,
                    Literal("-encoding"),
                    Literal("binary"),
                    Literal("-translation"),
                    Literal("crlf"),
                    Dynamic,
                    Dynamic,
                ],
                None,
            ),
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    current.commands(),
                    Some(current.context()),
                    crate::InvocationWords::structured(Literal(command), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let layout = selected
                .authored_source_channel_configuration()
                .map(|layout| (layout.encoding, layout.translation));
            assert_eq!(layout, expected, "{command} {arguments:?}");
        }
        let arguments = [
            Literal("configure"),
            Dynamic,
            Literal("-encoding"),
            Literal("binary"),
            Literal("-translation"),
            Literal("crlf"),
        ];
        assert!(
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                older.commands(),
                Some(older.context()),
                crate::InvocationWords::structured(Literal("chan"), &arguments)
                    .with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .is_none()
        );
        let mut registry = current
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut generic = registry.get("fconfigure").unwrap().clone();
        generic.name = "generic_channel";
        generic.channel_configuration = None;
        registry.insert(generic);
        let arguments = [
            Dynamic,
            Literal("-encoding"),
            Literal("binary"),
            Literal("-translation"),
            Literal("crlf"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("generic_channel"), &arguments)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(
            selected
                .authored_source_descriptors()
                .command
                .arg_roles
                .iter()
                .any(|(_, role)| *role == crate::ArgRole::Channel)
        );
        assert!(
            selected.authored_source_channel_configuration().is_none(),
            "channel argument role does not donate configuration semantics"
        );
    }
}
