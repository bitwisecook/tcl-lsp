use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::model::ingress::resolve_environment;
fn main() {
 let src="proc answer {} {return 42}; proc f {} {set y [answer]}; unknown_future_entry";
 for d in ["tcl8.6","jimtcl"] {
  let ctx=resolve_environment(d).default_context_registry(); let reg=ctx.commands();
  let unit=CompilationUnit::build_for_dialect(src,reg,true,d);
  for (id,f) in &unit.procedures {
   for block in f.ssa.blocks.values(){for s in &block.statements {
    if let Some(t)=s.statement.tokens(){for (off,b) in t.nested_bindings.iter() {
     println!("{} {} {:?} {} {:?} exec={:?} diag={:?} tokens={:?}",d,f.name,id,off,b.runtime_reachability(),b.proved_execution_target().map(|v| &v.command),b.proved_target().map(|v| &v.command),t.argv_texts);
    }}
   }}
  }
 }
}
