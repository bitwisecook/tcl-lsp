//! Scratch probe — NOT a real test. Prints optimiser behaviour for the
//! Python test snippets so the port author can map each assertion to actual
//! Rust behaviour. Delete after authoring.

use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_registry::registry_for_dialect;

fn probe(label: &str, src: &str, dialect: &str) {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    let opts = optimise_with_dialect(src, registry, d);
    let codes: Vec<String> = opts.iter().map(|o| o.code.as_str().to_owned()).collect();
    let rewritten = apply_optimisations(src, &opts);
    println!("=== {label} [{dialect}] ===");
    println!("  SRC:  {:?}", src);
    println!("  OUT:  {:?}", rewritten);
    println!("  CODES:{:?}", codes);
    for o in &opts {
        println!("    {} repl={:?} msg={:?} hint={}", o.code.as_str(), o.replacement, o.message, o.hint_only);
    }
}

#[test]
fn probe_all() {
    // ---- TestOptimiser core ----
    probe("prop_fold_expr", "set a 1\nset b [expr {$a + 2}]", "tcl8.6");
    probe("division_folds", "set a 1\nset b [expr {$a / 2}]", "tcl8.6");
    probe("non_static_not_prop", "set a [clock seconds]\nset b [expr {$a + 2}]", "tcl8.6");
    probe("reassign_updates", "set a 1\nset a 5\nset b [expr {$a + 2}]", "tcl8.6");
    probe("chained_fold", "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]", "tcl8.6");
    probe("unset_clears", "set a 1\nunset a\nset b [expr {$a + 2}]", "tcl8.6");
    probe("proc_body_opt", "proc add_two {} {\n    set a 1\n    return [expr {$a + 2}]\n}\n", "tcl8.6");
    probe("direct_expr_sub", "set v [expr {3}]", "tcl8.6");
    probe("escaping_cmd_sub", "set n [eval $s]\nset n 1\nputs $n\n", "tcl8.6");
    probe("interproc_static", "proc one {} { return 1 }\nset v [one]\n", "tcl8.6");
    probe("interproc_passthrough", "proc id {x} { return $x }\nset a 7\nset v [id $a]\n", "tcl8.6");
    probe("interproc_nonpure", "proc noisy {} { puts hi; return 1 }\nset v [noisy]\n", "tcl8.6");
    probe("interproc_ns", "namespace eval math {\n    proc one {} { return 1 }\n    proc use {} { return [one] }\n}\n", "tcl8.6");
    probe("no_prop_into_loop", "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n", "tcl8.6");
    probe("static_for_post_fold", "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\nif {$total > 5} {\n    puts ok\n}\n", "tcl8.6");
    probe("append_chain", "set msg {Hello}\nappend msg { }\nappend msg World", "tcl8.6");
    probe("append_chain_dyn", "set msg {}\nappend msg $name\nappend msg !", "tcl8.6");
    probe("write_chain_nonreading", "set msg {Hello}\nputs ok\nappend msg { World}", "tcl8.6");
    probe("write_chain_read", "set msg {Hello}\nputs $msg\nappend msg { World}", "tcl8.6");
    probe("interproc_add", "proc add {a b} { return [expr {$a + $b}] }\nset v [add 3 4]\n", "tcl8.6");
    probe("interproc_max", "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n", "tcl8.6");
    probe("interproc_incr", "proc inc3 {x} { incr x 3; return $x }\nset v [inc3 10]\n", "tcl8.6");
    probe("proc_in_expr", "proc one {} { return 1 }\nif {[one] != 0} {\n    puts yes\n}\n", "tcl8.6");
    probe("proc_args_in_expr", "proc add {a b} { return [expr {$a + $b}] }\nif {[add 3 4] == 7} {\n    puts yes\n}\n", "tcl8.6");
    probe("proc_expr_var", "proc one {} { return 1 }\nset x 5\nif {[one] + $x == 6} {\n    puts yes\n}\n", "tcl8.6");
    probe("impure_proc_expr", "proc noisy {} { puts hi; return 1 }\nif {[noisy] == 1} {\n    puts yes\n}\n", "tcl8.6");
    probe("dse_dead_store", "set a 1\nset a 2\nputs $a", "tcl8.6");
    probe("adce_trans_dead", "set a 1\nset a [expr {$a + 1}]\nset a 5\nputs $a", "tcl8.6");
    probe("dce_unreach", "if {0} {\n    puts never\n    set x 1\n}\nputs always\n", "tcl8.6");
    probe("instcombine_reassoc", "set v [expr {$a + 1 + 2}]", "tcl8.6");
}

fn int_x(body: &str) -> String {
    format!("proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n")
}

#[test]
fn probe_instcombine() {
    probe("pow_zero", &int_x("set v [expr {$x ** 0}]"), "tcl8.6");
    probe("pow_one", &int_x("set v [expr {$x ** 1}]"), "tcl8.6");
    probe("shift_zero", &int_x("set v [expr {$x << 0}]"), "tcl8.6");
    probe("rshift_zero", &int_x("set v [expr {$x >> 0}]"), "tcl8.6");
    probe("bitand_zero", &int_x("set v [expr {$x & 0}]"), "tcl8.6");
    probe("bitor_zero", &int_x("set v [expr {$x | 0}]"), "tcl8.6");
    probe("bitxor_zero", &int_x("set v [expr {$x ^ 0}]"), "tcl8.6");
    probe("mod_one", &int_x("set v [expr {$x % 1}]"), "tcl8.6");
    probe("and_zero", "set v [expr {$x && 0}]", "tcl8.6");
    probe("and_one", "set v [expr {$x && 1}]", "tcl8.6");
    probe("and_one_fold", "set v [expr {2 && 1}]", "tcl8.6");
    probe("or_one", "set v [expr {$x || 1}]", "tcl8.6");
    probe("or_zero", "set v [expr {$x || 0}]", "tcl8.6");
    probe("or_zero_fold", "set v [expr {3 || 0}]", "tcl8.6");
    probe("double_not_bool", "set v [expr {!!($a == $b)}]", "tcl8.6");
    probe("not_eq", "set v [expr {!($a == $b)}]", "tcl8.6");
    probe("not_lt", "set v [expr {!($a < $b)}]", "tcl8.6");
    probe("not_in", "set v [expr {!($a in $b)}]", "tcl8.6");
    probe("demorgan_not_and", "set v [expr {!($a && $b)}]", "tcl8.6");
    probe("demorgan_not_or", "set v [expr {!($a || $b)}]", "tcl8.6");
    probe("demorgan_cmp_inv", "set v [expr {!($a == $b && $c < $d)}]", "tcl8.6");
    probe("demorgan_or", "set v [expr {!($a == $b || $c < $d)}]", "tcl8.6");
    probe("demorgan_in_if", "if {!($x && $y)} { puts yes }", "tcl8.6");
    probe("double_bitnot", &int_x("set v [expr {~~$x}]"), "tcl8.6");
    probe("self_eq", "set v [expr {$x == $x}]", "tcl8.6");
    probe("self_ne", "set v [expr {$x != $x}]", "tcl8.6");
    probe("self_xor", &int_x("set v [expr {$x ^ $x}]"), "tcl8.6");
    probe("ternary_ident", "set v [expr {$c ? $a : $a}]", "tcl8.6");
    probe("ternary_not_flip", "set v [expr {!$c ? $a : $b}]", "tcl8.6");
    probe("ternary_01_neg", "set v [expr {$x ? 0 : 1}]", "tcl8.6");
    probe("ternary_10_bool", "if {($a > $b) ? 1 : 0} { puts yes }", "tcl8.6");
    probe("ternary_const_true", "set v [expr {1 ? $a : $b}]", "tcl8.6");
    probe("ternary_const_false", "set v [expr {0 ? $a : $b}]", "tcl8.6");
    probe("double_not_if", "if {!!$x} { puts yes }", "tcl8.6");
}

#[test]
fn probe_structure() {
    probe("if_true", "if {1} {\n    set x 1\n}", "tcl8.6");
    probe("if_false", "if {0} {\n    set x 1\n}\nputs always", "tcl8.6");
    probe("if_false_else", "if {0} {\n    set x 1\n} else {\n    set y 2\n}", "tcl8.6");
    probe("if_elseif", "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}", "tcl8.6");
    probe("while_false", "while {0} {\n    puts looping\n}\nputs done", "tcl8.6");
    probe("for_false_init", "for {set i 0} {0} {incr i} {\n    puts looping\n}", "tcl8.6");
    probe("for_false_empty", "for {} {0} {} {\n    puts looping\n}\nputs done", "tcl8.6");
    probe("switch_lit", "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}", "tcl8.6");
    probe("switch_default", "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}", "tcl8.6");
    probe("switch_nomatch", "switch xyz {\n    abc { set a 1 }\n}\nputs done", "tcl8.6");
    probe("switch_glob", "switch -glob aaab {\n    a*b { set x 1 }\n    b { set y 2 }\n    a* { set z 3 }\n    default { set w 4 }\n}", "tcl8.6");
    probe("switch_glob_default", "switch -glob xyz {\n    a* { set x 1 }\n    b* { set y 2 }\n    default { set z 3 }\n}", "tcl8.6");
    probe("switch_regexp", "switch -regexp abc {\n    ^a { set x 1 }\n    default { set y 2 }\n}", "tcl8.6");
    probe("switch_fallthrough", "switch -glob abc {\n    a* -\n    z* { set x 1 }\n    default { set y 2 }\n}", "tcl8.6");
    probe("switch_ft_chain", "switch abc {\n    abc -\n    def -\n    default { set x 1 }\n}", "tcl8.6");
    probe("switch_nocase", "switch -nocase ABC {\n    abc { set x 1 }\n    default { set y 2 }\n}", "tcl8.6");
    probe("runtime_cond", "if {$x} {\n    set y 1\n}", "tcl8.6");
    probe("nested_struct", "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}", "tcl8.6");
}

#[test]
fn probe_families() {
    // O126
    probe("o126_return_not_removed", "proc calcDb {mag} {\n    set db [expr {10*log($mag)}]\n    return $db\n}\n", "tcl8.6");
    probe("o126_braced_return", "proc foo {} {\n    set result 42\n    return {$result}\n}\n", "tcl8.6");
    // cross-event DSE
    probe("xevent_store", "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\nwhen HTTP_RESPONSE {\n    log local0. \"uri=$uri\"\n}\n", "f5-irules");
    probe("xevent_infoexists", "when DNS_REQUEST {\n    if { [DNS::header opcode] ne \"QUERY\" } {\n        set ans_cleared 1\n        DNS::return\n        return\n    }\n}\nwhen DNS_RESPONSE {\n    if { [info exists ans_cleared] } {\n        unset -nocomplain -- ans_cleared\n        return\n    }\n}\n", "f5-irules");
    // O100 propagation
    probe("o100_into_cmd", "set x 42\nputs $x", "tcl8.6");
    probe("o100_through_expr", "set a 1\nset b [expr {$a + 1}]\nputs $b", "tcl8.6");
    probe("o100_chained", "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]\nputs $c", "tcl8.6");
    probe("o100_alluses_dse", "set x 5\nputs $x\nset y [expr {$x + 1}]", "tcl8.6");
    probe("o105_var_string", "set x 5\nputs \"val=$x\"", "tcl8.6");
    probe("o100_braced_var", "set x 7\nputs ${x}", "tcl8.6");
    probe("o105_braced_string", "set x 7\nputs \"${x}\"", "tcl8.6");
    probe("o105_call_barrier", "set x 5\nstring length abc\nputs \"val=$x\"", "tcl8.6");
    probe("o105_combine", "set x 5\nset y [expr {$x + 1}]\nputs \"x=$x\"\nputs $y", "tcl8.6");
    probe("o100_braced_word", "set msg {Hello World}\nputs $msg", "tcl8.6");
    probe("o100_metachars", "set x {a $b [c]}\nputs $x", "tcl8.6");
    probe("o100_arrayref", "set x ${a(1)}\nputs $x", "tcl8.6");
}

#[test]
fn probe_more_families() {
    // pattern match (f5-irules)
    probe("regex_both", "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api$} } { pool p1 } }", "f5-irules");
    probe("regex_start", "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api/} } { pool p1 } }", "f5-irules");
    probe("regex_end", "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {/health$} } { pool p1 } }", "f5-irules");
    probe("regex_none", "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {api} } { pool p1 } }", "f5-irules");
    probe("regex_dot", "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {.html$} } { pool p1 } }", "f5-irules");
    probe("glob_lead", "when HTTP_REQUEST { if { $host matches_glob {*.example.com} } { pool p1 } }", "f5-irules");
    probe("glob_trail", "when HTTP_REQUEST { if { $host matches_glob {api.example.*} } { pool p1 } }", "f5-irules");
    probe("glob_both", "when HTTP_REQUEST { if { $host matches_glob {*example*} } { pool p1 } }", "f5-irules");
    probe("glob_none", "when HTTP_REQUEST { if { $host matches_glob {www.example.com} } { pool p1 } }", "f5-irules");
    // strength reduction
    probe("pow_two", "if {$x ** 2} {}", "tcl8.6");
    probe("mod_pow2", "if {$x % 8} {}", "tcl8.6");
    // incr idiom
    probe("incr_add1", "proc foo {} {\n    set x 0\n    set x [expr {$x + 1}]\n}", "tcl8.6");
    probe("incr_addn", "proc foo {} {\n    set x 0\n    set x [expr {$x + 5}]\n}", "tcl8.6");
    probe("incr_subn", "proc foo {} {\n    set x 0\n    set x [expr {$x - 3}]\n}", "tcl8.6");
    // nested expr
    probe("unwrap_if", "if {[expr {$x + 1}]} {}", "tcl8.6");
    probe("unwrap_return", "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}", "tcl8.6");
    probe("unwrap_set", "proc f {x} {\n    set y [expr {[expr {$x * 2}]}]\n    return $y\n}", "tcl8.6");
    // list folding
    probe("list_single", "set x [list a]\nputs $x", "tcl8.6");
    probe("list_multi", "set x [list a b c]\nputs $x", "tcl8.6");
    // strlen zero
    probe("strlen_eq0", "if {[string length $s] == 0} {}", "tcl8.6");
    probe("strlen_ne0", "if {[string length $s] != 0} {}", "tcl8.6");
    probe("strlen_gt0", "if {[string length $s] > 0} {}", "tcl8.6");
    // string compare
    probe("o120_eq_lit", "if {$a == \"hello\"} {}", "tcl8.6");
    probe("o120_ne_sub", "set ok [expr {$a != \"hello\"}]", "tcl8.6");
    probe("o120_var_const_nonnum", "set a foo\nif {$a == $b} {}", "tcl8.6");
    probe("o120_mixed", "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}", "tcl8.6");
    probe("o120_varvar_const", "set a foo\nset b bar\nif {$a == $b} {}", "tcl8.6");
}

#[test]
fn probe_lindex_endoffset() {
    probe("lindex_lit", "set x [lindex {a b c} 1]\nputs $x", "tcl8.6");
    probe("lindex_end", "set x [lindex {x y z} end]\nputs $x", "tcl8.6");
    // O119 multi-set packing
    probe("o119_consec", "set a 1\nset b 2\nset c 3\neval {$a $b $c}", "tcl8.6");
    probe("o119_intersp", "set a 1\nputs hello\nset b 2\nputs world\nset c 3\neval {$a $b $c}", "tcl8.6");
    probe("o119_readbreaks", "set a 1\nputs $a\nset b 2\nset c 3\nset d 4\neval {$a $b $c $d}", "tcl8.6");
    probe("o119_toofew", "set a 1\nset b 2\nputs \"$a $b\"", "tcl8.6");
    probe("o119_tcl9", "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"", "tcl9.0");
    // O128 end-offset
    probe("o128_last", "set x [lindex $L [expr {[llength $L] - 1}]]", "tcl8.6");
    probe("o128_2ndlast", "set x [lindex $L [expr {[llength $L] - 2}]]", "tcl8.6");
    probe("o128_lrange", "set x [lrange $L 0 [expr {[llength $L] - 1}]]", "tcl8.6");
    probe("o128_lreplace", "set x [lreplace $L [expr {[llength $L] - 2}] [expr {[llength $L] - 1}] foo]", "tcl8.6");
    probe("o128_strindex", "set x [string index $s [expr {[string length $s] - 1}]]", "tcl8.6");
    probe("o128_strrange", "set x [string range $s 0 [expr {[string length $s] - 1}]]", "tcl8.6");
    probe("o128_qualified", "set x [lindex ${my::list} [expr {[llength ${my::list}] - 1}]]", "tcl8.6");
    probe("o128_toplevel", "puts [lindex $L [expr {[llength $L] - 3}]]", "tcl8.6");
    probe("o128_arr_match", "set x [lindex $a(1) [expr {[llength $a(1)] - 1}]]", "tcl8.6");
    probe("o128_multi_first", "set x [lindex $L [expr {[llength $L] - 1}] 0]", "tcl8.6");
}

#[test]
fn probe_tail_and_misc() {
    // tail call
    probe("tc_factorial", "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n", "tcl8.6");
    probe("tc_bare", "proc loop {items} {\n    if {[llength $items] == 0} {\n        return\n    }\n    puts [lindex $items 0]\n    loop [lrange $items 1 end]\n}\n", "tcl8.6");
    probe("tc_nontail", "proc fib {n} {\n    if {$n <= 1} {\n        return $n\n    }\n    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}\n}\n", "tcl8.6");
    probe("tc_gcd", "proc gcd {a b} {\n    if {$b == 0} {\n        return $a\n    } else {\n        return [gcd $b [expr {$a % $b}]]\n    }\n}\n", "tcl8.6");
    probe("o123_fact", "proc factorial {n} {\n    if {$n <= 1} {\n        return 1\n    }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n", "tcl8.6");
    probe("o122_arity", "proc f {a b} {\n    return [f $a]\n}\n", "tcl8.6");
    // unused procs (irules)
    probe("o124_unused", "proc helper {x} {\n    return $x\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}", "f5-irules");
    probe("o124_used", "proc helper {} {\n    return 1\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper]\n}", "f5-irules");
    probe("o124_plain", "proc unused {} {\n    return 1\n}\nputs hello", "tcl8.6");
    // code sinking
    probe("o125_basic", "set b foo\nif {$a} {\n    puts $b\n}", "tcl8.6");
    probe("o125_indent", "set b foo\nif {$a} {\n    if {$c} {\n        set b bar\n    }\n    if {$d} {\n        puts $b\n    }\n}", "tcl8.6");
    // load forwarding
    probe("o127_cmdsub", "proc test {} {\n    set x [clock seconds]\n    puts $x\n}", "tcl8.6");
    probe("o127_varcopy", "proc test {arg} {\n    set x $arg\n    puts $x\n}", "tcl8.6");
    probe("o127_const", "proc test {} {\n    set x 42\n    puts $x\n}", "tcl8.6");
}

#[test]
fn probe_multipass() {
    let registry = registry_for_dialect("tcl8.6");
    let src = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (out, opts) = optimise_source_multipass(src, registry, Some("tcl8.6"), 10);
    println!("=== multipass banner ===");
    println!("  OUT: {:?}", out);
    println!("  CODES: {:?}", opts.iter().map(|o| o.code.as_str().to_owned()).collect::<Vec<_>>());

    let src2 = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    let (out2, _) = optimise_source_multipass(src2, registry, Some("tcl8.6"), 10);
    println!("=== multipass nested ===");
    println!("  OUT: {:?}", out2);
}
