// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual alias-record queries, relocation and physical order from original C Tcl.

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
    let input = include_str!(
        "../../../../experiments/name-handling/namespace-origin-query/alias-registration-records/input.tcl"
    );
    let completion = vm.try_eval_source(input).unwrap_or_else(|error| {
        panic!(
            "{engine}: {error:?}; output before refusal:\n{}",
            String::from_utf8_lossy(&output.borrow())
        )
    });
    assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
    assert!(vm.refused_completion().is_none(), "{engine}");
    assert_eq!(std::str::from_utf8(oracle).unwrap().lines().count(), 27);
    assert_eq!(
        *output.borrow(),
        oracle,
        "{engine}: all 26 original observations, including hidden generations, record collisions and unsorted growth/removal/reinsertion order"
    );
}
macro_rules! records {
    ($test:ident, $engine:literal, $provider:literal) => {
        #[test]
        fn $test() {
            compare($engine, include_bytes!(concat!("../../../../experiments/name-handling/namespace-origin-query/alias-registration-records/", $provider, "/stdout.bin")));
        }
    };
}
records!(original_alias_records_c84, "tcl8.4", "8.4.20");
records!(original_alias_records_c85, "tcl8.5", "8.5.19");
records!(original_alias_records_c86, "tcl8.6", "8.6.18");
records!(original_alias_records_c90, "tcl9.0", "9.0.4");
records!(original_alias_records_c91, "tcl9.1", "9.1.0");
