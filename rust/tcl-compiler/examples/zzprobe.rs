use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;
fn codes(src: &str, d: &str) -> Vec<String> {
    let mut o: Vec<String> = Analyser::new().analyse(src,d).diagnostics.iter().map(|x|x.code.to_string()).collect();
    let reg=registry_for_dialect(d); let cu=CompilationUnit::build_for(src,reg,false);
    for x in run_all_checks(&cu,reg,(!d.is_empty()).then_some(d)){ if x.code.is_optimisation(){continue;} o.push(x.code.to_string()); }
    o
}
fn full(src:&str,d:&str){ for x in Analyser::new().analyse(src,d).diagnostics.iter(){ println!("   A {} : {}", x.code, x.message);} 
    let reg=registry_for_dialect(d); let cu=CompilationUnit::build_for(src,reg,false);
    for x in run_all_checks(&cu,reg,(!d.is_empty()).then_some(d)){ if x.code.is_optimisation(){continue;} println!("   C {} : {}", x.code, x.message);} }
fn main(){
    println!("== _unused x params =="); full("proc foo {_unused x} { puts $x }","tcl8.6");
    println!("== proc child scope name =="); let r=Analyser::new().analyse("proc foo {x} { set y 1 }","tcl8.6"); println!("  name={:?}", r.global_scope.children[0].name);
    println!("== package files codes =="); println!("{:?}",codes("package files mypackage","tcl8.6")); full("package files mypackage","tcl8.6");
    println!("== package prefer codes =="); println!("{:?}",codes("package prefer","tcl8.6"));
    println!("== command_subst inside expr vars =="); let r=Analyser::new().analyse("set n [expr {[set y 1] + 2}]","tcl8.6"); println!("  vars={:?}", r.global_scope.variables.keys().collect::<Vec<_>>());
    println!("== trace unrelated b/other =="); full("proc watcher {name1 name2 op} { puts hello }\nproc other {a b} { puts $a }\ntrace add variable x write watcher\n","tcl8.6");
    println!("== dynamic providers each =="); 
    for s in ["load mylib.so\nmycommand arg1","package require Tk\nbutton .b -text hi","rename puts myputs\nmyputs hello","namespace import ::foo::*\nbar arg","lappend auto_path /opt/mylib\nmycmd arg"]{ println!("  -- {:?}", s); for (c,m) in Analyser::new().analyse(s,"tcl8.6").diagnostics.iter().filter(|d|d.code.to_string()=="W123").map(|d|(d.code.to_string(),d.message.clone())){ println!("     W123 {}", m);} }
    println!("== upvar local W210 =="); full("proc f {} { upvar 1 caller_v local; puts $local }","tcl8.6");
    println!("== dict_for braced data word =="); full("proc foo {used unused} {\n    set d [dict create]\n    dict for {k v} $d {\n        set msg {$unused}\n        puts $used\n    }\n}\n","tcl8.6");
}
