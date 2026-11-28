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
mod original_object_value_tests {
    use tcl_irules::*;

    #[test]
    fn object_candidates_decode_original_literal_values_and_refuse_live_components() {
        crate::support::install_surfaces();
        let registry = tcl_registry::model::ingress::static_context_for_profile(
            tcl_dialect::DialectProfile::irules(),
        )
        .commands();
        let source = concat!(
            "when HTTP_REQUEST {\n",
            "pool \"a\\\"b\"\n",
            "pool {p[set ::hint 1]}\n",
            "pool \"\\x24literal\"\n",
            "pool {茶}\n",
            "pool \"\"\n",
            "pool {}\n",
            "pool $dynamic\n",
            "pool [dynamic]\n",
            "}\n",
        );
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let references = extract_original_irules_source_object_references(source, &context, None);
        assert_eq!(
            references
                .iter()
                .map(|reference| reference.name.as_str())
                .collect::<Vec<_>>(),
            ["a\"b", "p[set ::hint 1]", "$literal", "茶", "", ""]
        );
        assert!(
            references
                .iter()
                .all(|reference| reference.category == IrulesObjectReferenceCategory::Pool)
        );
        assert_eq!(
            extract_original_irules_source_object_references(
                &format!("# changed\n{source}"),
                &context,
                None
            ),
            [] as [tcl_irules::IrulesObjectReference; 0]
        );
    }
}
