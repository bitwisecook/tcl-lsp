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

//! The pack interpreter, through `run_pack_program`.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_spec_hooks::pack_eval::{
    PackEvalConfig, PackEvalFailure, UnknownHandler, WordHandler, run_pack_program,
};

type Captured = Rc<RefCell<Vec<Vec<String>>>>;

fn capture_all() -> (Captured, Vec<(&'static str, WordHandler)>, UnknownHandler) {
    let seen: Captured = Rc::new(RefCell::new(Vec::new()));
    let make = |word: &'static str, seen: &Captured| -> WordHandler {
        let seen = Rc::clone(seen);
        Rc::new(move |_ctx, args| {
            let mut row = vec![word.to_owned()];
            row.extend(args.iter().cloned());
            seen.borrow_mut().push(row);
            Ok(None)
        })
    };
    let vocabulary = vec![("register", make("register", &seen))];
    let unknown: UnknownHandler = {
        let seen = Rc::clone(&seen);
        Rc::new(move |_ctx, name, args| {
            if name == "clock" || name.contains("mathfunc::") {
                return Err(format!("`{name}` denied"));
            }
            let mut row = vec!["?".to_owned(), name.to_owned()];
            row.extend(args.iter().cloned());
            seen.borrow_mut().push(row);
            Ok(None)
        })
    };
    (seen, vocabulary, unknown)
}

#[test]
fn pack_registration_keeps_constructed_colon_name_after_real_lookup() {
    crate::install_surfaces();
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual pack interpreter/vocabulary path, not a native name experiment.
    let (seen, mut vocabulary, unknown) = capture_all();
    vocabulary[0].0 = ":clock";
    run_pack_program(
        ":clock {A\0B}\n",
        &vocabulary,
        &unknown,
        &PackEvalConfig::default(),
    )
    .unwrap();
    assert_eq!(
        *seen.borrow(),
        [vec!["register".to_owned(), "A\0B".to_owned()]]
    );
}

#[test]
fn actual_pack_helpers_survive_the_selected_policy_without_random_state() {
    crate::install_surfaces();
    let (seen, vocabulary, unknown) = capture_all();
    run_pack_program(
        "register [dict get {key value} key] [string length abc] [expr {sqrt(9)}]",
        &vocabulary,
        &unknown,
        &PackEvalConfig::default(),
    )
    .expect("deterministic actual helpers retained");
    assert_eq!(seen.borrow()[0], vec!["register", "value", "3", "3.0"]);
}

#[test]
fn templated_registrations_run_and_capture_in_order() {
    crate::install_surfaces();
    let (seen, vocabulary, unknown) = capture_all();
    run_pack_program(
        "foreach x {a b c} { register $x }\n",
        &vocabulary,
        &unknown,
        &PackEvalConfig::default(),
    )
    .expect("runs");
    let rows: Vec<Vec<String>> = seen.borrow().clone();
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0], vec!["register".to_owned(), "a".to_owned()]);
    assert_eq!(rows[2], vec!["register".to_owned(), "c".to_owned()]);
}

#[test]
fn a_denied_command_fails_the_whole_program() {
    crate::install_surfaces();
    let (_seen, vocabulary, unknown) = capture_all();
    let failure = run_pack_program(
        "register ok\nclock seconds\nregister never\n",
        &vocabulary,
        &unknown,
        &PackEvalConfig::default(),
    )
    .expect_err("clock is denied");
    assert!(
        matches!(&failure, PackEvalFailure::Script(message) if message.contains("clock")),
        "{failure:?}"
    );
}

#[test]
fn the_command_budget_names_its_axis() {
    crate::install_surfaces();
    let (_seen, vocabulary, unknown) = capture_all();
    let failure = run_pack_program(
        "set i 0\nwhile {1} { register [incr i] }\n",
        &vocabulary,
        &unknown,
        &PackEvalConfig {
            budget: tcl_engine_api::Budget::of_commands(500)
                .with_wall_clock(std::time::Duration::from_secs(5)),
        },
    )
    .expect_err("the budget stops it");
    assert_eq!(failure, PackEvalFailure::Budget("command steps"));
}

/// `rand` is swept from the sandbox, so `rand()` meets `unknown` exactly as
/// a missing math function does in C Tcl (`tclsh9.0.4`: with
/// `proc unknown args {return U:$args}`, `expr {foo()}` is
/// `U:tcl::mathfunc::foo`). The unknown handler decides, so the random
/// generator is never reached and nothing is registered.
#[test]
fn rand_and_srand_reach_the_unknown_handler() {
    crate::install_surfaces();
    for call in ["rand()", "srand(1)"] {
        let (seen, vocabulary, unknown) = capture_all();
        let outcome = run_pack_program(
            &format!("register [expr {{{call}}}]\n"),
            &vocabulary,
            &unknown,
            &PackEvalConfig::default(),
        );
        assert!(
            matches!(&outcome, Err(PackEvalFailure::Script(message))
                if message.contains("tcl::mathfunc::") && message.contains("rand")),
            "{call}: {outcome:?}"
        );
        assert!(
            seen.borrow().iter().all(|row| row[0] != "register"),
            "{call}: no random value may be registered: {:?}",
            seen.borrow()
        );
    }
}
