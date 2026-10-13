// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual namespace-variable query results from retained reference executions.

fn compare_queries(engine: &str, observation: &str) {
    let profile = crate::environment::profile_for_dialect(engine);
    let mut vm = crate::native_fixture::interpreter(profile);
    let completion = vm
        .try_eval_source(include_str!(
            "../../../../experiments/name-handling/variable-namespace-query-bases/result-input.tcl"
        ))
        .unwrap();
    assert!(completion.code.is_ok(), "{engine}: {:?}", completion.result);
    assert!(vm.refused_completion().is_none(), "{engine}");
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let bytes = completion.result.native_string_bytes(protocol).unwrap();
    let actual = std::str::from_utf8(bytes.as_ref()).unwrap();
    let (actual_identity, actual_queries) = actual.split_once('\n').unwrap();
    let (oracle_identity, oracle_queries) = observation
        .strip_suffix('\n')
        .unwrap()
        .split_once('\n')
        .unwrap();
    if engine == "jim" {
        assert_eq!(actual_identity, "PATCHLEVEL 0.84");
        assert_eq!(oracle_identity, "PATCHLEVEL 0.84-9-g5bac7c9");
        eprintln!(
            "Jim query rows compare latest retained 5bac7c99ad65864c87da513e22e2f01703fa4e03; built core reporting ({actual_identity}) and external oracle reporting ({oracle_identity}) remain distinct"
        );
    } else {
        assert_eq!(
            actual_identity, oracle_identity,
            "{engine}: release reporting"
        );
    }
    assert_eq!(actual_queries.lines().count(), 8);
    assert_eq!(oracle_queries.lines().count(), 8);
    assert_eq!(
        actual_queries, oracle_queries,
        "{engine}: eight original namespace which -variable results"
    );
    eprintln!(
        "{engine}: 8 original namespace-variable query rows compared; no scalar-read surrogate"
    );
}

macro_rules! query_rows {
    ($name:ident, $engine:literal, $provider:literal) => {
        #[test]
        fn $name() {
            compare_queries(
                $engine,
                include_str!(concat!(
                    "../../../../experiments/name-handling/variable-namespace-query-bases/",
                    $provider,
                    "/result.stdout.bin"
                )),
            );
        }
    };
}

query_rows!(original_variable_queries_c84, "tcl8.4", "84");
query_rows!(original_variable_queries_c85, "tcl8.5", "85");
query_rows!(original_variable_queries_c86, "tcl8.6", "86");
query_rows!(original_variable_queries_c90, "tcl9.0", "90");
query_rows!(original_variable_queries_c91, "tcl9.1", "91");
query_rows!(original_variable_queries_jim, "jim", "jim");
