use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() { continue; }
        out.push(d.code.to_string());
    }
    out
}

fn main() {
    let cases: &[(&str, &str)] = &[
        ("set", "set bare"),
        ("break extra", "break extra"),
        ("set x 42", "set x 42"),
        ("puts -nonewline stderr hello", "puts nonewline"),
        ("puts a b c", "puts a b c"),
        ("regsub -all -line {\\n} $args {} str", "regsub switches"),
        ("regsub a b c d e", "regsub 5 positional"),
        ("while {1}", "while too few"),
        ("string bogus hello", "string bogus"),
        ("package prefer", "package prefer"),
        ("package files mypackage", "package files"),
        ("namespace", "namespace bare"),
        ("mycommand a b c d e", "unknown cmd arity"),
        ("for {set i 0} {$i < 10}", "for too few"),
        ("proc add {a b} { return [expr {$a + $b}] }\nadd 1\n", "proc call too few"),
        ("proc add {a b} { return [expr {$a + $b}] }\nadd 1 2 3\n", "proc call too many"),
        ("puts $x", "read before set top"),
        ("if {$x > 0} { puts yes }", "read before set expr"),
        ("set x 1\nputs $x", "read before set cleared"),
        ("set arr(key) 1\nputs $arr(key)", "array read base"),
        ("proc foo {} { set x 1 }", "unused assigned W211"),
        ("proc foo {} { set x 1; puts $x }", "used no W211"),
        ("proc foo {} { set x 1; set x 2; puts $x }", "dead assign W220"),
        ("proc foo {} { set x 0; set x 0; puts $x }", "paste error H300"),
        ("proc foo {} { set x 0; set x 1; puts $x }", "no paste val change"),
        ("if {1} { set x 1 } else { set y 2 }", "const if I230"),
        ("switch 1 {1 {set x 1} 2 {set y 2} default {set z 3}}", "const switch I231"),
        ("proc foo {x y} { puts $x }", "unused param W214"),
        ("proc foo {a b} { puts $a; puts $b }", "all params used"),
        ("proc foo {args} { puts hello }", "args not flagged"),
        ("proc foo {_unused x} { puts $x }", "underscore not flagged"),
        ("mycommand a b c", "W123 unknown"),
        ("set x 1", "W123 builtin none"),
        ("putz hello", "did you mean"),
        ("xyzzyplugh arg1", "no suggestion distant"),
        ("proc f {} { unset x }", "W213 unset"),
        ("proc f {} { ::unset x }", "W213 ::unset"),
        ("proc f {} { unset -nocomplain x }", "W213 nocomplain"),
        ("set \"weird}name\" 1", "W215 brace"),
        ("set arr(name) hello\nputs ${arr}(name)", "W216 brace paren"),
        ("proc f {} { set y 1 }", "W211 set y"),
        ("namespace eval ns { proc watcher {name1 name2 op} { puts hello }\n trace add variable x write watcher }", "trace cb in ns"),
        ("proc watcher {name1 name2 op} { puts hello }\ntrace add variable x write watcher\n", "trace add cb"),
        ("proc f {} { while 0 { puts dead } }", "while 0 I230"),
        ("proc f {} { while 1 { if {[g]} break } }", "while 1 no I230"),
    ];
    for (src, label) in cases {
        let c = codes(src, "tcl8.6");
        println!("{:50} => {:?}", label, c);
    }
}
