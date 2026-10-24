// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original selected format instructions retain object identity and callback order.

use super::tests::interpreter;
use super::*;

#[test]
fn original_format_single_value_retains_its_actual_list_object() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(
            interp.eval_str(b"proc p {value} {format {%s} $value}"),
            Code::Ok
        );
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let value = obj::Owned::fresh(crate::list::new_list_obj_native(&[], protocol));
        let head = obj::Owned::fresh(new_string(b"p"));
        assert_eq!(
            interp.dispatch(&[head.as_ptr(), value.as_ptr()]),
            Code::Ok,
            "{profile}"
        );
        assert_eq!(interp.result_obj(), value.as_ptr(), "{profile}");
        let body = interp.proc_def(b"p").unwrap();
        let artifact = cache(body.body.as_ptr()).expect("original format artifact");
        assert!(artifact.scripts.values().flat_map(|script| &script.commands).any(|command| matches!(&command.operation, Operation::FormatStringConcat { operands, .. } if operands.len() == 1)));
        assert!(!interp.host_refusal_pending(), "{profile}");
    }
}

#[test]
fn original_format_values_run_once_in_order_and_error_stops_later_values() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(interp.eval_str(b"set ::seen {}; proc take {label} {lappend ::seen $label; return $label}; proc p {} {format {a%s%%b%s} [take A] [take B]}; p"), Code::Ok, "{profile}");
        assert_eq!(interp.result_bytes(), b"aA%bB", "{profile}");
        assert_eq!(interp.eval_str(b"set ::seen"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"A B", "{profile}");
        assert_eq!(
            interp.eval_str(b"set ::seen {}; proc q {} {format {%s%s} [error FIRST] [take B]}; q"),
            Code::Error,
            "{profile}"
        );
        assert_eq!(interp.result_bytes(), b"FIRST", "{profile}");
        assert_eq!(interp.eval_str(b"set ::seen"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"", "{profile}");
        assert!(!interp.host_refusal_pending(), "{profile}");
    }
}

#[test]
fn original_format_literal_sharing_follows_the_selected_c_compiler() {
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        assert_eq!(
            interp.eval_str(b"proc p {value} {format {L%sL} $value}"),
            Code::Ok
        );
        let procedure = interp.proc_def(b"p").unwrap();
        let artifact = interp
            .prepare_original_c_body(procedure.body.as_ptr(), GLOBAL, Some(&procedure))
            .unwrap()
            .expect("original format artifact");
        let operands = artifact
            .scripts
            .values()
            .flat_map(|script| &script.commands)
            .find_map(|command| match &command.operation {
                Operation::FormatStringConcat { operands, .. } => Some(operands),
                _ => None,
            })
            .unwrap();
        let [NamespaceOperand::Literal(left), NamespaceOperand::Original(2), NamespaceOperand::Literal(right)] =
            operands.as_slice()
        else {
            panic!("exact original format stack");
        };
        assert_eq!(
            artifact.literals.original(*left).unwrap()
                == artifact.literals.original(*right).unwrap(),
            profile != "tcl9.1",
            "{profile}"
        );
        assert!(!interp.host_refusal_pending(), "{profile}");
    }
}
