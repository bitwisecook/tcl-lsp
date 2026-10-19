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

use super::*;

struct OpaqueResult;
impl NativeCommand for OpaqueResult {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        Completion::new(
            Code::Ok,
            Value::from_string_bytes(b"\xff".as_slice()),
            Value::empty(),
        )
    }
}
struct EarlierHost;
impl NativeCommand for EarlierHost {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        vm.refuse_host_command("original live evaluation Host".into())
    }
}

#[test]
fn original_live_result_projection_keeps_strict_bytes_earlier_eval_cause_and_next_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut session =
        LiveSession::new(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tcl"))
            .unwrap();
    session
        .vm_mut()
        .register_native_command("opaqueOriginalResult", Rc::new(OpaqueResult));
    session
        .vm_mut()
        .register_native_command("earlierOriginalHost", Rc::new(EarlierHost));
    assert_eq!(
        session.eval("opaqueOriginalResult"),
        Err(SessionError::Projection(
            tcl_syntax::raw_string::UnicodeAccessError {
                valid_up_to: 0,
                error_len: Some(1)
            }
        ))
    );
    assert_eq!(session.eval("return NEXT"), Ok("NEXT".into()));
    let error = session
        .eval("catch {earlierOriginalHost}; return LATER")
        .unwrap_err();
    assert!(
        matches!(error, SessionError::Execution(NativeExecutionError::HostCommandRefusal(actual)) if actual.reason == "original live evaluation Host")
    );
    assert_eq!(session.eval("return AGAIN"), Ok("AGAIN".into()));
}

#[test]
fn original_event_enrolment_refuses_utf8_before_domain_or_later_operand() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    for first in [true, false] {
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(tcl_vm::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("explicit original C9 core bootstrap");
        let selection = Rc::new(MeasuredEventNames {
            build: Cell::new(None),
            next_domain: Cell::new(7),
        });
        vm.register_native_command(
            "originalEnrol",
            Rc::new(EnrollEventHook(Rc::clone(&selection))),
        );
        let values = if first {
            [Value::from_string_bytes(b"\xff".as_slice()), Value::int(23)]
        } else {
            [
                Value::string("unselectedChild"),
                Value::from_string_bytes(b"\xff".as_slice()),
            ]
        };
        let counts = values
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>();
        let error = vm
            .try_invoke_command("originalEnrol", &values)
            .expect_err("strict enrolment input refuses");
        assert!(matches!(
            error,
            NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: 0,
                        error_len: Some(1)
                    }
                )
            )
        ));
        assert_eq!(selection.next_domain.get(), 7);
        assert_eq!(
            values
                .iter()
                .map(Value::native_object_reference_count)
                .collect::<Vec<_>>(),
            counts
        );
        if first {
            assert!(values[1].resident_string_bytes().is_none());
        }
    }
}
