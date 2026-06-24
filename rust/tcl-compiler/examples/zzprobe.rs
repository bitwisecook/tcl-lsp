use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new().analyse(src, dialect).diagnostics.iter().map(|d| d.code.to_string()).collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    for d in run_all_checks(&cu, registry, (!dialect.is_empty()).then_some(dialect)) {
        if d.code.is_optimisation() { continue; } out.push(d.code.to_string()); }
    out
}
fn w210_for(src:&str, var:&str)->usize{ Analyser::new().analyse(src,"tcl8.6").diagnostics.iter().filter(|d| d.code.to_string()=="W210" && d.message.contains(&format!("'{}'",var))).count() }
fn show(label:&str,src:&str){ println!("{:55} => {:?}", label, codes(src,"tcl8.6")); }
fn main(){
    // top-level regsub/regexp output var (Python tests are top-level)
    println!("--- W210 count for specific output var (top-level) ---");
    println!("regsub result: {}", w210_for("regsub {old} $text new result\nputs $result","result"));
    println!("regexp user: {}", w210_for("regexp {^(\\w+)@(\\w+)$} $email -> user domain\nputs $user","user"));
    println!("regexp match: {}", w210_for("regexp {\\d+} $text match\nputs $match","match"));
    println!("scan num: {}", w210_for("scan \"42 hello\" \"%d %s\" num word\nputs $num","num"));
    println!("lassign x: {}", w210_for("lassign {a b c} x y z\nputs \"$x $y $z\"","x"));
    println!("regsub-in-proc result: {}", w210_for("proc f {text} { regsub {old} $text new result; puts $result }","result"));
    println!("--- I231 variants ---");
    show("switch const arm I231", "switch 1 {1 {puts a} 2 {puts b} default {puts c}}");
    println!("--- canonicalisation W211/W213 qualified ---");
    show("::set y", "proc f {} { ::set y 1 }");
    show("set y", "proc f {} { set y 1 }");
    show("::unset x", "proc f {} { ::unset x }");
    show("aliased unset", "interp alias {} myunset {} unset\nproc f {} { myunset x }");
    show("aliased set", "interp alias {} s {} set\nproc f {} { s y 1 }");
    println!("--- W210 canonicalisation (variable/upvar decl) ---");
    println!("variable decl: {}", w210_for("namespace eval ns { variable v; proc f {} { variable v; puts $v } }","v"));
    println!("upvar decl: {}", w210_for("proc f {} { upvar 1 caller_v local; puts $local }","local"));
    println!("global decl: {}", w210_for("set g 0\nproc f {} { global g; set g 1 }","g"));
    println!("--- misc ---");
    show("interp alias too few no crash","interp alias {} = {}");
    show("trace add cb", "proc watcher {name1 name2 op} { puts hello }\ntrace add variable x write watcher\n");
    show("proc unused param + trace","proc watcher {name1 name2 op} { puts hello }\nproc other {a b} { puts $a }\ntrace add variable x write watcher\n");
    show("string eq const fold I230","set x foo\nif {$x eq \"foo\"} { puts hi }");
    show("string == const fold I230","set x foo\nif {$x == \"foo\"} { puts hi }");
}
