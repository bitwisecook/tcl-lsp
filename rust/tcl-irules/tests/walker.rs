//! Differential parity for the iRules walker (`extract_irules_object_references`)
//! against `irules_refs.extract_irules_object_references`, covering
//! literal refs, `set`-binding copy-propagation, widening, nested bodies, `if`
//! conditions, and GTM pool fan-out. Self-contained — no Python at test time.

use tcl_irules::extract_irules_object_references;
use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;

#[test]
fn walker_matches_python() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let cases: Vec<(String, String)> =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/irule_cases.json")).unwrap())
            .expect("parse cases");
    let golden = std::fs::read_to_string(format!("{dir}/irule_refs.golden.tsv")).expect("golden");

    let mut reg = CommandRegistry::build_default();
    reg.load_dialect(DialectSet::IRULES);

    for (i, (line, (source, rule_module))) in golden.lines().zip(&cases).enumerate() {
        let want = line.split('\t').nth(1).unwrap_or("-");
        let refs = extract_irules_object_references(source, Some(rule_module), &reg);
        let got = if refs.is_empty() {
            "-".to_owned()
        } else {
            refs.iter()
                .map(|r| {
                    format!(
                        "{}|{}|{}|{}",
                        r.name,
                        r.command,
                        r.argument_index,
                        r.kinds.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("  ;;  ")
        };
        assert_eq!(got, want, "case {i}: {source:?}");
    }
}
