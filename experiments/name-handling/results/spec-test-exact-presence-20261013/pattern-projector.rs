#[path="/workspace/tcl-lsp/rust/tcl-syntax/src/glob.rs"] mod glob;
fn main() {
 let input=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
 for name in std::str::from_utf8(&input).unwrap().split('\0').filter(|s| !s.is_empty()) {
  let word=tcl_syntax::list::list_element(name);
  let pattern=tcl_syntax::list::list_element(&glob::literal_pattern(name));
  println!("::__spec_test::present {word} {pattern}\nputs [list PRESENCE {word} $::__spec_test::skip]");
 }
}
