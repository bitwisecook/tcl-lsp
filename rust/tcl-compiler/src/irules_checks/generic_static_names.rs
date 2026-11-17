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

//! IRULE4002: `static::` variables whose generic names collide across rules.

use super::{
    IrulesCheckPayload, IrulesCheckWarning, IrulesSourceQuery, OriginalEventPresence, is_irules,
};
use crate::compilation_unit::CompilationUnit;
use crate::ir::Statement;
use crate::sccp::cfg_order;
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;

// IRULE4002 — generic `static::` variable name that will collide
//
// Walks every original event CFG body for statements that define a
// `static::` variable and flags bare names matching a set of generic
// regex patterns. Deduplicated across events — the first occurrence's
// span wins. The pattern set is configurable (`tclLsp.diagnostics.
// genericVariablePatterns`): pass `None` for the built-in default set,
// `Some(&[])` to disable the check, or `Some(custom)` to replace the
// defaults.

/// Default generic `static::` variable-name patterns. Each is matched case-insensitively
/// (the bare name is lowercased) against the part after the `static::`
/// prefix. A user-supplied list replaces this set wholesale.
pub const DEFAULT_GENERIC_VARIABLE_PATTERNS: &[&str] = &[
    // Debug / logging
    r"^debug(_level|_enabled)?$",
    r"^dbg$",
    r"^log_(level|server|enabled)$",
    r"^logging$",
    r"^verbose$",
    r"^trace$",
    // Configuration
    r"^(response_)?timeout$",
    r"^(max_)?retr(y|ies)$",
    r"^config$",
    r"^(enabled|disabled|active)$",
    r"^mode$",
    r"^(port|host|server|pool)$",
    // Counters / limits
    r"^count(er)?$",
    r"^(limit|max_connections|threshold|rate|interval)$",
    // Generic state
    r"^(flag|level|status|state|version|name|value|data|result|test|init|default)$",
];

/// Compile the generic-name patterns for IRULE4002. `None` selects the
/// built-in [`DEFAULT_GENERIC_VARIABLE_PATTERNS`]; `Some(custom)` uses the
/// caller's list verbatim (an empty slice compiles to no patterns, which
/// disables the check). Invalid regexes are skipped.
pub(super) fn compile_generic_patterns(patterns: Option<&[String]>) -> Vec<regex::Regex> {
    let build = |src: &str| {
        regex::RegexBuilder::new(src)
            .case_insensitive(true)
            .build()
            .ok()
    };
    match patterns {
        None => DEFAULT_GENERIC_VARIABLE_PATTERNS
            .iter()
            .filter_map(|p| build(p))
            .collect(),
        Some(pats) => pats.iter().filter_map(|p| build(p)).collect(),
    }
}

/// Configured lexical patterns apply only after the namespace root is selected.
fn matches_generic_variable_pattern(simple_name: &str, compiled: &[regex::Regex]) -> bool {
    let lower = simple_name.to_ascii_lowercase();
    compiled.iter().any(|re| re.is_match(&lower))
}

#[cfg(test)]
pub(super) fn is_generic_static_name(var_name: &str, compiled: &[regex::Regex]) -> bool {
    matches_generic_variable_pattern(
        var_name.strip_prefix("static::").unwrap_or(var_name),
        compiled,
    )
}

/// Generic-static-name warnings for IRULE4002. `generic_patterns` follows the
/// [`compile_generic_patterns`] convention (`None` → default set, `Some(&[])`
/// → disabled, `Some(custom)` → replace defaults).
#[must_use]
pub fn find_generic_static_name_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
    generic_patterns: Option<&[String]>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let compiled = compile_generic_patterns(generic_patterns);
    if compiled.is_empty() {
        return out;
    }
    let registry = cu.ir_module.resolved_registry();
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return out;
    };
    let mut seen = HashSet::new();
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for bn in cfg_order(&fu.cfg) {
            if !fu.sccp.executable_blocks.contains(&bn) {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            for (index, stmt) in block.statements.iter().enumerate() {
                let labels: Vec<&str> = match stmt {
                    Statement::AssignConst { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    Statement::Call { defs, .. } => defs.iter().map(String::as_str).collect(),
                    _ => continue,
                };
                let places =
                    crate::place_bridge::def_places_at(stmt, bn, index, &contexts, registry);
                for place in places {
                    if crate::side_effects::namespace_storage_scope_for_place(
                        &place,
                        registry.profile(),
                    ) != crate::side_effects::StorageScope::Static
                    {
                        continue;
                    }
                    let Some(key) = crate::var_resolve::canonical_place_key(&place) else {
                        continue;
                    };
                    // A single assignment's label reports that same operation. Multi-output
                    // calls join their authored labels through the actual SSA cell map.
                    let label = if matches!(stmt, Statement::Call { .. }) {
                        fu.ssa.source_symbols_at(bn, index).and_then(|symbols| {
                            labels.iter().copied().find(|label| {
                                symbols.get(*label).is_some_and(|symbol| {
                                    fu.ssa
                                        .cell_key(*symbol)
                                        .is_some_and(|cell| cell.root() == key.root())
                                })
                            })
                        })
                    } else {
                        labels.first().copied()
                    };
                    let Some(label) = label else {
                        continue;
                    };
                    check_generic_static(
                        &place,
                        &key,
                        label,
                        fu.abs_span(stmt.span()),
                        &compiled,
                        &mut seen,
                        &mut out,
                    );
                }
            }
        }
    }
    out
}

fn check_generic_static(
    place: &crate::place::Place,
    key: &crate::var_resolve::VariableCellKey,
    source_name: &str,
    span: Span,
    compiled: &[regex::Regex],
    seen: &mut HashSet<crate::var_resolve::VariableCellKey>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    let root = key.root().clone();
    if seen.contains(&root) {
        return;
    }
    let Some(bare) = crate::var_resolve::namespace_variable_simple_name(place) else {
        return;
    };
    let pattern_name = if let Some(index) = &place.index {
        if index.kind != crate::place::IndexKind::Literal {
            return;
        }
        let Ok(element) = index.value.try_utf8() else {
            return;
        };
        std::borrow::Cow::Owned(format!("{bare}({element})"))
    } else {
        std::borrow::Cow::Borrowed(bare)
    };
    if !matches_generic_variable_pattern(&pattern_name, compiled) {
        return;
    }
    seen.insert(root);
    out.push(IrulesCheckWarning {
        span,
        code: DiagCode::Irule4002,
        message: format!(
            "'{source_name}' is a generic name that can collide with other iRules. \
             static:: names share storage across rule owners on each executing TMM — \
             prefix with the application or rule name (e.g. 'static::<app>_{pattern_name}')."
        ),
        payload: IrulesCheckPayload::GenericStaticName {
            source_name: source_name.to_owned(),
            pattern_name: pattern_name.into_owned(),
        },
        replacement: None,
        fixes: Vec::new(),
    });
}
