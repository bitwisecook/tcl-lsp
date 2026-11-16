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

//! W313: destructive `file` subcommands on variable paths, normalisation and guards.

use super::*;

/// Run W313 over a single-block function whose SSA statements are built by
/// `build` against the function's variable interner.
fn w313_warnings(
    build: impl FnOnce(&mut SsaFunction) -> Vec<crate::ssa::SsaStatement>,
) -> Vec<TaintWarning> {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};

    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let ssa_stmts = build(&mut ssa);
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        for s in &ssa_stmts {
            b.statements.push(s.statement.clone());
        }
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: ssa_stmts,
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    find_destructive_file_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
    )
}

fn file_call(args: &[&str]) -> Statement {
    Statement::Call {
        span: tcl_lexer::Span::new(0, 20),
        command: "file".into(),
        canonical_command: Some("::file".into()),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

#[test]
fn w313_variable_path_to_file_delete() {
    use crate::ssa::SsaStatement;
    let w = w313_warnings(|ssa| {
        vec![SsaStatement {
            statement: file_call(&["delete", "$p"]),
            uses: [(ssa.intern_var("p"), 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }]
    });
    let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
    assert_eq!(d.variable, "p");
    assert!(
        d.message
            .contains("file delete with a variable path ($p) risks path-traversal"),
        "{}",
        d.message
    );
}

#[test]
fn w313_normalised_path_gets_softer_message() {
    use crate::ssa::SsaStatement;
    use tcl_lexer::Span;
    // set p [file normalize $base]; file delete $p
    let assign = Statement::AssignValue {
        span: Span::new(0, 25),
        name: "p".into(),
        name_braced: false,
        value: "[file normalize $base]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let w = w313_warnings(|ssa| {
        let base = ssa.intern_var("base");
        let p = ssa.intern_var("p");
        let s0 = ssa_stmt(
            assign,
            [(base, 0u32)].into_iter().collect(),
            [(p, 1u32)].into_iter().collect(),
        );
        let s1 = SsaStatement {
            statement: file_call(&["delete", "$p"]),
            uses: [(p, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };
        vec![s0, s1]
    });
    let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
    assert!(
        d.message.contains("file delete with normalised path ($p)"),
        "{}",
        d.message
    );
    // The `[string match …]` example must interpolate the variable
    // name (`${p}`), not render the literal placeholder `${name}`.
    assert!(
        d.message.contains("[string match \"$base/*\" ${p}]"),
        "{}",
        d.message
    );
    assert!(!d.message.contains("${name}"), "{}", d.message);
}

#[test]
fn w313_silent_for_literal_path() {
    use crate::ssa::SsaStatement;
    // `file delete /tmp/foo` has no variable path argument.
    assert_eq!(
        w313_warnings(|_ssa| {
            vec![SsaStatement {
                statement: file_call(&["delete", "/tmp/foo"]),
                uses: HashMap::new(),
                defs: HashMap::new(),
                may_defs: std::collections::HashSet::new(),
                destruction_defs: std::collections::HashSet::new(),
                quoted_uses: std::collections::HashSet::new(),
                name_only_uses: std::collections::HashSet::new(),
            }]
        })
        .len(),
        0
    );
}

#[test]
fn standalone_guard_names_follow_original_operand_kind_and_supplied_config() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (source, expected) in [
            ("[string match SAFE ${$literal}]", Some("$literal")),
            ("[string match SAFE ${}]", Some("")),
            ("[string match SAFE {$literal}]", None),
            ("[string match SAFE {${literal}}]", None),
            (
                "[string first SAFE ${qualified::literal}]",
                Some("qualified::literal"),
            ),
            ("[string first SAFE $arr(key)]", None),
            ("[string equal SAFE ${arr(key)}]", None),
            ("[string equal SAFE prefix$x]", None),
            ("[string match SAFE {*}$x]", None),
        ] {
            assert_eq!(
                guard_var_from_string_command(source, config).as_deref(),
                expected,
                "{label}: {source}"
            );
        }
        assert_eq!(
            guard_var_from_string_command("[string match SAFE ${a{b}c}]", config).as_deref(),
            if label == "tcl9.1" {
                Some("a{b}c")
            } else {
                None
            }
        );
    }
}

#[test]
fn w313_helpers_parse_vars_and_guards() {
    assert!(arg_var_names("$p", tcl_dialect::BracedVarStyle::default()).contains("p"));
    assert!(arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("dir"));
    assert!(arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("file"));
    assert_eq!(
        extract_var_name("$p", tcl_lexer::LexerConfig::default()).as_deref(),
        Some("p")
    );
    assert_eq!(
        extract_var_name("${p}", tcl_lexer::LexerConfig::default()).as_deref(),
        Some("p")
    );
    assert_eq!(
        guard_var_from_string_command(
            "[string match \"$base/*\" $p]",
            tcl_lexer::LexerConfig::default()
        )
        .as_deref(),
        Some("p")
    );
    assert_eq!(
        guard_var_from_string_command(
            "[string equal -length 4 $a $p]",
            tcl_lexer::LexerConfig::default()
        )
        .as_deref(),
        Some("a")
    );
}
