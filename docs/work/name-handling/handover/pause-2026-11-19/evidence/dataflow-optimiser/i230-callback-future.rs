use std::sync::atomic::{AtomicUsize,Ordering};
use std::io::Write as _;
use std::process::Command;
use tcl_compiler::ir::Statement;
fn releases_on_path() -> Vec<(&'static str, String)> {
 [("8.4","TCL_LSP_TCLSH84"),("8.5","TCL_LSP_TCLSH85"),("8.6","TCL_LSP_TCLSH86"),("9.0","TCL_LSP_TCLSH90"),("9.1","TCL_LSP_TCLSH91")].into_iter().map(|(s,k)|(s,std::env::var(k).unwrap())).collect()
}
fn fixture_id<'a>(inventory:impl IntoIterator<Item=(&'a tcl_compiler::ir::ProcedureId,&'a tcl_compiler::ir::Procedure)>, name:&str) -> tcl_compiler::ir::ProcedureId {
 let mut matching=inventory.into_iter().filter(|(_,p)| p.qualified_name==name);
 let (id,_)=matching.next().unwrap();assert!(matching.next().is_none());*id
}
fn run_script(tclsh: &str, script: &str) -> Option<(bool, String)> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "value-transfer-witness-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .and_then(|mut file| file.write_all(script.as_bytes()))
        .ok()?;
    let output = Command::new(tclsh).arg(&path).output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}
fn condition_claims(source: &str, dialect: &str) -> Vec<bool> {
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|diagnostic| format!("{:?}",diagnostic.code) == "I230")
        .filter_map(|diagnostic| {
            if diagnostic.message.contains("is always true") {
                Some(true)
            } else if diagnostic.message.contains("is always false") {
                Some(false)
            } else {
                None
            }
        })
        .collect()
}

fn main(){
 for (frame,definition) in [(" -frame caller","proc db_bind {spec} {upvar 1 done d; set d 1}"),("","proc db_bind {spec} {upvar 1 done d; set d 1}"),(" -frame own","proc db_bind {spec} {return}"),(" -frame none","proc db_bind {spec} {return}")] {
    let code=format!("# tcl-lsp: stubs-begin\n# tcl-lsp: stub db_bind {{spec}}{frame}\n# tcl-lsp: stubs-end\nset done 0\nafter 10 {{db_bind {{done}}}}\nafter 50\nupdate\nif {{$done}} {{puts 1}} else {{puts 0}}\n");
    for d in ["tcl8.4","tcl8.6","tcl9.1","jimtcl"] {
        let a=tcl_compiler::analyser::Analyser::new().analyse(&code,d);
        let input=a.resolved_input.as_ref().unwrap();let reg=input.borrowed_context_registry().commands();
        let unit=a.original_optimiser_source_unit(&code,reg,None).unwrap();
        println!("{d} {frame:?} slots={:?}",unit.ir_module.declared_frame_effects);
        let src=unit.ir_module.retained_source_bindings.as_ref().unwrap().positioned_for_module(&unit.ir_module,reg).unwrap();
        for body in src.future_body_inventories() {println!("{d} {frame:?} future at {} source {:?} calls {:?}",body.registration().offset,body.source().text,body.invocations().iter().map(|(o,b)|(*o,b.runtime_reachability(),b.proved_target().map(|t|t.command.clone()),b.proved_execution_target().map(|t|t.command.clone()))).collect::<Vec<_>>());}
        let f=&unit.top_level;let sem=f.semantic_values().unwrap();
        let ops=tcl_compiler::optimiser::manager::optimise_with_dialect(&code,reg,Some(input.unit_profile()));
        let ops=ops.iter().map(|v|format!("{:?}: {:?}",v.code,v.replacement)).collect::<Vec<_>>();
        for (id,b) in &f.cfg.blocks {
          if let Some(tcl_compiler::cfg::Terminator::Branch{true_target,false_target,..})=&b.terminator {
             println!("{d} {frame:?} block={} strictT={} strictF={} semT={} semF={} optics={ops:?}",f.cfg.block_name(*id),f.sccp.executable_blocks.contains(true_target),f.sccp.executable_blocks.contains(false_target),sem.reaches(*true_target),sem.reaches(*false_target));
          }
        }
        if d=="tcl8.6" {for (block,node) in &f.ssa.blocks { for (n,st) in node.statements.iter().enumerate() { println!("  {frame:?} {block:?}/{n} {} {:?} marker={:?} defs={:?}",node.name,std::mem::discriminant(&st.statement),st.statement.synthetic_marker(),st.defs); } }}
    }
 }
}
