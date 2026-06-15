//! Differential parity for `resolve_object_ref_args` (the iRules command→
//! `(arg_index, kinds)` table + the `class`/`persist` custom resolvers) against
//! the Python `irules_object_refs.resolve_object_ref_args`. Self-contained.

use tcl_irules::resolve_object_ref_args;

#[test]
fn resolve_args_match_python() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resolve_args.golden.tsv"
    );
    let golden = std::fs::read_to_string(path).expect("read golden");

    let mut checked = 0usize;
    for line in golden.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let command = f[0];
        let args: Vec<&str> = if f[1].is_empty() {
            Vec::new()
        } else {
            f[1].split(',').collect()
        };
        let rule_module = if f[2] == "<none>" { None } else { Some(f[2]) };
        let want = f[3];

        let res = resolve_object_ref_args(command, &args, rule_module);
        let got = if res.is_empty() {
            "-".to_owned()
        } else {
            res.iter()
                .map(|(i, kinds)| format!("{i}:{}", kinds.join("|")))
                .collect::<Vec<_>>()
                .join(";")
        };
        assert_eq!(got, want, "resolve({command:?}, {args:?}, {rule_module:?})");
        checked += 1;
    }
    assert!(checked > 100, "golden unexpectedly small: {checked}");
}
