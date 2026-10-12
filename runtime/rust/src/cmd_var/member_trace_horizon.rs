// SPDX-License-Identifier: AGPL-3.0-or-later
//! Whole original source outcomes do not grant private cell identity.

use crate::{counters, interp::Interp};

#[test]
fn original_array_member_trace_horizons_match_six_native_source_results() {
    // naming.variable.original-array-member-trace-retirement-horizon
    // docs/design/analysis/name-resolution-proofs/variable-original-array-member-trace-retirement-horizon.md
    // Compare all original completions and complete public log values. Jim's
    // original unavailable trace API is retained as its own guest error.
    macro_rules! provider {
        ($engine:literal, $version:literal) => {
            ($engine, [
                include_str!(concat!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/", $version,
                    "/variable_array_unset_root_changes_original_member_trace_via_alias/stdout")),
                include_str!(concat!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/", $version,
                    "/variable_array_unset_first_member_changes_later_member_trace_via_alias/stdout")),
                include_str!(concat!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/", $version,
                    "/variable_array_unset_captured_member_prefixes_ignore_recreated_root/stdout")),
            ])
        };
    }
    let sources = [
        include_bytes!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/variable_array_unset_root_changes_original_member_trace_via_alias.tcl").as_slice(),
        include_bytes!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/variable_array_unset_first_member_changes_later_member_trace_via_alias.tcl").as_slice(),
        include_bytes!("../../../../rust/tcl-registry/tests/data/native_array_member_trace_horizon343/variable_array_unset_captured_member_prefixes_ignore_recreated_root.tcl").as_slice(),
    ];
    for (engine, outputs) in [
        provider!("tcl8.4", "8.4.20"),
        provider!("tcl8.5", "8.5.19"),
        provider!("tcl8.6", "8.6.18"),
        provider!("tcl9.0", "9.0.4"),
        provider!("tcl9.1", "9.1.0"),
        provider!("jim", "jim"),
    ] {
        for (source, stdout) in sources.iter().zip(outputs) {
            let (expected_code, original) = stdout
                .lines()
                .find_map(|line| line.strip_prefix("ORIGINAL|"))
                .unwrap()
                .split_once('|')
                .unwrap();
            let expected: Vec<u8> = original
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let code = interp.eval_str(source);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(code.as_int().to_string(), expected_code, "{engine}");
                assert_eq!(interp.result_bytes(), expected, "{engine}");
            }
            assert_eq!(counters::finalize(), 0, "{engine}");
            assert_eq!(counters::double_free_count(), 0, "{engine}");
        }
    }
}
