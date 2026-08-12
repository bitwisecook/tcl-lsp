use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap};

fn main() {
    let src = r#"
# a comment
speclib tcl 1.0 {
    command lsort {
        arity 1..
        option -stride -takes strideLength -integer {Range 2 max} \
            -detail {Treat list as groups}
        subcommand at { arity 1 ; detail {Get header.} ; synopsis {HTTP::header at <index>} }
        hover {
            summary {Sort it.}
        }
        clause_flags {}
    }
}
"#;
    let sm = SourceMap::new(src);
    let (doc, _w) = build_document(src, LexerConfig::default());
    for seg in segments_from_document(doc, &sm) {
        println!("CMD {:?}", seg.texts);
        for (i, t) in seg.argv.iter().enumerate() {
            println!("   [{i}] kind={:?} co={} text={:?}", t.kind, t.content_offset, seg.texts[i]);
        }
    }
}
