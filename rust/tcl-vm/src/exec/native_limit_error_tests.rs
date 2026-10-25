// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn actual(profile: &str) -> Vm {
    crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find(profile).unwrap())
}

#[test]
fn expired_native_limit_publishes_original_result_before_error_frames() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(profile);
        let defined = vm
            .try_eval_source("proc budget_loop {} {while {1} {}}")
            .unwrap();
        assert_eq!(defined.code, Code::Ok);
        drop(defined);
        vm.set_time_limit_deadline(Some(0));
        let completion = vm.try_invoke_command("budget_loop", &[]).unwrap();
        assert_eq!(completion.code, Code::Error, "{profile}");
        assert_eq!(vm.limit_exceeded(), Some(crate::LimitKind::WallClock));
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"time limit exceeded"
        );
        let info = vm.error_info_value().unwrap();
        assert!(
            info.starts_with(b"time limit exceeded"),
            "{profile}: {info:?}"
        );
        assert!(
            info.windows(b"procedure".len())
                .any(|part| part == b"procedure")
        );
        assert!(
            vm.with_native_interp_result(|result| result.is_same_object(&completion.result))
                .unwrap()
        );
    }
}

#[test]
fn expired_native_limit_does_not_replace_an_existing_owner_refusal() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(profile);
        let _ = vm.refuse_host_command("original owner refusal".into());
        vm.set_time_limit_deadline(Some(0));
        let error = vm.try_eval_source("set forbidden_effect yes").unwrap_err();
        assert!(
            error.to_string().contains("original owner refusal"),
            "{profile}: {error}"
        );
    }
}
