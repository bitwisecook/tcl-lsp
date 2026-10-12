// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite ASCII source observations, independent of event runtime capability.

use super::{
    ArgRole, InvocationArguments, InvocationDialect, NativeVwaitForm, NativeVwaitProtocol,
};
use tcl_dialect::TclVersion;

fn providers() -> Vec<(&'static str, &'static str, InvocationDialect)> {
    TclVersion::ALL
        .into_iter()
        .zip(["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"])
        .map(|(version, directory)| {
            (
                directory,
                directory,
                InvocationDialect::for_version(version),
            )
        })
        .chain([(
            "jim",
            "0.84-9-g5bac7c9",
            InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment("jim").unit_profile(),
            ),
        )])
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn observation(provider: &str, version: &str, case: &str) -> (u8, String) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/native_vwait_original")
        .join(provider)
        .join(format!("{case}.stdout.tsv"));
    let contents = std::fs::read_to_string(path).expect("retained native source observation");
    let mut rows = contents.lines();
    assert_eq!(
        rows.next(),
        Some(
            format!(
                "V|actual-provider|0|{}|{}",
                version.len(),
                hex(version.as_bytes())
            )
            .as_str()
        ),
        "{provider}/{case}: actual patchlevel"
    );
    let fields = rows
        .next()
        .expect("one native result")
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 5, "{provider}/{case}");
    assert_eq!(&fields[..2], &["R", case]);
    let length = fields[3]
        .parse::<usize>()
        .expect("counted native result length");
    assert_eq!(fields[4].len(), length * 2);
    assert!(fields[4].bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(rows.next().is_none(), "one fresh process/case");
    (
        fields[2].parse().expect("native completion code"),
        fields[4].to_owned(),
    )
}

#[test]
fn original_vwait_basic_boundaries_match_twenty_four_native_source_rows() {
    // naming.event.vwait-native-basic-name-boundaries
    // docs/design/analysis/name-resolution-proofs/event-vwait-native-basic-name-boundaries.md
    // naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
    for (provider, version, dialect) in providers() {
        let protocol = dialect.native_vwait_protocol().unwrap();
        for (case, name) in [
            ("basic", "x"),
            ("lone-all", "-all"),
            ("lone-signal", "-signal"),
            ("lone-end", "--"),
        ] {
            let observed = observation(provider, version, case);
            assert_eq!(observed.0, 0, "{provider}/{case}");
            assert_eq!(observed.1, if case == "lone-end" { "" } else { "4e4557" });
            // Equal lone -- results do not erase the independently selected
            // C9 parser exception. These captures supply no raw-zero argv.
            let extended =
                case == "lone-end" && matches!(protocol, NativeVwaitProtocol::CExtended(_));
            assert_eq!(
                protocol.form(1, |_| Ok::<_, ()>(name.as_bytes())).unwrap(),
                if extended {
                    NativeVwaitForm::Extended
                } else {
                    NativeVwaitForm::Basic
                },
                "{provider}/{case}"
            );
            assert_eq!(
                protocol.source_roles(InvocationArguments::literals(&[name]).with_dialect(dialect)),
                Some(if extended {
                    vec![]
                } else {
                    vec![(0, ArgRole::VarWrite)]
                }),
                "{provider}/{case}"
            );
        }
    }
}

#[test]
fn original_vwait_extended_boundaries_match_sixty_native_source_rows() {
    // naming.event.vwait-native-extended-argv-boundaries
    // docs/design/analysis/name-resolution-proofs/event-vwait-native-extended-argv-boundaries.md
    // naming.event.original-vwait-operand-boundaries
    // docs/design/analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md
    use ArgRole::{Body, VarWrite};
    let cases: &[(
        &str,
        &[&str],
        Option<Vec<(u8, ArgRole)>>,
        Option<Vec<(u8, ArgRole)>>,
    )] = &[
        (
            "end-name",
            &["--", "--"],
            Some(vec![(1, VarWrite)]),
            Some(vec![(0, VarWrite), (1, Body)]),
        ),
        (
            "option-variable",
            &["-variable", "x", "-timeout", "100"],
            Some(vec![(1, VarWrite)]),
            None,
        ),
        (
            "abbreviated-variable",
            &["-v", "x", "-t", "100"],
            Some(vec![(1, VarWrite)]),
            None,
        ),
        (
            "timeout-or-jim-body",
            &["-timeout", "1"],
            Some(vec![]),
            Some(vec![(0, VarWrite), (1, Body)]),
        ),
        (
            "signal-name",
            &["-signal", "x"],
            None,
            Some(vec![(1, VarWrite)]),
        ),
        (
            "name-body",
            &["x", "break"],
            Some(vec![(0, VarWrite), (1, VarWrite)]),
            Some(vec![(0, VarWrite), (1, Body)]),
        ),
        (
            "signal-name-body",
            &["-signal", "x", "break"],
            None,
            Some(vec![(1, VarWrite), (2, Body)]),
        ),
        (
            "abbreviation-is-jim-name",
            &["-sig", "x"],
            None,
            Some(vec![(0, VarWrite), (1, Body)]),
        ),
        (
            "missing-option-value",
            &["-variable", "x", "-timeout"],
            None,
            None,
        ),
        (
            "missing-channel",
            &["-readable", "no_such_original_channel"],
            Some(vec![]),
            Some(vec![(0, VarWrite), (1, Body)]),
        ),
    ];
    for (provider, version, dialect) in providers() {
        let protocol = dialect.native_vwait_protocol().unwrap();
        for (case, arguments, c_roles, jim_roles) in cases {
            let (code, result) = observation(provider, version, case);
            let form = protocol
                .form(arguments.len(), |index| {
                    Ok::<_, ()>(arguments[index].as_bytes())
                })
                .unwrap();
            let native_arity = code == 1 && result == hex(protocol.wrong_arguments().as_bytes());
            assert_eq!(
                form == NativeVwaitForm::WrongArity,
                native_arity,
                "{provider}/{case}"
            );
            assert_ne!(form, NativeVwaitForm::Basic, "{provider}/{case}");
            let expected = match protocol {
                NativeVwaitProtocol::CLegacy => None,
                NativeVwaitProtocol::CExtended(_) => c_roles.clone(),
                NativeVwaitProtocol::Jim084 => jim_roles.clone(),
            };
            assert_eq!(
                protocol
                    .source_roles(InvocationArguments::literals(arguments).with_dialect(dialect)),
                expected,
                "{provider}/{case}"
            );
            // Extended marks the missing Rust capability, including native
            // option/channel errors. Source roles do not certify execution.
            if matches!(protocol, NativeVwaitProtocol::CExtended(_)) && *case == "missing-channel" {
                assert_eq!(code, 1);
                assert_eq!(
                    result,
                    hex(b"can not find channel named \"no_such_original_channel\"")
                );
                assert_eq!(expected, Some(vec![]));
            }
        }
    }
}
