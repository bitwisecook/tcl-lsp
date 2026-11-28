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

#[cfg(test)]
mod tests {
    use tcl_irules::*;
    use tcl_registry::InvocationWord;

    #[test]
    fn original_data_group_operands_keep_unknown_item_and_refuse_unknown_reference() {
        crate::support::install_surfaces();
        use InvocationWord::{Dynamic, Expanded, Literal};
        let context = tcl_registry::model::ingress::static_context_for("f5-irules");
        let select = |args: &[InvocationWord<'_>]| {
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(InvocationWord::Literal("class"), args),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .map_or_else(Vec::new, |selected| {
                original_source_object_operands(&selected, None)
            })
        };
        let operands = select(&[Literal("match"), Dynamic, Literal("equals"), Literal("dg")]);
        assert_eq!(operands.len(), 1);
        assert_eq!(operands[0].argument(), 3);
        assert_eq!(operands[0].attachment(), None);
        assert_eq!(
            operands[0].kinds(),
            vec![
                "ltm_data_group_external",
                "ltm_data_group_internal",
                "sys_file_data_group"
            ]
        );
        for args in [
            vec![Literal("match"), Dynamic, Literal("equals"), Dynamic],
            vec![Literal("match"), Expanded, Literal("equals"), Literal("dg")],
            vec![
                Literal("match"),
                Literal("-unknown"),
                Dynamic,
                Literal("equals"),
                Literal("dg"),
            ],
        ] {
            assert!(select(&args).is_empty(), "{args:?}");
        }
    }

    #[test]
    fn typed_configuration_decline_does_not_reenter_literal_compatibility() {
        crate::support::install_surfaces();
        use InvocationWord::Literal;
        let context = tcl_registry::model::ingress::static_context_for("f5-irules");
        for args in [
            vec!["match", "item", "equals", "dg", "extra"],
            vec!["match", "-unknown", "item", "equals", "dg"],
            vec!["match", "item", "and", "dg"],
            vec!["match", "item", "equals"],
            vec!["match", "item", "equals", "dg\0tail"],
        ] {
            let words = args.iter().copied().map(Literal).collect::<Vec<_>>();
            let selected =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    tcl_registry::InvocationWords::structured(Literal("class"), &words),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let selected = selected.resolved().unwrap();
            assert!(
                original_source_object_operands(&selected, None).is_empty(),
                "{args:?}"
            );
        }
        let compatibility = tcl_irules::resolve_object_ref_args(
            "class",
            &["match", "item", "equals", "dg", "extra"],
            None,
        );
        assert_eq!(compatibility.len(), 1);
        assert_eq!(compatibility[0].0, 3);
    }

    #[test]
    fn original_attachment_operands_keep_snat_pool_purpose_and_original_cardinality() {
        crate::support::install_surfaces();
        let context = tcl_registry::model::ingress::static_context_for("f5-irules");
        let select = |head, args: &[InvocationWord<'_>]| {
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(InvocationWord::Literal(head), args),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .map_or_else(Vec::new, |selected| {
                original_source_object_operands(&selected, None)
            })
        };
        for (head, kind) in [
            ("pool", IrulesSourceAttachmentKind::Pool),
            ("node", IrulesSourceAttachmentKind::Node),
            ("snatpool", IrulesSourceAttachmentKind::SnatPool),
        ] {
            let operands = select(head, &[InvocationWord::Dynamic]);
            assert_eq!(operands.len(), 1);
            assert_eq!(operands[0].argument(), 0);
            assert_eq!(operands[0].attachment(), Some(kind));
            assert!(select(head, &[InvocationWord::Expanded]).is_empty());
        }
        assert!(select("snat", &[InvocationWord::Literal("10.0.0.1")]).is_empty());
        let operands = select(
            "snat",
            &[InvocationWord::Literal("pool"), InvocationWord::Dynamic],
        );
        assert_eq!(operands.len(), 1);
        assert_eq!(operands[0].argument(), 1);
        assert_eq!(
            operands[0].attachment(),
            Some(IrulesSourceAttachmentKind::SnatPool)
        );
        assert!(select("pool", &[InvocationWord::Literal("none")]).is_empty());
    }
}
