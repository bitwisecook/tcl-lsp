#!/usr/bin/env tclsh
# Comprehensive Tcl 9.0.3 corner-case probe.  Each "probe" runs an
# isolated snippet via apply, captures the result/error, and reports.
# Output is structured so a Python regression test can parse it and
# cross-check our analyser/completion behaviour.

set ::PASS 0
set ::FAIL 0
set ::SKIP 0
set ::OUTCOMES [list]

proc probe {label script {expected ""}} {
    set rc [catch {apply [list {} $script]} result options]
    set status [expr {$rc == 0 ? "OK" : "ERR"}]
    set ec ""
    if {$rc != 0} {
        catch {set ec [dict get $options -errorcode]}
    }
    set first_line [lindex [split $result \n] 0]
    set out "$label\t$status\t"
    if {$rc == 0} {
        append out "result={$result}"
    } else {
        append out "err={$first_line}\tcode={$ec}"
    }
    puts $out
    lappend ::OUTCOMES [list $label $status $rc $result $ec]
    if {$expected eq "*"} {
        incr ::SKIP
    } elseif {$expected ne ""} {
        if {$rc == 0 && $result eq $expected} {
            incr ::PASS
        } else {
            incr ::FAIL
            puts "    !!! expected={$expected}"
        }
    }
}

# =============================================================
# §A. Variable substitution forms
# =============================================================

probe A.bare-simple        {set foo 42; set v $foo; set v}                                    42
probe A.bare-underscore    {set foo_bar 1; set v $foo_bar; set v}                             1
probe A.bare-digit-after   {set foo123 1; set v $foo123; set v}                               1
probe A.bare-leading-glob  {set ::g 1; set v $::g; set v}                                     1
probe A.bare-ns-chain      {namespace eval ::ns {set ::ns::x 5}; set v $::ns::x; set v}       5
probe A.bare-stops-at-dot  {set foo.bar 1; set v $foo; set v}
probe A.bare-stops-hyphen  {set foo-bar 1; set v $foo; set v}
probe A.bare-utf8-name     {set ñame 7; set v $ñame; set v}                                   7
probe A.bare-no-name-after-dollar   {set rc [catch {eval "set v \$"} err]; list rc=$rc err=$err}
probe A.brace-simple       {set foo 1; set v ${foo}; set v}                                   1
probe A.brace-hyphen       {set "weird-name" 7; set v ${weird-name}; set v}                   7
probe A.brace-space        {set "with space" 8; set v ${with space}; set v}                   8
probe A.brace-dot          {set "a.b" 9; set v ${a.b}; set v}                                 9
probe A.brace-utf8         {set café 10; set v ${café}; set v}                                10
probe A.brace-empty-name   {set "" "EMPTY"; set v ${}; set v}                                 EMPTY
probe A.brace-with-newline {set "a\nb" 99; set v ${a
b}; set v}                                                                                    99
probe A.brace-paren-elem   {set a(x) 1; set v ${a(x)}; set v}                                 1

# §A.brace-escape — Tcl 9.0 brace-form recognises ``\X`` (consume 2)
# and tracks inner ``{...}`` (brace count).  Tested in a separate file
# (tcl_probes_brace_escape.tcl) to avoid escaping conflicts in the
# wrapper probe block.

# §A.brace-then-paren -- the W216 trap
probe A.brace-then-paren-scalar-concat {
    set arr 100
    set v "${arr}(name)"
    set v
} "100(name)"
probe A.brace-then-paren-array-error {
    set arr(x) 7
    set rc [catch {set v "${arr}(x)"} err]
    list rc=$rc err=$err
}

# §A.brace-array-no-subst-in-index
probe A.brace-array-literal-index {
    set foo bar
    set arr(bar) "BARE"
    set "arr(\$foo)" "BRACE"
    list bare=$arr($foo) brace=${arr($foo)}
} {bare=BARE brace=BRACE}

# =============================================================
# §B. Set / variable creation edge cases
# =============================================================

probe B.set-empty-name              {set "" 7; info exists ""}                                1
probe B.set-array-elem-empty-idx    {set a() 1; info exists a()}                              1
probe B.set-via-quoted-name         {set "weird\}name" 1; info exists "weird\}name"}          1
probe B.set-name-with-dollar        {set foo bar; set "lit-\$x" 1; info exists "lit-\$x"}     1
probe B.set-array-and-scalar-collision {
    set foo "scalar"
    set rc [catch {set foo(x) 1} err]
    list rc=$rc err=$err
}
probe B.set-name-from-substitution {
    # ``set [some_command_returning_name] value`` -- name is dynamic.
    set name "dynamic_var"
    set $name 5
    set v $dynamic_var
    set v
} 5
probe B.set-name-array-nested {
    # ``set foo(a)(b) 1`` -- Tcl arrays are flat; this sets element ``a``
    # of array ``foo`` and... what?
    set rc [catch {set foo(a)(b) 1} err]
    list rc=$rc err=$err
}

# =============================================================
# §C. set indirection (the [set "name"] pattern)
# =============================================================

probe C.set-indirect-read {set "weird\}name" 42; set v [set "weird\}name"]; set v}            42
probe C.set-indirect-with-subst {
    set foo bar
    set arr(bar) 88
    set v [set "arr($foo)"]
    set v
} 88
probe C.set-via-list-indirection {
    # If the name itself comes from a variable, we use [set $varname].
    set name foo
    set foo 42
    set v [set $name]
    set v
} 42

# =============================================================
# §D. Globals & namespaces
# =============================================================

probe D.proc-bare-fails-without-decl {
    set ::g 1
    proc f {} {set rc [catch {set v $g} err]; list rc=$rc err=$err}
    f
}
probe D.proc-qualified-bare-works {
    set ::g 1
    proc f {} {return $::g}
    f
} 1
probe D.global-bare-decl {
    set ::g 5
    proc f {} {global g; return $g}
    f
} 5
probe D.global-qualified-decl {
    # ``global ::g`` -- aliases ``g`` (unqualified) to ``::g``.
    set ::g 5
    proc f {} {global ::g; return [info exists g]:[info exists ::g]:[set g]:[set ::g]}
    f
} 1:1:5:5
probe D.global-namespaced-decl {
    # ``global ::ns::x`` -- works in real Tcl (despite docs hinting at global only).
    namespace eval ::ns {set ::ns::x 7}
    proc f {} {
        set rc [catch {global ::ns::x} err]
        list rc=$rc x=[info exists x] qual=[info exists ::ns::x]
    }
    f
}
probe D.variable-ns-eval {
    namespace eval ::nsv {variable v 1}
    set ::nsv::v
} 1
probe D.variable-decl-in-proc {
    namespace eval ::nsp {
        variable v 11
        proc f {} {variable v; return $v}
    }
    ::nsp::f
} 11
probe D.variable-with-substituted-name {
    # ``variable $name 7`` -- name is substituted.  Does it work?
    namespace eval ::nsv2 {
        set name dyn
        variable $name 7
        set v [set $name]
        set v
    }
} 7
probe D.namespace-and-var-same-name {
    set ::same 1
    namespace eval ::same {variable x 1}
    list ::same=$::same ::same::x=$::same::x
} {::same=1 ::same::x=1}

# =============================================================
# §E. upvar / namespace upvar
# =============================================================

probe E.upvar-explicit-1 {
    proc inner {var} {upvar 1 $var local; set local 9}
    set x 1; inner x; set x
} 9
probe E.upvar-default-level {
    proc inner {var} {upvar $var local; set local 9}
    set x 1; inner x; set x
} 9
probe E.upvar-zero-current-scope {
    set x 5
    upvar 0 x y    ;# alias y to x in CURRENT scope
    set y
} 5
probe E.upvar-pound-zero-global {
    proc inner {} {upvar #0 ::g local; set local 999}
    set ::g 0; inner; set ::g
} 999
probe E.upvar-pound-N-absolute {
    # ``upvar #N`` -- absolute frame N from bottom.
    proc level1 {} {set local1 7; level2}
    proc level2 {} {upvar #1 local1 alias; return $alias}
    level1
} 7
probe E.upvar-multiple-pairs {
    proc inner {a b} {upvar 1 $a la $b lb; set la 10; set lb 20}
    set x 0; set y 0; inner x y; list $x $y
} {10 20}
probe E.namespace-upvar {
    namespace eval ::nsu {variable x 1}
    proc f {} {namespace upvar ::nsu x local; set local 99}
    f
    set ::nsu::x
} 99

# =============================================================
# §F. uplevel
# =============================================================

probe F.uplevel-zero-creates-global {
    proc inner {} {uplevel #0 {set ::created 99}}
    inner
    set ::created
} 99
probe F.uplevel-1-sees-caller {
    proc outer {} {set local 5; inner}
    proc inner {} {uplevel 1 {set local}}
    outer
} 5
probe F.uplevel-default-level-1 {
    proc outer {} {set local 5; inner}
    proc inner {} {uplevel {set local}}
    outer
} 5
probe F.uplevel-string-zero {
    proc inner {} {uplevel "0" {set ::sl 1}}
    inner; info exists ::sl
} 1
probe F.uplevel-multiple-bodies {
    # ``uplevel #0 body1 body2`` -- bodies are concatenated.
    proc inner {} {uplevel #0 {set ::a 1} {set ::b 2}}
    inner
    list a=$::a b=$::b
} {a=1 b=2}

# =============================================================
# §G. dict
# =============================================================

probe G.dict-with-substitutes-keys {set d {name alice age 30}; dict with d {set v $name}; set v}    alice
probe G.dict-with-locals-persist {
    # Surprise: dict-with body locals PERSIST after the body ends.
    set d {a 1}
    proc f {} {set d {a 1}; dict with d {set tmp 1}; info exists a}
    f
} 1
probe G.dict-with-modifies-source {
    set d {a 1 b 2}
    dict with d {set a 10}
    set d
} {a 10 b 2}
probe G.dict-with-funny-key-as-local {
    # Dict key "two words" -- becomes local var with that name.
    set d [dict create "two words" XX]
    dict with d {set v [set "two words"]; set v}
} XX
probe G.dict-update-aliases {
    set d {a 1 b 2}
    dict update d a la b lb {set la 10; set lb 20}
    set d
} {a 10 b 20}
probe G.dict-update-creates-missing-key {
    set d {a 1}
    dict update d nokey local {set local 99}
    set d
} {a 1 nokey 99}
probe G.dict-for {
    set d {a 1 b 2}
    set out {}
    dict for {k v} $d {lappend out $k=$v}
    set out
} {a=1 b=2}

# =============================================================
# §H. Array oddities
# =============================================================

probe H.array-elem-name-substituted {
    set foo bar
    set arr($foo) 1   ;# command-word subst -> arr(bar)
    info exists arr(bar)
} 1
probe H.array-elem-name-literal-dollar {
    set "arr(\$foo)" 1
    info exists "arr(\$foo)"
} 1
probe H.array-deep-nested-fails {
    set foo(a) 1
    set rc [catch {set foo(a)(b) 2} err]
    list rc=$rc err=$err
}
probe H.array-trailing-paren-mismatch {
    set rc [catch {eval "set arr(foo 1"} err]
    list rc=$rc err=$err
}
probe H.array-double-paren {
    # ``$arr((x))`` -- inner parens.
    set foo(\(x\)) 1
    set rc [catch {set v $foo((x))} err]
    list rc=$rc err=$err
}
probe H.array-empty-name {
    set rc [catch {set "(x)" 1} err]
    list rc=$rc exists_empty_name=[info exists "(x)"]
}

# =============================================================
# §I. expr & substitution context
# =============================================================

probe I.expr-substitutes-vars {set x 5; expr {$x + 1}}                                        6
probe I.expr-substitutes-array {set k 1; set a(1) 5; expr {$a($k) + 1}}                       6
probe I.expr-quoted-string {set x 5; expr {"$x" eq "5"}}                                      1
probe I.expr-no-subst-in-braces {
    # The expr arg ``{$x}`` -- the lexer's inner brace literal. ``$x``
    # is substituted by expr's parser, NOT the command parser.
    set x 5
    expr {$x}
} 5

# =============================================================
# §J. The W215 / W216 traps
# =============================================================

probe J.W215-name-with-close-brace-creatable {
    set "weird\}name" 1
    info exists "weird\}name"
} 1
probe J.W215-unreachable-via-bare {
    set "weird\}name" 1
    set rc [catch {eval "set v \$weird\\}name"} err]
    list rc=$rc err=$err
}
probe J.W215-array-paren-in-index {
    set "arr(weird\)idx)" 1
    info exists "arr(weird\)idx)"
} 1

probe J.W216-pattern1-array-error {
    set arr(x) 1
    set rc [catch {set v "${arr}(x)"} err]
    list rc=$rc err=$err
}
probe J.W216-pattern1-scalar-concat {
    set arr "ARR"
    set v "${arr}(x)"
    set v
} "ARR(x)"
probe J.W216-pattern2-no-subst {
    set foo bar
    set arr(bar) "BARE"
    set "arr(\$foo)" "BRACE"
    list bare=$arr($foo) brace=${arr($foo)}
} {bare=BARE brace=BRACE}

# =============================================================
# §K. Misc edge cases
# =============================================================

probe K.bare-double-dollar {set x foo; set v $$x; set v}                                      "\$foo"
probe K.bare-just-double-colon {set rc [catch {eval "puts \$::"} err]; list rc=$rc err=$err}
probe K.bare-trailing-colons {set x_ 1; set rc [catch {set v $x::} err]; list rc=$rc err=$err}
probe K.dollar-in-quotes      {set x 5; set v "x is $x"; set v}                               "x is 5"
probe K.dollar-in-braces      {set x 5; set v {x is $x}; set v}                               {x is $x}
probe K.escape-dollar         {set x 5; set v "literal \$x"; set v}                           {literal $x}
probe K.command-subst-quoted  {set v "[expr {2 + 2}] is four"; set v}                         "4 is four"
probe K.bracket-in-array-idx  {
    set k 5
    set arr(5) "value"
    set v $arr([set k])
    set v
} value
probe K.namespace-current {
    namespace eval ::ns3 {set v [namespace current]}
    set ::ns3::v
} ::ns3
probe K.global-namespace-name-empty {
    namespace current
} ::

puts ""
puts "PROBE SUMMARY: $::PASS pass, $::FAIL fail, $::SKIP open-ended"
