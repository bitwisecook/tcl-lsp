#[derive(Clone, Copy)]
enum Input {
    Bytes(&'static [u8]),
    List(&'static [&'static [u8]]),
    Parsed(&'static [u8]),
    Rendered(&'static [&'static [u8]]),
}
fn inputs(case: usize) -> Vec<Input> {
    use Input::{Bytes as B, List as L, Parsed as P, Rendered as R};
    match case {
        0 => vec![],
        1 | 2 => vec![B(b"")],
        3 => vec![L(&[b"a", b"b"])],
        4 => vec![L(&[])],
        5 => vec![L(&[b"a"]), L(&[b"b"])],
        6 => vec![L(&[b"a"]), L(&[b"#b"])],
        7 => vec![L(&[b"#a"]), L(&[b"b"])],
        8 => vec![P(b" a  {b} ")],
        9 => vec![R(&[b"a", b"b"])],
        10 => vec![B(b""), L(&[b"a"])],
        11 => vec![B(b"  "), L(&[b"a"])],
        12 => vec![L(&[b"a"]), L(&[])],
        13 => vec![L(&[]), L(&[b"#b"])],
        14 => vec![B(b" a "), B(b" b ")],
        15 => vec![B(b""), B(b"")],
        16 => vec![B(b" "), B(b"\t")],
        17 => vec![B(b"a\\ "), B(b"b")],
        18 => vec![B(b"a\\\\ "), B(b"b")],
        19 => vec![L(&[b"a"]), P(b" #b ")],
        _ => unreachable!("fixed original twenty native controls"),
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").unwrap();
        output
    })
}
fn state(
    snapshot: &tcl_syntax::native_object::NativeObjectSnapshot,
    refs: usize,
    version: Option<tcl_dialect::TclVersion>,
) -> String {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
    let (kind, canonical) = match &snapshot.cache {
        Cache::None => ("NULL", false),
        Cache::List { canonical, .. } => (
            "list",
            match version {
                None => true,
                Some(tcl_dialect::TclVersion::V8_4) => snapshot.resident.is_none(),
                _ => *canonical || snapshot.resident.is_none(),
            },
        ),
        Cache::String {
            num_chars: None,
            unicode: None,
            ..
        } => ("string", false),
        other => panic!("unexpected actual concat primary {other:?}"),
    };
    let mut result = format!(
        "{kind},{},{},{refs},{},{}",
        usize::from(snapshot.resident.is_some()),
        usize::from(canonical),
        snapshot
            .resident
            .as_ref()
            .map_or(-1, |bytes| isize::try_from(bytes.len()).unwrap()),
        snapshot
            .resident
            .as_ref()
            .map_or_else(String::new, |bytes| hex(bytes))
    );
    if version.is_some() {
        result.push_str(match snapshot.storage {
            None => ",storage=absent",
            Some(Storage::Allocated) => ",storage=allocated",
            Some(Storage::CanonicalEmpty) => ",storage=canonical",
            other => panic!("unattested native resident storage {other:?}"),
        });
        if kind == "string" {
            result.push_str(",chars=-1");
        }
    }
    result
}
fn rows(engine: &str) -> &'static str {
    match engine {
        "tcl8.4" => include_str!("8.4.20.tsv"),
        "tcl8.5" => include_str!("8.5.19.tsv"),
        "tcl8.6" => include_str!("8.6.18.tsv"),
        "tcl9.0" => include_str!("9.0.4.tsv"),
        "tcl9.1" => include_str!("9.1.0.tsv"),
        "jim" => include_str!("jim.tsv"),
        _ => unreachable!("exact native six catalogue"),
    }
}
fn row(engine: &str, case: usize, window: &str) -> Vec<&'static str> {
    let prefix = format!("D\t{case}\t{window}\t");
    rows(engine)
        .lines()
        .find(|row| row.starts_with(&prefix))
        .unwrap()
        .split('\t')
        .collect()
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
    rows(engine)
        .lines()
        .find(|row| row.starts_with(&prefix))
        .unwrap()
        .split('\t')
        .collect()
}
