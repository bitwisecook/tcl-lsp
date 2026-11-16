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

//! Project source headers: per-slot arities, opaque slots and the body-edit firewall.

use super::*;

#[test]
fn formal_count_project_headers_keep_the_selected_c_and_jim_contract() {
    // naming.database.original-formal-count-header
    // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
    use salsa::Setter as _;
    use tcl_compiler::signature_scan::formal_count::SourceFormalCountOrigin;
    for (dialect, minimum, grammar) in [
        ("tcl8.4", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl8.5", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl8.6", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl9.0", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl9.1", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("jim", 2, tcl_dialect::ParameterGrammar::Jim),
    ] {
        let mut db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            "proc mixed {a {b B} c} {return FIRST}".to_owned(),
            dialect.to_owned(),
            None,
        );
        let project = Project::new(&db, vec![file]);
        let headers = item_sigs(&db, file);
        let signature = &headers[0];
        let header = signature.original_declaration.as_ref().unwrap();
        let count = header.formal_count_projection();
        assert_eq!(
            count.origin(),
            SourceFormalCountOrigin::OriginalSource,
            "{dialect}"
        );
        assert_eq!(count.parameter_grammar(), Some(grammar), "{dialect}");
        let key = header.name().clone();
        let original = project_original_command_arities(&db, project);
        assert_eq!(original.get(&key), Some(&vec![(minimum, 3)]), "{dialect}");
        let selected = original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, key.policy(), key.slot().clone()),
        );
        assert_eq!(selected.as_deref(), Some(&vec![(minimum, 3)]), "{dialect}");
        file.set_text(&mut db)
            .to("proc mixed {a {b B} c} {return A_LONGER_BODY}".to_owned());
        assert_eq!(
            project_original_command_arities(&db, project),
            original,
            "{dialect}"
        );
        assert_eq!(
            original_command_arity(
                &db,
                project,
                OriginalCommandSlot::new(&db, key.policy(), key.slot().clone()),
            ),
            selected,
            "{dialect}"
        );
        assert_eq!(item_sigs(&db, file), headers, "{dialect}");
    }
}

#[test]
fn original_project_headers_keep_opaque_slots_and_body_free_arity() {
    // Proof naming.database.original-header-arity:
    // docs/design/analysis/name-resolution-proofs/database-original-header-arity.md
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        r"proc p\uD800 {a} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![file]);
    let declarations = file_decls(&db, file);
    let first = declarations
        .original_declarations
        .iter()
        .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x80")
        .expect("first original byte declaration")
        .name()
        .clone();
    let second = declarations
        .original_declarations
        .iter()
        .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x81")
        .expect("second original byte declaration")
        .name()
        .clone();
    let first_key = OriginalCommandSlot::new(&db, first.policy(), first.slot().clone());
    let second_key = OriginalCommandSlot::new(&db, second.policy(), second.slot().clone());
    assert_eq!(
        original_command_arity(&db, project, first_key)
            .unwrap()
            .as_ref(),
        &[(1, 1)]
    );
    assert_eq!(
        original_command_arity(&db, project, second_key)
            .unwrap()
            .as_ref(),
        &[(2, 2)]
    );
    let wrong_provider = OriginalCommandSlot::new(
        &db,
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_5),
        first.slot().clone(),
    );
    assert!(original_command_arity(&db, project, wrong_provider).is_none());
    let before = project_original_command_arities(&db, project);
    file.set_text(&mut db)
        .to(r"proc p\uD800 {a} {return A_LONGER_BODY}
proc p\uD801 {a b} {return ANOTHER_BODY}"
            .to_owned());
    assert_eq!(project_original_command_arities(&db, project), before);
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(1, 1)]
    );
    file.set_text(&mut db)
        .to(r"proc p\uD800 {a b c} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
            .to_owned());
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(3, 3)]
    );
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, second.policy(), second.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(2, 2)]
    );
}

/// Original header arities ignore body edits and recompute once when the
/// selected signature changes.
#[test]
fn project_command_arities_firewall() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("project_original_command_arities"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let a = SourceFile::new(
        &db,
        "proc helper {x y} { set z 1 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "proc other {} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![a, b]);

    let headers = item_sigs(&db, a);
    let header = headers[0].original_declaration.as_ref().unwrap().name();

    // Cold: `helper` is a 2-param proc → arity (2, 2).
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
        )
        .map(|arities| (*arities).clone()),
        Some(vec![(2, 2)]),
        "cold: helper has arity (2, 2)"
    );
    let _ = runs(&log);

    // BODY edit to `helper` — `item_sigs(a)` is byte-identical (signatures
    // unchanged), so the arity table backdates and does NOT recompute.
    a.set_text(&mut db)
        .to("proc helper {x y} { set z 99999 }\n".to_owned());
    let _ = original_command_arity(
        &db,
        project,
        OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
    );
    assert_eq!(
        runs(&log),
        0,
        "body edit must not recompute project_original_command_arities"
    );

    // SIGNATURE edit — drop a parameter — `item_sigs(a)` changes, so the table
    // recomputes exactly once and the new arity (1, 1) flows through.
    a.set_text(&mut db)
        .to("proc helper {x} { set z 99999 }\n".to_owned());
    let arities = original_command_arity(
        &db,
        project,
        OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
    );
    assert_eq!(
        runs(&log),
        1,
        "signature edit must recompute project_original_command_arities exactly once"
    );
    assert_eq!(
        arities.map(|arities| (*arities).clone()),
        Some(vec![(1, 1)]),
        "the new parameter list flows through to arity (1, 1)"
    );
}
