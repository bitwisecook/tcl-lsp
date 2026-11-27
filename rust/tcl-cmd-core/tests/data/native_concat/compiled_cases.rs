fn compiled_rows(engine: &str) -> &'static str {
    match engine {
        "tcl8.4" => include_str!("8.4.20.tsv"),
        "tcl8.5" => include_str!("8.5.19.tsv"),
        "tcl8.6" => include_str!("8.6.18.tsv"),
        "tcl9.0" => include_str!("9.0.4.tsv"),
        "tcl9.1" => include_str!("9.1.0.tsv"),
        _ => unreachable!("exact native C catalogue"),
    }
}
const COMPILED_BODIES: [&[u8]; 8] = [
    b"concat",
    b"concat {}",
    b"concat {} {}",
    b"concat { } {\t}",
    b"concat A",
    b"concat {A B} C",
    b"concat {A\\ } B",
    b"concat {A\\\\ } B",
];
fn compiled_row(engine: &str, case: usize) -> Vec<&'static str> {
    let prefix = format!("C\t{case}\tresult\t");
    tcl_test_support::fixture::tsv_fields(
        compiled_rows(engine)
            .lines()
            .find(|row| row.starts_with(&prefix))
            .unwrap(),
    )
    .collect()
}
