use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap};

fn seg(src: &str) -> Vec<(Vec<String>, Vec<String>)> {
    let sm = SourceMap::new(src);
    let (doc, _w) = build_document(src, LexerConfig::default());
    segments_from_document(doc, &sm)
        .into_iter()
        .map(|s| {
            let kinds = s.argv.iter().map(|t| format!("{:?}/{}", t.kind, t.content_offset)).collect();
            (s.texts, kinds)
        })
        .collect()
}

fn walk(src: &str, depth: usize) {
    for (texts, kinds) in seg(src) {
        println!("{}CMD {:?}", "  ".repeat(depth), texts);
        println!("{}    kinds {:?}", "  ".repeat(depth), kinds);
    }
}

fn main() {
    let inner = r#"
    command lsort {
        arity 1..
        option -stride -takes strideLength -integer {Range 2 max} \
            -detail {Treat list as groups}   ;# trailing comment
        subcommand at { arity 1 ; detail {Get header.} ; synopsis {HTTP::header at <index>} }
        clause_flags {}
        option --  -detail {end of options}
        example {a
b

c}
    }
"#;
    walk(inner, 0);
    println!("--- inner of command body:");
    let body = &seg(inner)[0].0[2];
    walk(body, 1);
}
