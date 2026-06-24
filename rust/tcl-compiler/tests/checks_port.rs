//! PROBE (temporary) — captures ground truth, will be replaced.
use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::registry_for_dialect;

const D: &str = "tcl8.6";

fn diags(src: &str, dialect: &str) -> Vec<(String, String, Severity, Vec<String>)> {
    let mut out: Vec<(String, String, Severity, Vec<String>)> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.message.clone(),
                d.severity,
                d.fixes.iter().map(|f| f.new_text.clone()).collect(),
            )
        })
        .collect();
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push((d.code.to_string(), d.message.clone(), d.severity, vec![]));
    }
    out
}

fn probe(label: &str, src: &str, dialect: &str, code: &str) {
    let ds = diags(src, dialect);
    let matched: Vec<_> = ds.iter().filter(|(c, _, _, _)| c == code).collect();
    eprintln!(
        "PROBE [{label}] code={code} dialect={dialect} count={} :: {:?}",
        matched.len(),
        matched
            .iter()
            .map(|(_, m, s, f)| format!("sev={s:?} fixes={f:?} msg={:.70}", m))
            .collect::<Vec<_>>()
    );
}

#[test]
fn probe_all() {
    // W100 unbraced expr
    probe("w100_expr_var", "expr $x + 1", D, "W100");
    probe("w100_if_quoted_lit", "if \"1 > 0\" {puts yes}", D, "W100");
    probe("w100_braced", "expr {$x + 1}", D, "W100");
    probe("w100_if_unbraced", "if $x {puts yes}", D, "W100");
    probe("w100_while_unbraced", "while $running {update}", D, "W100");
    probe("w100_for_test", "for {set i 0} $cond {incr i} {puts $i}", D, "W100");
    probe("w100_expr_42", "expr 42", D, "W100");
    probe("w100_if_true", "if true {puts yes}", D, "W100");
    probe("w100_cmd_subst", "expr [clock seconds]", D, "W100");
    probe("w100_quoted_fix", "set ok [expr \"$a == $b\"]", D, "W100");
    probe("w100_if_fix", "if $x {puts yes}", D, "W100");
    // W101 eval
    probe("w101_var", "eval \"puts $x\"", D, "W101");
    probe("w101_braced", "eval {puts hello}", D, "W101");
    probe("w101_multi_braced", "eval {set x 1} {puts $x}", D, "W101");
    probe("w101_cmdsubst", "eval [build_cmd]", D, "W101");
    probe("w101_literal", "eval puts hello", D, "W101");
    probe("w101_list", "eval [list set $varname $value]", D, "W101");
    probe("w101_concat", "eval [concat $script $args]", D, "W101");
    probe("w101_fix", "eval \"process $x\"", D, "W101");
    // W102 subst
    probe("w102_var", "subst $template", D, "W102");
    probe("w102_braced", "subst {hello $name}", D, "W102");
    probe("w102_flags", "subst -nobackslashes $template", D, "W102");
    probe("w102_nocommands", "subst -nocommands $template", D, "W102");
    probe("w102_noc_nov", "subst -nocommands -novariables $template", D, "W102");
    // W103 open
    probe("w103_lit_pipe", "open \"|ls -la\"", D, "W103");
    probe("w103_pipe_var", "open \"|$cmd\"", D, "W103");
    probe("w103_var", "open $filename", D, "W103");
    probe("w103_lit_file", "open \"data.txt\"", D, "W103");
    // W104
    probe("w104_lead_space", "append items \" item\"", D, "W104");
    probe("w104_normal", "append result \"value\"", D, "W104");
    probe("w104_sep", "append msg \", \"", D, "W104");
    probe("w104_genuine", "append items \" $item\"", D, "W104");
    probe("w104_usage", "append result \" ?option value?...\"", D, "W104");
    // W110
    probe("w110_eq", "if {$x == \"foo\"} {puts yes}", D, "W110");
    probe("w110_ne", "if {$x != \"bar\"} {puts no}", D, "W110");
    probe("w110_num", "if {$x == 42} {puts yes}", D, "W110");
    probe("w110_varonly", "if {$a == $b} {puts yes}", D, "W110");
    probe("w110_numstr", "if {$x == \"42\"} {puts yes}", D, "W110");
    probe("w110_neg", "if {!($x == \"foo\")} {puts no}", D, "W110");
    probe("w110_mixed", "if {$a == $b || $x == \"foo\"} {puts y}", D, "W110");
    // W201 path concat
    probe("w201_pathvar", "set path \"$dir/file.txt\"", D, "W201");
    probe("w201_lit", "set path \"/etc/config\"", D, "W201");
    probe("w201_url", "set u http://example.com/$page", D, "W201");
    probe("w201_genuine", "set p \"$dir/$file\"", D, "W201");
    // W300 source
    probe("w300_var", "source $script_path", D, "W300");
    probe("w300_lit", "source \"lib/utils.tcl\"", D, "W300");
    probe("w300_enc", "source -encoding utf-8 $path", D, "W300");
    // W301 uplevel
    probe("w301_unbraced", "uplevel 1 \"set x $y\"", D, "W301");
    probe("w301_braced", "uplevel 1 {set x 1}", D, "W301");
    probe("w301_multi", "uplevel 1 set x $y", D, "W301");
    probe("w301_list", "uplevel 1 [list set $varname $value]", D, "W301");
    probe("w301_barevar", "proc f {body} { uplevel 1 $body }", D, "W301");
    // W302 catch
    probe("w302_noresult", "catch {error oops}", D, "W302");
    probe("w302_result", "catch {error oops} result", D, "W302");
    probe("w302_aftercancel", "catch {after cancel $h}", D, "W302");
    probe("w302_filedelete", "catch {file delete $f}", D, "W302");
    probe("w302_genuine", "catch {parse_user_input $x}", D, "W302");
    probe("w302_chanconfig", "catch {chan configure $h -buffering none}", D, "W302");
    probe("w302_dictset", "catch {dict set d k v}", D, "W302");
    probe("w302_dictunset", "catch {dict unset d k}", D, "W302");
    // W303 ReDoS
    probe("w303_nested_q", "regexp {(a+)+} $input", D, "W303");
    probe("w303_safe", "regexp {^\\d+$} $input", D, "W303");
    probe("w303_switch", "switch -regexp $x { {(a+)+} {puts bad} {^ok} {puts ok} }", D, "W303");
    probe("w303_switch_glob", "switch -glob $x { (a+)+ {puts bad} }", D, "W303");
    // W304 option terminator
    probe("w304_re_var", "regexp $pattern $text", D, "W304");
    probe("w304_re_safe_paren", "regexp {(a+)+$} $text", D, "W304");
    probe("w304_re_dash", "regexp {-[0-9]+} $text", D, "W304");
    probe("w304_re_term", "regexp -- $pattern $text", D, "W304");
    probe("w304_exec", "exec $cmd", D, "W304");
    probe("w304_subst", "subst $template", D, "W304");
    probe("w304_filedelete", "file delete $path", D, "W304");
    probe("w304_unset", "unset $varname", D, "W304");
    probe("w304_lsearch", "lsearch -exact $domain c", D, "W304");
    // W306 literal expected
    probe("w306_bare_var", "regexp -- $pattern $text", D, "W306");
    probe("w306_cmd", "regsub -- [build_re] $text X out", D, "W306");
    probe("w306_charclass", "regexp -- [a-z] $text", D, "W306");
    probe("w306_concat", "regexp -- $pattern$suffix $text", D, "W306");
    probe("w306_quoted_var", "regexp -- \"$pattern\" $text", D, "W306");
    probe("w306_dollar_anchor", "regexp -- \"^foo$\" $text", D, "W306");
    // W307 non-literal command
    probe("w307_var", "$cmd arg1 arg2", D, "W307");
    probe("w307_cmdsubst", "[get_cmd] arg1", D, "W307");
    probe("w307_lit", "puts hello", D, "W307");
    // W105 / W106 unbraced body
    probe("w105_if_var", "if {1} \"puts $x\"", D, "W105");
    probe("w105_eval_var", "eval $script", D, "W105");
    probe("w106_switch_var", "switch -- $x a \"puts $y\" b {puts ok}", D, "W106");
    // W108 non-ASCII
    probe("w108_smart", "set x \u{201c}hello\u{201d}", D, "W108");
    probe("w108_cyrillic", "set x \u{0410}bc", D, "W108");
    probe("w108_copyright", "set x \u{00a9}value", D, "W108");
    probe("w108_degree", "set x {90\u{00b0}}", D, "W108");
    // W308 subst-nocommands & tcloo method
    probe("w308_subst_bracket", "subst {hello [clock seconds]}", D, "W308");
    probe("w308_subst_noc", "subst -nocommands {hello [clock seconds]}", D, "W308");
    // W309 double subst
    probe("w309_eval_subst", "eval [subst $template]", D, "W309");
    probe("w309_uplevel_subst", "uplevel 1 [subst $template]", D, "W309");
    probe("w309_eval_clean", "eval {puts hello}", D, "W309");
    // W212 name vs value
    probe("w212_set", "set $x 1", D, "W212");
    probe("w212_incr", "incr $count", D, "W212");
    probe("w212_append", "append $buf \"text\"", D, "W212");
    probe("w212_unset", "unset $x", D, "W212");
    probe("w212_unset_multi", "unset $a $b", D, "W212");
    probe("w212_infoexists", "info exists $x", D, "W212");
    probe("w212_infocommands", "info commands $pattern", D, "W212");
    probe("w212_upvar_local", "upvar 1 other $local", D, "W212");
    probe("w212_upvar_remote", "upvar 1 $remote local", D, "W212");
    probe("w212_upvar_multi", "upvar 1 a $b c $d", D, "W212");
    // E100 unmatched bracket
    probe("e100_bare", "set x foo]", D, "E100");
    probe("e100_matched", "set x [string length foo]", D, "E100");
    probe("e100_braces", "set x {contains ]}", D, "E100");
    probe("e100_blah", "set x blah]", D, "E100");
    // W213 unset nocomplain
    probe("w213_undef", "unset x", D, "W213");
    probe("w213_noc", "unset -nocomplain x", D, "W213");
    probe("w213_def", "set x 1\nunset x", D, "W213");
    // W121 subnet mask
    probe("w121_invalid", "set mask 255.255.255.1", D, "W121");
    probe("w121_valid", "set mask 255.255.255.0", D, "W121");
    // W122/W124 IP
    probe("w122_over255", "set addr 192.168.1.256", D, "W122");
    probe("w124_over255", "set addr 192.168.1.256", D, "W124");
    probe("w122_leadzero", "set addr 192.168.01.1", D, "W122");
    probe("w122_valid", "set addr 192.168.1.1", D, "W122");
    probe("w124_valid", "set addr 192.168.1.1", D, "W124");
    probe("w122_ldapoid", "set oid 1.3.6.1.4.1.4203.1.11.3", D, "W122");
    probe("w124_in_proc", "proc f {} { set a 192.168.1.256 }", D, "W124");
    // W126 channel
    probe("w126_int", "proc f {} { set x 42; close $x }", D, "W126");
    probe("w126_open", "proc f {} { set fd [open /tmp/t r]; gets $fd line; close $fd }", D, "W126");
    probe("w126_literal", "proc f {} { gets notachannel line }", D, "W126");
    probe("w126_list", "proc f {} { set l [list a b]; close $l }", D, "W126");
    probe("w126_param", "proc f {fd} { close $fd }", D, "W126");
    // W125 orphaned keyword
    probe("w125_else", "if {1} {\n    puts yes\n}\nelse {\n    puts no\n}\n", D, "W125");
    probe("w125_elseif", "if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\n", D, "W125");
    probe("w125_correct", "if {1} {\n    puts yes\n} else {\n    puts no\n}\n", D, "W125");
    probe("w125_proc_else", "proc else {x} { puts $x }\nelse hello\n", D, "W125");
    // W113 shadow
    probe("w113_set", "proc set {a b} {}", D, "W113");
    probe("w113_exit", "proc exit {} {}", D, "W113");
    probe("w113_ns", "proc ::snit::type {name def} {}", D, "W113");
    // W310 credentials
    probe("w310_password", "http::geturl $url -password \"s3cret\"", D, "W310");
    probe("w310_password_var", "http::geturl $url -password $pw", D, "W310");
    probe("w310_token", "some_command -token \"abc123def456\"", D, "W310");
    // W311 encoding
    probe("w311_mismatch", "fconfigure $fd -encoding binary -translation auto", D, "W311");
    probe("w311_binbin", "fconfigure $fd -encoding binary -translation binary", D, "W311");
    probe("w311_chan", "chan configure $fd -encoding binary -translation crlf", D, "W311");
    // W312 interp eval
    probe("w312_unbraced", "interp eval $child \"set x $y\"", D, "W312");
    probe("w312_braced", "interp eval $child {set x 1}", D, "W312");
    probe("w312_multi", "interp eval $child set x $y", D, "W312");
    probe("w312_list", "interp eval $child [list set $varname $value]", D, "W312");
    // W313 destructive file
    probe("w313_delete", "set path /tmp/x\nfile delete $path", D, "W313");
    probe("w313_lit", "file delete /tmp/myfile.txt", D, "W313");
    probe("w313_mkdir", "set dir /tmp/x\nfile mkdir $dir", D, "W313");
    probe("w313_exists", "set path /tmp/x\nfile exists $path", D, "W313");
    // W200 binary format - dialect sensitive
    probe("w200_su_84", "binary format su $val", "tcl8.4", "W200");
    probe("w200_su_86", "binary format su $val", "tcl8.6", "W200");
    probe("w200_s_84", "binary format s $val", "tcl8.4", "W200");
    probe("w200_scan_84", "binary scan $data su x", "tcl8.4", "W200");
    probe("w200_su_irules", "binary format su $val", "f5-irules", "W200");
    // iRules codes
    probe("irule2002_clientaddr", "client_addr", "f5-irules", "IRULE2002");
    probe("irule2002_redirect", "redirect \"http://example.com\"", "f5-irules", "IRULE2002");
    probe("irule2002_tcl86", "set x 1", "tcl8.6", "IRULE2002");
    probe("irule2003_uplevel", "uplevel 1 {set x 1}", "f5-irules", "IRULE2003");
    probe("irule2003_history", "history", "f5-irules", "IRULE2003");
    probe("irule2003_tcl86", "uplevel 1 {set x 1}", "tcl8.6", "IRULE2003");
    probe("irule3102_path", "set p [HTTP::path]", "f5-irules", "IRULE3102");
    probe("irule3102_path_norm", "set p [HTTP::path -normalized]", "f5-irules", "IRULE3102");
    probe("irule1002_unknown", "when FAKE_EVENT {puts hi}", "f5-irules", "IRULE1002");
    probe("irule1002_known", "when HTTP_REQUEST {puts hi}", "f5-irules", "IRULE1002");
    // Integration
    probe("integ_eval", "\nproc dangerous {input} {\n    set result [eval \"process $input\"]\n    expr $result + 1\n    catch {risky_op}\n}\n", D, "W101");
    probe("integ_expr", "\nproc dangerous {input} {\n    set result [eval \"process $input\"]\n    expr $result + 1\n    catch {risky_op}\n}\n", D, "W100");
    probe("integ_catch", "\nproc dangerous {input} {\n    set result [eval \"process $input\"]\n    expr $result + 1\n    catch {risky_op}\n}\n", D, "W302");
    // empty / comments
    probe("empty", "", D, "W100");
    probe("expr_noargs_e002", "expr", D, "E002");
    probe("expr_noargs_w100", "expr", D, "W100");
    // W214 dispatch protocol & empty body
    probe("w214_emptybody", "proc reverse {fa} {}\n", D, "W214");
    probe("w214_realbody", "proc foo {x y} { puts hello }\n", D, "W214");
    probe("w214_switch_subj", "proc editStartCmd {tbl row col text} {\n    switch -- $col {\n        1 { set combList columns }\n        default { return $text }\n    }\n    return $combList\n}\n", D, "W214");
    probe("w214_protocol", "namespace eval ::g {\n    proc ALNUM {s e} { return alnum }\n    proc ALPHA {s e} { return alpha }\n    proc DIGIT {s e} { return digit }\n    proc walk {rule s e} { $rule $s $e }\n}\n", D, "W214");
    // W210 patterns
    probe("w210_catch_body", "if {![catch {set x 1}]} { puts $x }", D, "W210");
    probe("w210_info_exists", "proc f {} { if {![info exists ns]} { set ns 1 }\n return $ns }", D, "W210");
    probe("w210_return_unset", "proc f {} { return $x }", D, "W210");
    probe("w210_return_set", "proc f {} { set x 1\n return $x }", D, "W210");
    probe("w210_try_handler", "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}", D, "W210");
    // W220 array element
    probe("w220_distinct_elem", "proc f {} { set a(k) 1\n set a(j) 2\n puts $a(k) }", D, "W220");
    probe("w220_unused_elem", "proc f {} { set a(k) 1\n return 0 }", D, "W220");
    probe("w220_scalar", "proc f {} { set x 1\n set x 2\n return $x }", D, "W220");
}

#[test]
fn probe2() {
    fn d(src: &str, code: &str) -> usize {
        diags(src, D).iter().filter(|(c,_,_,_)| c==code).count()
    }
    eprintln!("P2 catch_body_toplevel W210 = {}", d("if {![catch {set x 1}]} { puts $x }", "W210"));
    eprintln!("P2 catch_body_inproc W210 = {}", d("proc f {} { if {![catch {set x 1}]} { puts $x } }", "W210"));
    eprintln!("P2 catch_body_gets W210 = {}", d("if {![catch {gets stdin line}]} { puts $line }", "W210"));
    eprintln!("P2 catch_body_gets_inproc W210 = {}", d("proc f {} { if {![catch {gets stdin line}]} { puts $line } }", "W210"));
    eprintln!("P2 sc_and_true W210 = {}", d("if {1 && ![catch {set x 1}]} { puts $x }", "W210"));
    // W308 multi: tcloo unknown method
    eprintln!("P2 tcloo_unknown_method W308 = {}", d("oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj nonexistent_method\n", "W308"));
    eprintln!("P2 tcloo_destroy W308 = {}", d("oo::class create MyClass {\n    method greet {} { puts hello }\n}\nset obj [MyClass new]\n$obj destroy\n", "W308"));
    // W308 subst-nocommands? confirm it is NOT this
    eprintln!("P2 subst_bracket W308 = {} (expect 0, W308 is tcloo)", d("subst {hello [clock seconds]}", "W308"));
    // W125 then/finally/on/trap
    eprintln!("P2 w125_then = {}", d("if {1}\nthen {\n    puts yes\n}\n", "W125"));
    eprintln!("P2 w125_finally = {}", d("try {\n    error oops\n}\nfinally {\n    puts cleanup\n}\n", "W125"));
    eprintln!("P2 w125_on = {}", d("try {\n    error oops\n}\non error {msg opts} {\n    puts $msg\n}\n", "W125"));
    eprintln!("P2 w125_trap = {}", d("try {\n    error oops\n}\ntrap {} e {\n    puts caught\n}\n", "W125"));
    eprintln!("P2 w125_proc_finally = {}", d("proc finally {body} { uplevel 1 $body }\nfinally { puts done }\n", "W125"));
    eprintln!("P2 w125_multiple = {}", d("if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\nelse {\n    puts c\n}\n", "W125"));
    // W104 sep variants
    eprintln!("P2 w104_colon_sep = {}", d("append msg \": \"", "W104"));
    eprintln!("P2 w104_combined = {}", d("append combined $value {, }", "W104"));
    eprintln!("P2 w104_newline = {}", d("append msg \"  $dirs\\n\\n\"", "W104"));
    eprintln!("P2 w104_lappend = {}", d("lappend items item", "W104"));
    eprintln!("P2 w104_placeholder = {}", d("append name \" <value>\"", "W104"));
    // W306 quoted live dollar
    eprintln!("P2 w306_live_dollar = {}", d("regexp -- \"^foo$bar\" $text", "W306"));
    // E100 cmd subst proper
    eprintln!("P2 e100_unknown_proc = {}", d("set username unknown_proc arg1 arg2]", "E100"));
    // W124 leading zero severity & message
    let lz = diags("set addr 192.168.01.1", D);
    eprintln!("P2 w124_leadzero: {:?}", lz.iter().filter(|(c,_,_,_)| c=="W124").map(|(_,m,s,_)| format!("{s:?} {:.40}", m)).collect::<Vec<_>>());
    // W122 vs W124 at top level
    eprintln!("P2 toplevel_192.168.1.256 W122={} W124={}", d("set addr 192.168.1.256", "W122"), d("set addr 192.168.1.256", "W124"));
    eprintln!("P2 puts_mask W121={}", d("puts \"mask is 255.255.255.1\"", "W121"));
    eprintln!("P2 puts_badip W122={} W124={}", d("puts \"mask is 255.255.255.1\"", "W122"), d("set x \"prefix 192.168.1.999 suffix\"", "W124"));
    // W214 args param
    eprintln!("P2 w214_args = {}", d("proc foo {args} { puts hello }", "W214"));
    // W110 has fix on simple
    let f = diags("if {$x == \"foo\"} {puts yes}", D);
    eprintln!("P2 w110_fixcount = {}", f.iter().find(|(c,_,_,_)| c=="W110").map(|(_,_,_,fx)| fx.len()).unwrap_or(99));
    // empty / comments
    eprintln!("P2 comments_total = {}", diags("# just a comment\n# another comment", D).len());
    eprintln!("P2 empty_total = {}", diags("", D).len());
}

#[test]
fn probe3() {
    fn d(src: &str, code: &str) -> usize { diags(src, D).iter().filter(|(c,_,_,_)| c==code).count() }
    fn dd(src: &str, dia: &str, code: &str) -> usize { diags(src, dia).iter().filter(|(c,_,_,_)| c==code).count() }
    // W307 snit/tcloo suppressions
    eprintln!("P3 tcloo_var_cmd W307 = {}", d("oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj greet world\n", "W307"));
    eprintln!("P3 foreach_known W307 = {}", d("foreach cmd [list puts set expr] {\n    $cmd hello\n}\n", "W307"));
    eprintln!("P3 foreach_unknown W307 = {}", d("foreach cmd [list unknown_cmd_a unknown_cmd_b] {\n    $cmd value\n}\n", "W307"));
    eprintln!("P3 param_dispatcher W307 = {}", d("proc once {tree} { $tree visit }", "W307"));
    eprintln!("P3 local_dispatcher W307 = {}", d("proc f {} {\n    set foo [some_factory]\n    $foo method\n}", "W307"));
    // W306 bare var in proc body
    eprintln!("P3 w306_bare_proc = {}", d("proc f {} {\n    if {1} {\n        foreach x [list a b] {\n            regexp $pattern $x\n        }\n    }\n}\n", "W306"));
    // W214 protocol w/ dispatcher
    eprintln!("P3 w214_2peer s/e = {}", d("namespace eval ::g {\n    proc rule_a {s e} { return alnum }\n    proc rule_b {s e} { return alpha }\n}\n", "W214"));
    eprintln!("P3 w214_3peer_nodispatch token = {}", d("namespace eval ::n {\n    proc a {ctx token} { puts $ctx }\n    proc b {ctx token} { puts $ctx }\n    proc c {ctx token} { puts $ctx }\n}\n", "W214"));
    // E100 fix presence
    let e = diags("set x blah]", D);
    eprintln!("P3 e100_blah_fixes = {:?}", e.iter().find(|(c,_,_,_)| c=="E100").map(|(_,_,_,f)| f.len()));
    // W113 namespaced lib not flagged
    eprintln!("P3 w113_base64 = {}", d("namespace eval base64 { proc encode {s} {return $s} }", "W113"));
    // W125 multiple-codes
    let m = diags("if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\nelse {\n    puts c\n}\n", D);
    eprintln!("P3 w125_multiple msgs = {:?}", m.iter().filter(|(c,_,_,_)| c=="W125").map(|(_,msg,_,_)| {
        // extract keyword between quotes
        msg.split('"').nth(1).unwrap_or("?").to_string()
    }).collect::<Vec<_>>());
    // Integration clean proc - count W codes excluding W123
    let clean = diags("\nproc safe {x} {\n    set result [expr {$x * 2}]\n    if {$result > 10} {\n        puts \"big\"\n    }\n    catch {risky_op} err\n    return $result\n}\n", D);
    eprintln!("P3 clean_proc_wcodes = {:?}", clean.iter().filter(|(c,_,_,_)| c.starts_with('W') && c != "W123").map(|(c,_,_,_)| c.clone()).collect::<Vec<_>>());
    // security critical
    let sec = diags("\nset script \"doThing $userArg\"\neval $script\nsource $untrusted_path\nsubst $user_template\nopen \"|$user_cmd\"\n", D);
    eprintln!("P3 security_codes = {:?}", { let mut v: Vec<_> = sec.iter().map(|(c,_,_,_)| c.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect(); v.sort(); v });
    // no false positives standard
    let std = diags("\nset x 42\nset y [expr {$x + 1}]\nif {$y > 40} {\n    puts \"yes\"\n} else {\n    puts \"no\"\n}\nwhile {$x > 0} {\n    incr x -1\n}\nfor {set i 0} {$i < 10} {incr i} {\n    lappend result $i\n}\nforeach item {a b c} {\n    puts $item\n}\ncatch {risky} result opts\nopen {data.txt} r\nsource {lib/utils.tcl}\neval {set a 1}\n", D);
    eprintln!("P3 std_wcodes = {:?}", std.iter().filter(|(c,_,_,_)| c.starts_with('W') && c != "W123").map(|(c,_,_,_)| c.clone()).collect::<Vec<_>>());
    // W210 try-handler genuine warns
    eprintln!("P3 try_genuine W210 = {}", d("proc f {} {\n    try {\n        error boom\n    } on error {} {\n        puts $y\n    }\n}", "W210"));
    eprintln!("P3 try_earlier_throw W210 = {}", d("proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}", "W210"));
    // W212 unset terminator
    eprintln!("P3 w212_unset_term = {}", d("unset -nocomplain -- $x", "W212"));
    // W304 table/class under irules
    eprintln!("P3 w304_table = {}", dd("table set $key $value", "f5-irules", "W304"));
    eprintln!("P3 w304_classmatch = {}", dd("class match $item starts_with my_class", "f5-irules", "W304"));
    // E100 cmdsub with f5
    eprintln!("P3 e100_access = {}", dd("switch -- ACCESS::policy result]", "f5-irules", "E100"));
    // W306 class match literal under irules
    eprintln!("P3 w306_classmatch = {}", dd("class match -- [HTTP::uri] ends_with {my_class}", "f5-irules", "W306"));
    // IRULE3102 setter not flagged
    eprintln!("P3 irule3102_setter = {}", dd("HTTP::path /lowercase/path", "f5-irules", "IRULE3102"));
    // W310 sensitive header irules
    eprintln!("P3 w310_authheader = {}", dd("HTTP::header insert Authorization \"Bearer hardcoded\"", "f5-irules", "W310"));
    eprintln!("P3 w310_ctheader = {}", dd("HTTP::header insert Content-Type \"text/html\"", "f5-irules", "W310"));
    // structural body doesn't leak
    let leak = diags("proc outer {p} {\n    set x 1\n    puts $y\n    oo::class create C { method m {} { puts $x; set y 1; set p 2 } }\n}\n", D);
    eprintln!("P3 structural_leak codes = {:?}", { let mut v: Vec<_> = leak.iter().map(|(c,_,_,_)| c.clone()).filter(|c| c=="W210"||c=="W211"||c=="W214").collect::<std::collections::HashSet<_>>().into_iter().collect(); v.sort(); v });
}

#[test]
fn probe4() {
    fn show(src: &str) -> Vec<String> {
        let mut v: Vec<String> = diags(src, D).iter().map(|(c,m,_,_)| format!("{c}:{:.45}", m)).collect();
        v.sort(); v
    }
    eprintln!("P4 structural_leak ALL = {:?}", show("proc outer {p} {\n    set x 1\n    puts $y\n    oo::class create C { method m {} { puts $x; set y 1; set p 2 } }\n}\n"));
    // Does plain `set x 1; (never read)` in a proc fire W211?
    eprintln!("P4 simple_unused = {:?}", show("proc outer {p} {\n    set x 1\n}\n"));
    // W210 catch body — full diag list
    eprintln!("P4 catchbody ALL = {:?}", show("if {![catch {set x 1}]} { puts $x }"));
    // foreach over known commands - full
    eprintln!("P4 foreach_known ALL = {:?}", show("foreach cmd [list puts set expr] {\n    $cmd hello\n}\n"));
    // std integration: which vars get W211/W220
    eprintln!("P4 std_integration = {:?}", show("\nset x 42\nset y [expr {$x + 1}]\nif {$y > 40} {\n    puts \"yes\"\n} else {\n    puts \"no\"\n}\nwhile {$x > 0} {\n    incr x -1\n}\nfor {set i 0} {$i < 10} {incr i} {\n    lappend result $i\n}\nforeach item {a b c} {\n    puts $item\n}\ncatch {risky} result opts\nopen {data.txt} r\nsource {lib/utils.tcl}\neval {set a 1}\n"));
}

#[test]
fn probe5() {
    fn d(src: &str, code: &str) -> usize { diags(src, D).iter().filter(|(c,_,_,_)| c==code).count() }
    eprintln!("P5 w214_protocol_dispatch = {}", d("namespace eval ::g {\n    proc ALNUM {s e} { return alnum }\n    proc ALPHA {s e} { return alpha }\n    proc DIGIT {s e} { return digit }\n    proc walk {rule s e} { $rule $s $e }\n}\n", "W214"));
    eprintln!("P5 w214_single = {}", d("proc foo {x y unused} { puts $x; puts $y }\n", "W214"));
    eprintln!("P5 w214_emptybody_defaults = {}", d("proc complete {fa {sink {}}} {}\n", "W214"));
    // W302 multi-command / package require / namespace eval
    eprintln!("P5 w302_multicmd = {}", d("catch {parse_x; parse_y}", "W302"));
    eprintln!("P5 w302_pkgreq = {}", d("catch {package require Tk}", "W302"));
    eprintln!("P5 w302_nseval = {}", d("catch {namespace eval ::ns {}}", "W302"));
    eprintln!("P5 w302_close = {}", d("catch {close $fh}", "W302"));
    eprintln!("P5 w302_qualclose = {}", d("catch {::close $h}", "W302"));
    eprintln!("P5 w302_arrayunset = {}", d("catch {array unset a *}", "W302"));
    eprintln!("P5 w302_renameempty = {}", d("catch {rename foo \"\"}", "W302"));
    eprintln!("P5 w302_filecopy = {}", d("catch {file copy a b}", "W302"));
    eprintln!("P5 w302_interpcreate = {}", d("catch {interp create slave}", "W302"));
    eprintln!("P5 w302_aftertimer = {}", d("catch {after 1000 puts hi}", "W302"));
    eprintln!("P5 w302_nsdelete = {}", d("catch {namespace delete ::ns}", "W302"));
    eprintln!("P5 w302_unset = {}", d("catch {unset var}", "W302"));
    eprintln!("P5 w302_chanclose = {}", d("catch {chan close $h}", "W302"));
    eprintln!("P5 w302_interpdelete = {}", d("catch {interp delete slave}", "W302"));
    // W301 list idioms
    eprintln!("P5 w301_lsort = {}", d("uplevel 1 [lsort $cmdlist]", "W301"));
    eprintln!("P5 w301_nscur = {}", d("set ns [uplevel 1 [list ::namespace current]]", "W301"));
    eprintln!("P5 w301_concat = {}", d("uplevel 1 [concat $script $args]", "W301"));
    eprintln!("P5 w301_bracedvar = {}", d("proc f {body} { uplevel 1 ${body} }", "W301"));
    eprintln!("P5 w301_quoted = {}", d("proc f {x} { uplevel 1 \"set y $x\" }", "W301"));
    eprintln!("P5 w301_concatvars = {}", d("proc f {x y} { uplevel 1 $x$y }", "W301"));
    // W210 globals written by procs
    eprintln!("P5 w210_proc_global = {}", diags("proc set_name {v} {global package_name; set package_name $v}\nsource foo.tcl\nset out $package_name\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("package_name")).count());
    eprintln!("P5 w210_proc_global_noset = {}", diags("proc reader {} {global package_name; return $package_name}\nset out $package_name\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("package_name")).count());
    eprintln!("P5 w210_unset_global = {}", diags("proc clear {} {unset ::x}\nputs $x\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("'x'")).count());
    // info exists / array exists guard
    eprintln!("P5 w210_info_exists_assigned = {}", d("proc f {} { set y [info exists x]\n return $y }", "W210"));
    eprintln!("P5 w210_array_exists = {}", d("proc f {} { if {[array exists arr]} { return [array size arr] }\n return 0 }", "W210"));
    // qualified variable alias
    eprintln!("P5 w210_qual_var = {}", d("proc f {} {\n    variable ::tcl::WordBreakRE\n    regexp -indices -- $WordBreakRE(after) abc result\n    return $result\n}", "W210"));
    eprintln!("P5 w210_genuine_local = {}", d("proc f {} {\n    set kids $children\n    return $kids\n}", "W210"));
    // uplevel bare var not injection
    eprintln!("P5 w301_arrayelem = {}", d("proc f {} { uplevel 1 $cmds(init) }", "W301"));
    eprintln!("P5 w301_vardotliteral = {}", d("proc f {x} { uplevel 1 $x.foo }", "W301"));
    // array element dead store distinction
    eprintln!("P5 w220_array_built = {}", d("proc f {} { set o(-mode) cbc\n set o(-key) k\n return [array get o] }", "W220"));
    // read-modify-write in cmdsub
    eprintln!("P5 w220_incr_cmdsub = {}", d("proc f {} { set i 0\n foreach j {1 2 3} { lappend r [incr i $j] }\n return $r }", "W220"));
    eprintln!("P5 w211_incr_cmdsub = {}", d("proc f {} { set i 0\n foreach j {1 2 3} { lappend r [incr i $j] }\n return $r }", "W211"));
    // expr substitution read tracking
    eprintln!("P5 w220_incr_expr = {}", d("proc f {} { set w 5; set i 0; incr i [expr {$w}]; return $i }", "W220"));
    eprintln!("P5 w211_var_cmdsub_expr = {}", d("proc f {n} { set scale 3; return [expr {$n * $scale}] }", "W211"));
    // eval body read tracking
    eprintln!("P5 w220_eval_braced = {}", d("proc f {} { set x 1; eval {puts $x} }", "W220"));
    eprintln!("P5 w211_eval_nobody = {}", d("proc f {} { set x 1; eval {puts hi} }", "W211"));
    // namespace eval body scope
    eprintln!("P5 w214_ns_eval = {}", diags("proc g {x} { namespace eval ::ns { puts \"hello $x\" } }", D).iter().filter(|(c,m,_,_)| c=="W214"&&m.contains("'x'")).count());
    eprintln!("P5 w214_plain_eval = {}", diags("proc g {x} { eval { puts \"hello $x\" } }", D).iter().filter(|(c,m,_,_)| c=="W214"&&m.contains("'x'")).count());
}

#[test]
fn probe6() {
    fn d(src: &str, code: &str) -> usize { diags(src, D).iter().filter(|(c,_,_,_)| c==code).count() }
    // unset global via alias
    eprintln!("P6 w210_unset_alias = {}", diags("proc clear {} {global x; unset x}\nputs $x\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("'x'")).count());
    eprintln!("P6 w210_unset_direct = {}", diags("proc clear {} {unset ::x}\nputs $x\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("'x'")).count());
    eprintln!("P6 w210_incr_global = {}", diags("proc bump {} {global counter; incr counter}\nputs $counter\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("counter")).count());
    eprintln!("P6 w210_unrelated_global = {}", diags("proc set_name {v} {global package_name; set package_name $v}\nputs $other_var\n", D).iter().filter(|(c,m,_,_)| c=="W210"&&m.contains("other_var")).count());
    // tcloo W308 chained / objdefine / method unknown
    eprintln!("P6 w308_chained = {}", d("oo::class create Dog {\n    method bark {} { puts woof }\n}\n[Dog new] nonexistent\n", "W308"));
    eprintln!("P6 w308_method_unknown = {}", d("oo::class create Proxy {\n    method unknown {name args} { puts $name }\n}\nset p [Proxy new]\n$p anyMethod\n", "W308"));
    eprintln!("P6 w308_objdefine = {}", d("oo::class create Base {\n    method existing {} { puts hello }\n}\nset obj [Base new]\noo::objdefine $obj method custom {} { puts custom }\n$obj custom\n", "W308"));
    eprintln!("P6 w308_external = {}", d("set circuit [Circuit new {Monte-Carlo}]\n$circuit runAndRead\n", "W308"));
    // W123 oo class name as command
    eprintln!("P6 w123_oo_class = {}", d("oo::class create Logger {\n    method add {msg} { puts $msg }\n}\nset log [Logger new]\n", "W123"));
    // W307 external class new fires
    eprintln!("P6 w307_external_new = {}", d("set circuit [Circuit new {Monte-Carlo}]\n$circuit runAndRead\n", "W307"));
    // W306 quoted dollar anchor edge already done; check w306 quoted-with-var message contains quotes
    let q = diags("regexp -- \"$pattern\" $text", D);
    eprintln!("P6 w306_quoted_msg = {:?}", q.iter().find(|(c,_,_,_)| c=="W306").map(|(_,m,_,_)| m.to_lowercase().contains("quotes")));
    // W110 has fix new_text contains eq
    let f = diags("if {$x == \"foo\"} {puts yes}", D);
    eprintln!("P6 w110_fix = {:?}", f.iter().find(|(c,_,_,_)| c=="W110").map(|(_,_,_,fx)| fx.first().map(|s| s.contains("eq"))));
    // W100 fix new_text
    let w = diags("expr $x + 1", D);
    eprintln!("P6 w100_fix = {:?}", w.iter().find(|(c,_,_,_)| c=="W100").map(|(_,_,_,fx)| fx.first().cloned()));
    // W100 if fix exact {$x}
    let wi = diags("if $x {puts yes}", D);
    eprintln!("P6 w100_if_fix = {:?}", wi.iter().find(|(c,_,_,_)| c=="W100").map(|(_,_,_,fx)| fx.first().cloned()));
    // W101 eval fix exact
    let we = diags("eval \"process $x\"", D);
    eprintln!("P6 w101_fix = {:?}", we.iter().find(|(c,_,_,_)| c=="W101").map(|(_,_,_,fx)| fx.first().cloned()));
}
