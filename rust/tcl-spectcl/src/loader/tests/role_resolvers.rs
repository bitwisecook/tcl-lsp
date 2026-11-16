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

//! Argument-role resolvers: native cardinality, scan-count and structured-layout contracts and the capability set a resolver states.

use super::*;

#[test]
fn native_cardinality_role_contract_loads_and_tcl_count_hook_is_rejected() {
    let pack = evaluate_pack(
        r"speclib counts 1.0 {
            command copied {
                arg_role_count_resolver -native foreach::arg_role_count_resolver
                arg_role_resolver_roles Body
            }
            command unsupported {
                arg_role_count_resolver {nwords} {return {2 Body}}
                arg_role_resolver_roles Body
            }
        }",
    );
    let selected = pack.command("copied").expect("native contract loads");
    let resolver = selected
        .spec
        .arg_role_count_resolver
        .expect("actual count resolver");
    assert_eq!(resolver(3), vec![(2, ArgRole::Body)]);
    assert_eq!(resolver(5), vec![(4, ArgRole::Body)]);
    assert!(pack.command("unsupported").is_none());
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("Tcl count hooks are unsupported"))
    );
}

#[test]
fn native_scan_count_contracts_load_without_value_hooks() {
    let pack = evaluate_pack(
        r"speclib scans 1.0 {
                command copiedscan {
                    arg_role_count_resolver -native scan::arg_role_count_resolver
                    arg_role_resolver_roles {ScanFormat VarWrite}
                }
                command copiedbinary {
                    arg_role_count_resolver -native binary::scan::arg_role_count_resolver
                    arg_role_resolver_roles {ScanFormat VarWrite}
                }
            }",
    );
    for name in ["copiedscan", "copiedbinary"] {
        let spec = &pack
            .command(name)
            .expect("native cardinality descriptor")
            .spec;
        assert!(spec.arg_role_resolver.is_none());
        assert_eq!(
            spec.arg_role_count_resolver
                .expect("count-only native hook")(4),
            vec![
                (1, ArgRole::ScanFormat),
                (2, ArgRole::VarWrite),
                (3, ArgRole::VarWrite)
            ]
        );
    }
}

#[test]
fn native_structured_layout_loads_and_conflicting_inputs_are_rejected() {
    let pack = evaluate_pack(
        r"speclib layouts 1.0 {
            command copied {
                arg_role_layout_resolver -native regsub::arg_role_layout_resolver
                arg_role_resolver_roles {Pattern FormatString VarWrite}
            }
            command unsupported {
                arg_role_layout_resolver {words options} {return {0 Pattern}}
                arg_role_resolver_roles Pattern
            }
            command conflict {
                arg_role_layout_resolver -native regsub::arg_role_layout_resolver
                arg_role_count_resolver -native foreach::arg_role_count_resolver
                arg_role_resolver_roles {Pattern FormatString VarWrite Body}
            }
        }",
    );
    assert!(
        pack.command("copied")
            .unwrap()
            .spec
            .arg_role_layout_resolver
            .is_some()
    );
    assert!(pack.command("unsupported").is_none());
    assert!(pack.command("conflict").is_none());
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("Tcl layout hooks are unsupported"))
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("mutually exclusive"))
    );
}

#[test]
fn resolver_role_capabilities_load_at_both_registry_scopes() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n\
             command demo {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles {Expr Body Keyword Body}\n\
               subcommand run {\n\
                 arg_role_resolver {words ctx} { return }\n\
                 arg_role_resolver_roles {VarWrite Body}\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(
        spec.arg_role_resolver_roles,
        &[ArgRole::Expr, ArgRole::Body, ArgRole::Keyword]
    );
    assert_eq!(
        spec.subcommands[0].arg_role_resolver_roles,
        &[ArgRole::VarWrite, ArgRole::Body]
    );
}

#[test]
fn resolver_role_capabilities_fail_closed_for_assistance() {
    let pack = evaluate_pack(
        "speclib probe 2.1 {\n\
             command missing {\n\
               arg_role_resolver {words ctx} { return }\n\
             }\n\
             command empty {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles {}\n\
             }\n\
             command orphan {\n\
               arg_role_resolver_roles Body\n\
             }\n\
             command nested {\n\
               subcommand bad {\n\
                 arg_role_resolver {words ctx} { return }\n\
                 arg_role_resolver_roles {Body Invented}\n\
               }\n\
             }\n\
             }",
    );

    for name in ["missing", "empty", "orphan", "nested"] {
        assert!(
            pack.command(name)
                .expect("the safe degraded spec loads")
                .degraded,
            "{name} must not advertise complete assistance facts"
        );
    }
    assert!(
        pack.command("nested")
            .expect("nested loads")
            .spec
            .subcommands[0]
            .arg_role_resolver_roles
            .is_empty(),
        "one invalid member rejects the incomplete capability set"
    );
    assert!(
        pack.notices
            .iter()
            .all(|notice| notice.class == VocabularyClass::Assistance),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("without a non-empty")),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("without an `arg_role_resolver`")),
        "{:#?}",
        pack.notices
    );
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("`Invented`")),
        "{:#?}",
        pack.notices
    );
}

#[test]
fn resolver_role_capabilities_require_the_2_1_declaration() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n\
             command demo {\n\
               arg_role_resolver {words ctx} { return }\n\
               arg_role_resolver_roles Body\n\
             }\n\
             }",
    );
    assert!(pack.command("demo").is_some());
    assert!(pack.notices.iter().any(|notice| {
        notice
            .message
            .contains("`arg_role_resolver_roles` is SpecTcl 2.1 vocabulary")
    }));
}
