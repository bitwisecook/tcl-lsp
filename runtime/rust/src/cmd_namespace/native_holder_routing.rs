// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original source controls for current versus rooted command holders.

struct Control {
    source: &'static [u8],
    prelude: Option<&'static [u8]>,
    query: Option<&'static [u8]>,
}

macro_rules! source {
    ($file:literal) => {
        include_bytes!(concat!(
            "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
            $file
        ))
    };
}
macro_rules! rows {
    ($version:literal) => {
        [
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
                $version,
                "/deferred-original/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
                $version,
                "/recreated-object-private-dispatcher/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
                $version,
                "/synchronous-create/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
                $version,
                "/synchronous-replace/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_namespace_holder_routing202/",
                $version,
                "/deferred-alias-relative-rooted/stdout"
            )),
        ]
    };
}

const CONTROLS: [Control; 5] = [
    Control {
        source: source!("deferred-original.tcl"),
        prelude: None,
        query: None,
    },
    Control {
        source: source!("recreated-object-private-dispatcher.tcl"),
        prelude: None,
        query: None,
    },
    Control {
        source: source!("synchronous-create.tcl"),
        prelude: Some(source!("synchronous-create.prelude.tcl")),
        query: Some(source!("synchronous-create.query.tcl")),
    },
    Control {
        source: source!("synchronous-replace.tcl"),
        prelude: Some(source!("synchronous-replace.prelude.tcl")),
        query: Some(source!("synchronous-replace.query.tcl")),
    },
    Control {
        source: source!("deferred-alias-relative-rooted.tcl"),
        prelude: None,
        query: None,
    },
];

fn expected(rows: &str, label: &str) -> (i64, Vec<u8>) {
    let fields: Vec<_> = rows
        .lines()
        .find(|row| row.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect();
    let bytes = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    (fields[1].parse().unwrap(), bytes)
}

fn assert_phase(
    interp: &mut crate::interp::Interp,
    source: &[u8],
    rows: &str,
    label: &str,
    context: &str,
) {
    let code = interp.eval_str(source);
    assert!(
        !interp.host_refusal_pending(),
        "{context}/{label}: refusal={:?}, admission={:?}",
        interp.native_access_refusal(),
        interp.native_compilation_admission_error()
    );
    let (expected_code, bytes) = expected(rows, label);
    assert_eq!(
        code.as_int(),
        expected_code,
        "{context}/{label}: {:?}",
        interp.result_bytes()
    );
    assert_eq!(interp.result_bytes(), bytes, "{context}/{label}");
}

#[test]
fn original_command_holder_source_controls_match_all_native_providers() {
    // Native proof: naming.namespace.original-command-holder-routing
    // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
    // These are original public source completions; no private token or table
    // identity is inferred from reported command spellings or enumeration.
    crate::counters::reset();
    for (engine, rows) in [
        ("tcl8.4", rows!("8.4.20")),
        ("tcl8.5", rows!("8.5.19")),
        ("tcl8.6", rows!("8.6.18")),
        ("tcl9.0", rows!("9.0.4")),
        ("tcl9.1", rows!("9.1.0")),
        ("jim", rows!("jim")),
    ] {
        for (index, (control, rows)) in CONTROLS.iter().zip(rows).enumerate() {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // The native provider starts with its distribution extensions loaded.
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            let context = format!("{engine}/{index}");
            if let Some(prelude) = control.prelude {
                assert_phase(&mut interp, prelude, rows, "PRELUDE", &context);
            }
            assert_phase(&mut interp, control.source, rows, "ORIGINAL", &context);
            if let Some(query) = control.query {
                assert_phase(&mut interp, query, rows, "QUERY", &context);
            }
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

macro_rules! ensemble_source {
    ($file:literal) => {
        include_bytes!(concat!(
            "../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/",
            $file
        ))
    };
}
macro_rules! ensemble_rows {
    ($version:literal) => {
        [
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/",
                $version,
                "/default-live/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/",
                $version,
                "/relative-qualified-live/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/",
                $version,
                "/default-retained-parent/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/",
                $version,
                "/relative-retained-parent/stdout"
            )),
        ]
    };
}

#[test]
fn original_ensemble_holder_source_controls_match_all_native_providers() {
    // Native proof: naming.ensemble.original-command-holder-routing
    // docs/design/analysis/name-resolution-proofs/ensemble-original-command-holder-routing.md
    // Original source, prelude and query evaluations remain independent. Public
    // completions grant no private namespace token or registration-table identity.
    crate::counters::reset();
    let sources: [&[u8]; 4] = [
        ensemble_source!("default-live.tcl"),
        ensemble_source!("relative-qualified-live.tcl"),
        ensemble_source!("default-retained-parent.tcl"),
        ensemble_source!("relative-retained-parent.tcl"),
    ];
    for (engine, rows) in [
        ("tcl8.4", ensemble_rows!("8.4.20")),
        ("tcl8.5", ensemble_rows!("8.5.19")),
        ("tcl8.6", ensemble_rows!("8.6.18")),
        ("tcl9.0", ensemble_rows!("9.0.4")),
        ("tcl9.1", ensemble_rows!("9.1.0")),
        ("jim", ensemble_rows!("jim")),
    ] {
        for (index, (source, rows)) in sources.iter().zip(rows).enumerate() {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_library(
                &mut interp,
                tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble,
            );
            let context = format!("{engine}/ensemble/{index}");
            assert_phase(
                &mut interp,
                ensemble_source!("holder-prelude.tcl"),
                rows,
                "PRELUDE",
                &context,
            );
            assert_phase(&mut interp, source, rows, "ORIGINAL", &context);
            if index >= 2 {
                assert_phase(
                    &mut interp,
                    ensemble_source!("retained-query.tcl"),
                    rows,
                    "QUERY",
                    &context,
                );
            }
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

macro_rules! jim_library_source {
    ($file:literal) => {
        include_bytes!(concat!(
            "../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/",
            $file
        ))
    };
}
macro_rules! jim_library_rows {
    ($version:literal) => {
        [
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/",
                $version,
                "/original-library-formals-and-bodies/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/",
                $version,
                "/global-helper-replacement/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/",
                $version,
                "/namespace-local-helper-replacement/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_namespace_distribution236/",
                $version,
                "/original-unsupported-and-arity/stdout"
            )),
        ]
    };
}

#[test]
fn original_jim_namespace_library_sources_match_all_native_provider_results() {
    // Native proof: naming.namespace.jim-original-scripted-helper-availability-and-forwarding
    // docs/design/analysis/name-resolution-proofs/namespace-jim-original-scripted-helper-availability-and-forwarding.md
    // These whole original sources compare public results, including actual
    // helper availability. They grant no native core inventory, private token,
    // frame identity or compiler admission from stored library source.
    let sources: [&[u8]; 4] = [
        jim_library_source!("original-library-formals-and-bodies.tcl"),
        jim_library_source!("global-helper-replacement.tcl"),
        jim_library_source!("namespace-local-helper-replacement.tcl"),
        jim_library_source!("original-unsupported-and-arity.tcl"),
    ];
    for (engine, rows) in [
        ("tcl8.4", jim_library_rows!("8.4.20")),
        ("tcl8.5", jim_library_rows!("8.5.19")),
        ("tcl8.6", jim_library_rows!("8.6.18")),
        ("tcl9.0", jim_library_rows!("9.0.4")),
        ("tcl9.1", jim_library_rows!("9.1.0")),
        ("jim", jim_library_rows!("jim")),
    ] {
        for (index, (source, rows)) in sources.iter().zip(rows).enumerate() {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_library(
                &mut interp,
                tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble,
            );
            let context = format!("{engine}/namespace-library/{index}");
            assert_phase(&mut interp, source, rows, "ORIGINAL", &context);
        }
    }
}

macro_rules! rooted_factory_source {
    ($file:literal) => {
        include_bytes!(concat!(
            "../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/",
            $file
        ))
    };
}
macro_rules! rooted_factory_rows {
    ($version:literal) => {
        [
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/",
                $version,
                "/default-namespace-command/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/",
                $version,
                "/namespace-prefix-override/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/",
                $version,
                "/literal-command-prefix-override/stdout"
            )),
            include_str!(concat!(
                "../../../../rust/tcl-registry/tests/data/native_jim_ensemble_rooted_result245/",
                $version,
                "/exact-create-and-option-selection/stdout"
            )),
        ]
    };
}

#[test]
fn original_jim_ensemble_sources_match_all_native_rooted_public_results() {
    // Native proof: naming.namespace.jim-original-scripted-ensemble-publication-with-rooted-result
    // docs/design/analysis/name-resolution-proofs/namespace-jim-original-scripted-ensemble-publication-with-rooted-result.md
    // Exact originals expose only callable publication and selector/option results.
    // They retain the explicit rooted result variable and literal trailing-space
    // command-name prefix, without private mapping or compiler authority.
    crate::counters::reset();
    let sources: [&[u8]; 4] = [
        rooted_factory_source!("default-namespace-command.tcl"),
        rooted_factory_source!("namespace-prefix-override.tcl"),
        rooted_factory_source!("literal-command-prefix-override.tcl"),
        rooted_factory_source!("exact-create-and-option-selection.tcl"),
    ];
    for (engine, rows) in [
        ("tcl8.4", rooted_factory_rows!("8.4.20")),
        ("tcl8.5", rooted_factory_rows!("8.5.19")),
        ("tcl8.6", rooted_factory_rows!("8.6.18")),
        ("tcl9.0", rooted_factory_rows!("9.0.4")),
        ("tcl9.1", rooted_factory_rows!("9.1.0")),
        ("jim", rooted_factory_rows!("jim")),
    ] {
        for (index, (source, rows)) in sources.iter().zip(rows).enumerate() {
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_library(
                &mut interp,
                tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble,
            );
            let context = format!("{engine}/namespace-factory/{index}");
            assert_phase(&mut interp, source, rows, "ORIGINAL", &context);
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
