// SPDX-License-Identifier: AGPL-3.0-or-later
//! Whole original inventory and current helper controls.

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields: Vec<_> = rows
        .lines()
        .find(|line| line.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect();
    (
        fields[1].parse().unwrap(),
        fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

type OriginalControl = (&'static str, &'static [u8], [&'static str; 6]);

const CONTROLS: &[OriginalControl] = &[
    ("native_jim_info_command_inventory254/original-all-option-current-namespace", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-all-option-current-namespace.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-all-option-current-namespace/stdout"),
    ]),
    ("native_jim_info_command_inventory254/original-all-option-qualified-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-all-option-qualified-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-all-option-qualified-pattern/stdout"),
    ]),
    ("native_jim_info_command_inventory254/original-abbreviated-all-option", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-abbreviated-all-option.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-abbreviated-all-option/stdout"),
    ]),
    ("native_jim_info_command_inventory254/original-option-without-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-option-without-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-option-without-pattern/stdout"),
    ]),
    ("native_jim_info_command_inventory254/original-pattern-and-extra-argument", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-pattern-and-extra-argument.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-pattern-and-extra-argument/stdout"),
    ]),
    ("native_jim_info_command_inventory254/original-absent-and-literal-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/original-absent-and-literal-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.4.20/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.5.19/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/8.6.18/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.0.4/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/9.1.0/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_command_inventory254/jim/original-absent-and-literal-pattern/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-all-option-current-namespace", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-all-option-current-namespace.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-all-option-current-namespace/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-all-option-current-namespace/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-all-option-qualified-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-all-option-qualified-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-all-option-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-all-option-qualified-pattern/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-abbreviated-all-option", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-abbreviated-all-option.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-abbreviated-all-option/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-abbreviated-all-option/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-option-without-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-option-without-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-option-without-pattern/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-pattern-and-extra-argument", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-pattern-and-extra-argument.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-pattern-and-extra-argument/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-pattern-and-extra-argument/stdout"),
    ]),
    ("native_jim_rooted_info_command_inventory256/original-absent-and-literal-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/original-absent-and-literal-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.4.20/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.5.19/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/8.6.18/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.0.4/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/9.1.0/original-absent-and-literal-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_rooted_info_command_inventory256/jim/original-absent-and-literal-pattern/stdout"),
    ]),
    ("native_jim_core_command_inventory257/root-exact-all-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/root-exact-all-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/root-exact-all-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/root-exact-all-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/root-exact-all-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/root-exact-all-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/root-exact-all-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/root-exact-all-pattern/stdout"),
    ]),
    ("native_jim_core_command_inventory257/root-plain-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/root-plain-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/root-plain-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/root-plain-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/root-plain-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/root-plain-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/root-plain-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/root-plain-pattern/stdout"),
    ]),
    ("native_jim_core_command_inventory257/root-all-qualified-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/root-all-qualified-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/root-all-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/root-all-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/root-all-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/root-all-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/root-all-qualified-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/root-all-qualified-pattern/stdout"),
    ]),
    ("native_jim_core_command_inventory257/root-option-without-pattern", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/root-option-without-pattern.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/root-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/root-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/root-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/root-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/root-option-without-pattern/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/root-option-without-pattern/stdout"),
    ]),
    ("native_jim_core_command_inventory257/root-original-option-arity", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/root-original-option-arity.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/root-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/root-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/root-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/root-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/root-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/root-original-option-arity/stdout"),
    ]),
    ("native_jim_core_command_inventory257/namespace-explicit-nons-all-patterns", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/namespace-explicit-nons-all-patterns.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/namespace-explicit-nons-all-patterns/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/namespace-explicit-nons-all-patterns/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/namespace-explicit-nons-all-patterns/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/namespace-explicit-nons-all-patterns/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/namespace-explicit-nons-all-patterns/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/namespace-explicit-nons-all-patterns/stdout"),
    ]),
    ("native_jim_core_command_inventory257/namespace-explicit-nons-absent-and-all", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/namespace-explicit-nons-absent-and-all.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/namespace-explicit-nons-absent-and-all/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/namespace-explicit-nons-absent-and-all/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/namespace-explicit-nons-absent-and-all/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/namespace-explicit-nons-absent-and-all/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/namespace-explicit-nons-absent-and-all/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/namespace-explicit-nons-absent-and-all/stdout"),
    ]),
    ("native_jim_core_command_inventory257/namespace-explicit-nons-original-option-arity", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/namespace-explicit-nons-original-option-arity.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.4.20/namespace-explicit-nons-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.5.19/namespace-explicit-nons-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/8.6.18/namespace-explicit-nons-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.0.4/namespace-explicit-nons-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/9.1.0/namespace-explicit-nons-original-option-arity/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_core_command_inventory257/jim/namespace-explicit-nons-original-option-arity/stdout"),
    ]),
    ("native_jim_info_helper_forwarding258/original-helper-formals-and-body", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/original-helper-formals-and-body.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.4.20/original-helper-formals-and-body/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.5.19/original-helper-formals-and-body/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.6.18/original-helper-formals-and-body/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.0.4/original-helper-formals-and-body/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.1.0/original-helper-formals-and-body/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/jim/original-helper-formals-and-body/stdout"),
    ]),
    ("native_jim_info_helper_forwarding258/original-helper-replacement-caller-and-vector", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/original-helper-replacement-caller-and-vector.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.4.20/original-helper-replacement-caller-and-vector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.5.19/original-helper-replacement-caller-and-vector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.6.18/original-helper-replacement-caller-and-vector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.0.4/original-helper-replacement-caller-and-vector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.1.0/original-helper-replacement-caller-and-vector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/jim/original-helper-replacement-caller-and-vector/stdout"),
    ]),
    ("native_jim_info_helper_forwarding258/original-root-qualified-pattern-forwards-current-helper", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/original-root-qualified-pattern-forwards-current-helper.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.4.20/original-root-qualified-pattern-forwards-current-helper/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.5.19/original-root-qualified-pattern-forwards-current-helper/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/8.6.18/original-root-qualified-pattern-forwards-current-helper/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.0.4/original-root-qualified-pattern-forwards-current-helper/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/9.1.0/original-root-qualified-pattern-forwards-current-helper/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_info_helper_forwarding258/jim/original-root-qualified-pattern-forwards-current-helper/stdout"),
    ]),
];

#[test]
fn original_jim_inventory_and_current_helper_sources_match_all_native_columns() {
    // Native proof: naming.info.original-command-inventory-option-and-scope
    // docs/design/analysis/name-resolution-proofs/info-original-command-inventory-option-and-scope.md
    // Native proof: naming.info.original-rooted-core-command-inventory-option-and-scope
    // docs/design/analysis/name-resolution-proofs/info-original-rooted-core-command-inventory-option-and-scope.md
    // Native proof: naming.info.original-root-and-explicit-nons-command-inventory
    // docs/design/analysis/name-resolution-proofs/info-original-root-and-explicit-nons-command-inventory.md
    // Native proof: naming.info.original-namespace-helper-declaration-and-current-forwarding
    // docs/design/analysis/name-resolution-proofs/info-original-namespace-helper-declaration-and-current-forwarding.md
    // Complete public source results are independent of private table/cache identity.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_library(&mut interp,
                tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble);
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 138);
}

const ALIAS_CONTROLS: &[OriginalControl] = &[
    ("original-prefix-query", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/original-prefix-query.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.4.20/original-prefix-query/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.5.19/original-prefix-query/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.6.18/original-prefix-query/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.0.4/original-prefix-query/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.1.0/original-prefix-query/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim/original-prefix-query/stdout"),
    ]),
    ("original-missing-and-nonalias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/original-missing-and-nonalias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.4.20/original-missing-and-nonalias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.5.19/original-missing-and-nonalias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.6.18/original-missing-and-nonalias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.0.4/original-missing-and-nonalias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.1.0/original-missing-and-nonalias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim/original-missing-and-nonalias/stdout"),
    ]),
    ("original-flat-alias-inventory", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/original-flat-alias-inventory.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.4.20/original-flat-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.5.19/original-flat-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.6.18/original-flat-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.0.4/original-flat-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.1.0/original-flat-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim/original-flat-alias-inventory/stdout"),
    ]),
    ("original-namespace-alias-inventory", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/original-namespace-alias-inventory.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.4.20/original-namespace-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.5.19/original-namespace-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/8.6.18/original-namespace-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.0.4/original-namespace-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/9.1.0/original-namespace-alias-inventory/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_introspection274/jim/original-namespace-alias-inventory/stdout"),
    ]),
];

fn assert_original_alias_public_window(case: &str, observed: &[u8], captured: &[u8], jim: bool) {
    if !jim || case != "original-namespace-alias-inventory" {
        assert_eq!(observed, captured, "{case}");
        return;
    }
    // Native274 retains this whole raw result. Only the last inventory field
    // is compared as an alias member multiset: its actual hash iteration order
    // is not supplied by the sorted reporting interface. Codes, the helper's
    // exact error and all other fields remain byte-exact; no physical table
    // capacity, seed or order is inferred from this comparison.
    let fields = |bytes: &[u8]| {
        tcl_syntax::list::split_native_list_bytes(
            bytes,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap()
        .into_iter()
        .map(|field| field.into_owned())
        .collect::<Vec<_>>()
    };
    let actual = fields(observed);
    let expected = fields(captured);
    assert_eq!(actual.len(), 4);
    assert_eq!(expected.len(), 4);
    assert_eq!(actual[..3], expected[..3], "{case}");
    let mut actual_names = fields(&actual[3]);
    let mut expected_names = fields(&expected[3]);
    actual_names.sort();
    expected_names.sort();
    assert_eq!(actual_names, expected_names, "{case}");
}

#[test]
fn original_alias_queries_and_inventory_compare_all_24_native_windows() {
    // naming.alias.jim-original-query-and-inventory
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-query-and-inventory.md
    // Original source bytes and captured streams are immutable. The last Jim
    // namespace inventory field is an explicitly bounded member comparison;
    // all other result windows and original completion codes are exact.
    // No native alias prefix pointer/cache identity follows from these bytes.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in ALIAS_CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            assert_original_alias_public_window(
                case,
                &interp.result_bytes(),
                &expected_result,
                *engine == "jim",
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 24);
}

const ALIAS_EXTENT_CONTROLS: &[OriginalControl] = &[
    ("original-missing-alias-diagnostic-extent", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-missing-alias-diagnostic-extent.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-missing-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-missing-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-missing-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-missing-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-missing-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-missing-alias-diagnostic-extent/stdout"),
    ]),
    ("original-nonalias-alias-diagnostic-extent", include_bytes!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/original-nonalias-alias-diagnostic-extent.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.4.20/original-nonalias-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.5.19/original-nonalias-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/8.6.18/original-nonalias-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.0.4/original-nonalias-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/9.1.0/original-nonalias-alias-diagnostic-extent/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/jim/original-nonalias-alias-diagnostic-extent/stdout"),
    ]),
];

#[test]
fn original_alias_diagnostic_extents_match_all_12_native_public_windows() {
    // naming.alias.jim-original-alias-diagnostic-name-extent
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-alias-diagnostic-name-extent.md
    // Whole unchanged ASCII source programs produce the name bytes through
    // binary format H*: counted NUL, UTF8 e-acute, and invalid FF are runtime
    // bytes, not configuration/source Unicode. The original C branches report
    // NOT_APPLICABLE rather than invoking a different alias API.
    // Only the byte-producing command adapter is installed explicitly for Jim;
    // no complete distribution, table identity or helper availability is granted.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in ALIAS_EXTENT_CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if *engine == "jim" {
                crate::cmd_binary::install(&mut interp);
            }
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 12);
}

const PROCEDURE_USAGE_CONTROLS: &[OriginalControl] = &[
    ("original-called-name-nul-usage", include_bytes!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/original-called-name-nul-usage.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20/original-called-name-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19/original-called-name-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18/original-called-name-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4/original-called-name-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0/original-called-name-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim/original-called-name-nul-usage/stdout"),
    ]),
    ("original-required-formal-nul-usage", include_bytes!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/original-required-formal-nul-usage.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20/original-required-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19/original-required-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18/original-required-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4/original-required-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0/original-required-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim/original-required-formal-nul-usage/stdout"),
    ]),
    ("original-default-formal-nul-usage", include_bytes!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/original-default-formal-nul-usage.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20/original-default-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19/original-default-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18/original-default-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4/original-default-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0/original-default-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim/original-default-formal-nul-usage/stdout"),
    ]),
    ("original-reference-formal-nul-usage", include_bytes!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/original-reference-formal-nul-usage.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.4.20/original-reference-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.5.19/original-reference-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/8.6.18/original-reference-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.0.4/original-reference-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/9.1.0/original-reference-formal-nul-usage/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_procedure_usage_extents288/jim/original-reference-formal-nul-usage/stdout"),
    ]),
];

#[test]
fn original_procedure_usage_extents_match_all_24_native_public_windows() {
    // naming.procedure.original-called-name-and-formal-usage-extents
    // docs/design/analysis/name-resolution-proofs/procedure-original-called-name-and-formal-usage-extents.md
    // Four unchanged ASCII source programs produce their NUL through binary
    // format H*. Name/formal lengths, definition completion/result, and caught
    // call completion/message stay separate fields in each whole native result.
    // C providers execute their original APIs: no unavailable branch substitutes
    // for an actual definition or invocation. These observations prove no native
    // formal object/header/table identity or internal CString conversion stage.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in PROCEDURE_USAGE_CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // Explicit binary command capability matches only the byte-producing
            // source dependency; no complete Jim distribution is asserted.
            if *engine == "jim" {
                crate::cmd_binary::install(&mut interp);
            }
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 24);
}

const INFO_DISPATCH_CONTROLS: &[OriginalControl] = &[
    ("original-info-no-selector", include_bytes!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/original-info-no-selector.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.4.20/original-info-no-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.5.19/original-info-no-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.6.18/original-info-no-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.0.4/original-info-no-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.1.0/original-info-no-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/jim/original-info-no-selector/stdout"),
    ]),
    ("original-info-empty-selector", include_bytes!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/original-info-empty-selector.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.4.20/original-info-empty-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.5.19/original-info-empty-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/8.6.18/original-info-empty-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.0.4/original-info-empty-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/9.1.0/original-info-empty-selector/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_original_dispatch292/jim/original-info-empty-selector/stdout"),
    ]),
];

#[test]
fn original_info_dispatch_matches_all_12_native_public_windows() {
    // naming.info.original-missing-and-empty-selector-dispatch
    // docs/design/analysis/name-resolution-proofs/info-original-missing-and-empty-selector-dispatch.md
    // Both original ASCII sources catch the original invocation and retain its
    // code and complete message as separate fields. Every provider executes its
    // genuine public info command; no unavailable branch replaces a result.
    // Root info uses its authentic core dispatcher, without a library helper
    // substitution. These windows grant no private command/table/header identity.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in INFO_DISPATCH_CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 12);
}
