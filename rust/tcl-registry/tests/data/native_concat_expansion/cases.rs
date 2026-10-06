const EXPANSION_BODIES: [&[u8]; 8] = [
    b"concat {*}{A B} $c",
    b"concat {*}{A B} C",
    b"concat {*}{} $c",
    b"concat {*}{ {A B} {} } $c",
    b"concat {*}{A\\ B C} $c",
    b"concat {*}$a $c",
    b"concat {*}{\"} $c",
    b"concat {*}{A B} {*}$a $c",
];
fn expansion_rows(engine: &str) -> &'static str {
    match engine {
        "tcl8.4" => include_str!("8.4.20.tsv"),
        "tcl8.5" => include_str!("8.5.19.tsv"),
        "tcl8.6" => include_str!("8.6.18.tsv"),
        "tcl9.0" => include_str!("9.0.4.tsv"),
        "tcl9.1" => include_str!("9.1.0.tsv"),
        "jim" => include_str!("jim0.84.tsv"),
        _ => unreachable!("exact native six catalogue"),
    }
}
fn expansion_result(engine: &str, case: usize) -> Vec<&'static str> {
    let prefix = format!("RESULT\t{case}\t");
    expansion_rows(engine)
        .lines()
        .find(|row| row.starts_with(&prefix))
        .unwrap()
        .split('\t')
        .collect()
}
