mod syntax {
#[path = "/workspace/.proofs/root-cst-trivia-token639/candidate/rust/tcl-compiler/src/parsing/syntax/green.rs"]
pub mod green;
#[path = "/workspace/.proofs/root-cst-trivia-token639/candidate/rust/tcl-compiler/src/parsing/syntax/build.rs"]
pub mod build;
#[path = "/workspace/.proofs/root-cst-trivia-token639/candidate/rust/tcl-compiler/src/parsing/syntax/red.rs"]
pub mod red;
}
fn main() {
 let mut cases=0;let mut mismatches=0;
 for dialect in ["tcl8.4","tcl8.5","tcl8.6","tcl9.0","tcl9.1","jim","f5-irules"] {
  let config=tcl_lexer::LexerConfig::for_dialect(dialect);
  for source in ["set $x {a $b}","# note\r\nset é [list value]; puts \"${é}\"","set x A\\\n B","\u{feff}set x 1","list {*}{one two} {};\n","foo {a}{b}","list $$x ${a{b}c}",""," \t\n"] {
   for image in [tcl_lexer::SourceImage::document(source),tcl_lexer::SourceImage::native(source.as_bytes())] {
    let sm=tcl_lexer::SourceMap::from_image(&image);
    let original=tcl_lexer::Lexer::with_source_map(sm,config).tokenise_all().unwrap();
    let(document,_)=syntax::build::build_document_image(&image,config).unwrap();
    let tree=syntax::red::SyntaxTree::new(document);
    let projected=tree.root().lexer_tokens();
    cases+=1;
    if original!=projected {mismatches+=1;println!("mismatch\t{}\t{:?}\t{:?}\noriginal\t{:?}\nprojection\t{:?}",dialect,image.channel(),source,original,projected);}
   }
  }
 }
 println!("cases\t{}\nmismatches\t{}",cases,mismatches);
 if mismatches!=0 {std::process::exit(1);}
}
