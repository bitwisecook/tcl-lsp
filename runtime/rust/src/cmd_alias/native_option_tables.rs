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

//! Original root and child interpreter option-table diagnostics.

macro_rules! source {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
            $name,
            ".tcl"
        ))
    };
}
macro_rules! rows {
    ($version:literal) => {
        [
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/root-empty/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/root-missing/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/root-c-prefix/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/root-e-prefix/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/child-missing/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/child-empty/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/child-h-prefix/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_interpreter_option_tables210/",
                $version,
                "/child-hi-prefix/stdout"
            )),
        ]
    };
}

fn compare(
    interp: &mut crate::interp::Interp,
    source: &[u8],
    expected: &str,
    phase: &str,
    context: &str,
) {
    let code = interp.eval_str(source);
    assert!(
        !interp.host_refusal_pending(),
        "{context}/{phase}: {:?}",
        interp.native_access_refusal()
    );
    let fields: Vec<_> = expected
        .lines()
        .find(|row| row.split('|').next() == Some(phase))
        .unwrap()
        .split('|')
        .collect();
    let result = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{context}/{phase}: {:?}",
        interp.result_bytes()
    );
    assert_eq!(interp.result_bytes(), result, "{context}/{phase}");
}

#[test]
fn interp_subcommand_words_resolve_like_original_native_option_tables() {
    // Native proof: naming.interpreter.original-option-table-diagnostics
    // docs/design/analysis/name-resolution-proofs/interpreter-original-option-table-diagnostics.md
    // Counted source and independent child prelude match original public
    // diagnostics. Jim's unavailable C child API is not substituted.
    crate::counters::reset();
    let sources: [&[u8]; 8] = [
        source!("root-empty"),
        source!("root-missing"),
        source!("root-c-prefix"),
        source!("root-e-prefix"),
        source!("child-missing"),
        source!("child-empty"),
        source!("child-h-prefix"),
        source!("child-hi-prefix"),
    ];
    for (engine, rows) in [
        ("tcl8.4", rows!("8.4.20")),
        ("tcl8.5", rows!("8.5.19")),
        ("tcl8.6", rows!("8.6.18")),
        ("tcl9.0", rows!("9.0.4")),
        ("tcl9.1", rows!("9.1.0")),
        ("jim", rows!("jim")),
    ] {
        for (index, (source, rows)) in sources.iter().zip(rows).enumerate() {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let context = format!("{engine}/{index}");
            if index >= 4 {
                compare(
                    &mut interp,
                    source!("child-prelude"),
                    rows,
                    "PRELUDE",
                    &context,
                );
            }
            compare(&mut interp, source, rows, "ORIGINAL", &context);
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
