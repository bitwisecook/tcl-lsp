// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original public command-query output from retained reference executions.

use std::{cell::RefCell, io::Write, rc::Rc};

struct Output(Rc<RefCell<Vec<u8>>>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn compare(engine: &str, oracle: &[u8]) {
    let profile = crate::environment::profile_for_dialect(engine);
    let output = Rc::new(RefCell::new(Vec::new()));
    let mut vm = crate::native_fixture::interpreter_with_output(
        profile,
        Box::new(Output(Rc::clone(&output))),
    );
    let completion = vm
        .try_eval_source(include_str!(
            "../../../tcl-registry/tests/data/source_contract_vectors/namespace-command-query/input-portable.tcl"
        ))
        .unwrap();
    assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
    assert!(vm.refused_completion().is_none(), "{engine}");
    let actual = output.borrow();
    let (actual_identity, actual_rows) =
        actual.split_at(actual.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    let (oracle_identity, oracle_rows) =
        oracle.split_at(oracle.iter().position(|byte| *byte == b'\n').unwrap() + 1);
    if engine == "jim" {
        assert_eq!(actual_identity, b"IDENTITY 0.84\n");
        assert_eq!(oracle_identity, b"IDENTITY 0.84-9-g5bac7c9\n");
    } else {
        assert_eq!(
            actual_identity, oracle_identity,
            "{engine}: provider reporting"
        );
    }
    let rows = std::str::from_utf8(oracle_rows).unwrap();
    assert_eq!(rows.lines().count(), 33);
    let setup_and_context = [
        "alias-setup",
        "path-setup",
        "path-reverse-setup",
        "path-clear-setup",
        "export-setup",
        "import-setup",
        "colon-left-current",
        "colon-right-current",
    ];
    let mut queries = 0;
    for row in rows.lines() {
        let columns = tcl_syntax::list::split_list(row).unwrap();
        assert_eq!(columns.len(), 5);
        assert_eq!(columns[0], "OBSERVATION");
        if !setup_and_context.contains(&columns[1].as_ref()) {
            queries += 1;
        }
    }
    assert_eq!(queries, 25);
    assert_eq!(
        actual_rows, oracle_rows,
        "{engine}: exact original command-query output, including setup outcomes and equal display names"
    );
    eprintln!(
        "{engine}:25 actual query rows+6 setup outcomes+2 context displays; answer bytes are output only, never reconstructed command identities"
    );
}

macro_rules! queries {
    ($test:ident, $engine:literal, $provider:literal) => {
        #[test]
        fn $test() {
            compare(
                $engine,
                include_bytes!(concat!(
                    "../../../tcl-registry/tests/data/source_contract_vectors/namespace-command-query/portable/",
                    $provider,
                    "/stdout.bin"
                )),
            );
        }
    };
}
queries!(original_command_queries_c84, "tcl8.4", "8.4.20");
queries!(original_command_queries_c85, "tcl8.5", "8.5.19");
queries!(original_command_queries_c86, "tcl8.6", "8.6.18");
queries!(original_command_queries_c90, "tcl9.0", "9.0.4");
queries!(original_command_queries_c91, "tcl9.1", "9.1.0");
queries!(original_command_queries_jim, "jim", "jim");
