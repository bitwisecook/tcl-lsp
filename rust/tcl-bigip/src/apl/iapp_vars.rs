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

//! Original iApp source references projected through the APL field convention.

use std::sync::Arc;

use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::model::ContextRegistry;

use super::variable_names::project_variable_root;
use crate::range::Range;

/// An original variable substitution with a conventional APL field label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IappVarRef {
    /// Original analytical root spelling, independently of a native cell.
    pub tcl_name: String,
    /// Conventional presentation field label.
    pub apl_name: String,
    /// Original complete substitution range.
    pub range: Range,
}

/// Source candidates with independent absence coverage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IappSourceReferences {
    references: Vec<IappVarRef>,
    complete: bool,
}

impl IappSourceReferences {
    /// Positive original source candidates remain useful without full coverage.
    #[must_use]
    pub fn references(&self) -> &[IappVarRef] {
        &self.references
    }

    /// Only covered source can establish that a field is never referenced.
    #[must_use]
    pub fn complete_references(&self) -> Option<&[IappVarRef]> {
        self.complete.then_some(self.references.as_slice())
    }
}

/// Extract original source advice under the exact selected editing inputs.
/// Unknown body roles or computed names cannot establish reference absence.
#[must_use]
pub fn extract_iapp_var_refs_with_context(
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
    context: Arc<ContextRegistry>,
    config: LexerConfig,
) -> IappSourceReferences {
    let Some(original) =
        tcl_irules::original_hosted_source_variable_references(source, profile, context, config)
    else {
        return IappSourceReferences::default();
    };
    let line_index = LineIndex::new(source);
    let mut references = Vec::new();
    let mut complete = original.complete;
    for span in original.references {
        let Some(raw) = source.as_bytes().get(span.as_range()) else {
            complete = false;
            continue;
        };
        let Ok(Some(root)) = tcl_syntax::naming::variable_reference_root_bytes(raw, config) else {
            complete = false;
            continue;
        };
        let Ok(root) = std::str::from_utf8(root) else {
            complete = false;
            continue;
        };
        let Some(convention) = project_variable_root(root) else {
            continue;
        };
        if !convention.absolute {
            complete = false;
            continue;
        }
        references.push(IappVarRef {
            tcl_name: root.to_owned(),
            apl_name: convention.apl_name,
            range: Range::from_offsets(
                source,
                &line_index,
                span.start() as usize,
                span.end().saturating_sub(1) as usize,
            ),
        });
    }
    IappSourceReferences {
        references,
        complete,
    }
}

/// Compatibility report of positive source candidates under `f5-iapps`.
/// The returned slice alone does not certify complete reference coverage.
#[must_use]
pub fn extract_iapp_var_refs(source: &str) -> Vec<IappVarRef> {
    let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
    extract_iapp_var_refs_with_context(
        source,
        environment.analyser_profile(),
        environment.default_context_registry(),
        LexerConfig::from_grammar(environment.grammar()),
    )
    .references
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_braced_and_unbraced() {
        let src = "set a $::basic__addr\nset b ${::ssl__members__port}";
        let refs = extract_iapp_var_refs(src);
        let names: Vec<&str> = refs.iter().map(|r| r.apl_name.as_str()).collect();
        assert_eq!(names, vec!["basic.addr", "ssl.members.port"]);
        assert_eq!(refs[0].tcl_name, "::basic__addr");
    }

    #[test]
    fn drops_plain_non_iapp_vars() {
        // No `__` separator — not an iApp reference.
        assert!(extract_iapp_var_refs("set x $::tcl_version").is_empty());
    }

    #[test]
    fn ignores_array_index() {
        let refs = extract_iapp_var_refs("set v $::pool__members(0)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].apl_name, "pool.members");
    }

    fn configured(source: &str) -> IappSourceReferences {
        let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
        extract_iapp_var_refs_with_context(
            source,
            environment.analyser_profile(),
            environment.default_context_registry(),
            LexerConfig::from_grammar(environment.grammar()),
        )
    }

    #[test]
    fn original_source_components_exclude_comments_and_inert_data() {
        let source = r#"# $::unseen__comment
set a {$::unseen__literal}
set b \$::unseen__escaped
set c "$::basic__addr"
"#;
        let refs = configured(source);
        assert_eq!(
            refs.references()
                .iter()
                .map(|r| r.apl_name.as_str())
                .collect::<Vec<_>>(),
            ["basic.addr"]
        );
        assert!(refs.complete_references().is_some());
        let span = refs.references()[0].range;
        assert_eq!(
            &source[span.start.offset as usize..=span.end.offset as usize],
            "$::basic__addr"
        );
    }

    #[test]
    fn original_root_geometry_preserves_literal_colons_and_array_forms() {
        let source = r#"set a $:::basic__addr
set b ${:basic__addr}
set c $::ns::basic__addr
set d $::pool__members(0)
set e ${::pool__members(0)}"#;
        let refs = configured(source);
        assert_eq!(
            refs.references()
                .iter()
                .map(|r| r.apl_name.as_str())
                .collect::<Vec<_>>(),
            ["basic.addr", "pool.members", "pool.members"]
        );
        assert_eq!(refs.references()[0].tcl_name, ":::basic__addr");
        assert!(refs.complete_references().is_none());
    }

    #[test]
    fn selected_body_roles_keep_positive_candidates_without_absence_coverage() {
        let source = "proc helper {} {set a $::basic__addr}\nunknown_helper {$::unseen__field}";
        let refs = configured(source);
        assert_eq!(
            refs.references()
                .iter()
                .map(|r| r.apl_name.as_str())
                .collect::<Vec<_>>(),
            ["basic.addr"]
        );
        assert!(refs.complete_references().is_none());
    }

    #[test]
    fn expression_and_command_source_keep_their_original_substitutions() {
        let source = "set a [list $::basic__addr]\nexpr {$::ssl__members__port eq \"443\"}";
        let refs = configured(source);
        assert_eq!(
            refs.references()
                .iter()
                .map(|r| r.apl_name.as_str())
                .collect::<Vec<_>>(),
            ["basic.addr", "ssl.members.port"]
        );
        assert!(refs.complete_references().is_none());
    }

    #[test]
    fn relative_convention_labels_cannot_supply_a_global_reference() {
        let refs = configured("set a $local__field");
        assert!(refs.references().is_empty());
        assert!(refs.complete_references().is_none());
    }
}
