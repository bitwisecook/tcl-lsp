use tcl_compiler::analyser::Analyser;
fn w210(src:&str,var:&str)->usize{ Analyser::new().analyse(src,"tcl8.6").diagnostics.iter().filter(|d|d.code.to_string()=="W210"&&d.message.contains(var)).count() }
fn w123full(src:&str){ for d in Analyser::new().analyse(src,"tcl8.6").diagnostics.iter().filter(|d|d.code.to_string()=="W123"){ println!("    W123: {}", d.message);} }
fn main(){
    println!("upvar bare local: {}", w210("proc f {} { upvar 1 caller_v local; puts $local }","'local'"));
    println!("upvar ::qualified local: {}", w210("proc f {} { ::upvar 1 caller_v local; puts $local }","'local'"));
    println!("upvar aliased local: {}", w210("interp alias {} link {} upvar\nproc f {} { link 1 caller_v local; puts $local }","'local'"));
    println!("-- which providers suppress W123 --");
    for s in ["load mylib.so\nmycommand arg1","package require Tk\nbutton .b -text hi","rename puts myputs\nmyputs hello","namespace import ::foo::*\nbar arg","lappend auto_path /opt/mylib\nmycmd arg"]{
        let n=Analyser::new().analyse(s,"tcl8.6").diagnostics.iter().filter(|d|d.code.to_string()=="W123").count();
        println!("  n={} :: {:?}", n, s.lines().next().unwrap());
    }
    println!("has_dynamic_providers flags:");
    for s in ["load mylib.so\nmycommand arg1","rename puts myputs\nmyputs hello","namespace import ::foo::*\nbar arg","lappend auto_path /opt/mylib\nmycmd arg"]{
        println!("  hdp={} :: {:?}", Analyser::new().analyse(s,"tcl8.6").has_dynamic_providers, s.lines().next().unwrap());
    }
}
