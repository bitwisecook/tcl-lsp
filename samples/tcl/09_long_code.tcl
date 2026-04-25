#!/usr/bin/env tclsh9.0
# Parser Exerciser for Tcl 9.0
# A single-file program designed to touch lots of Tcl syntax and features.
# It should run on a stock Tcl 9.0 build without external packages.

# Utilities: assertions, timing
namespace eval ::util {
    variable stats [dict create passes 0 fails 0 notes 0]
    variable t0 [clock microseconds]

    proc nowMicros {} { clock microseconds }

    proc note {msg} {
        variable stats
        dict incr stats notes
        puts "NOTE: $msg"
    }

    proc pass {msg} {
        variable stats
        dict incr stats passes
        puts "PASS: $msg"
    }

    proc fail {msg} {
        variable stats
        dict incr stats fails
        puts "FAIL: $msg"
        return -code error $msg
    }

    proc assert {exprString {msg "assertion failed"}} {
        if {![uplevel 1 [list expr $exprString]]} {
            fail $msg
        }
        return 1
    }

    proc assertEq {a b {msg ""}} {
        if {$a ne $b} {
            if {$msg eq ""} { set msg "Expected '$a' eq '$b'" }
            fail $msg
        }
        return 1
    }

    proc elapsedMs {} {
        variable t0
        set dt [expr {[clock microseconds] - $t0}]
        return [expr {$dt / 1000.0}]
    }
}

# Exercise quoting, substitution, lists
namespace eval ::syntax {
    proc quotingDemo {} {
        set name "World"
        set literal {Hello $name [clock seconds]}
        set interpolated "Hello $name [clock seconds]"
        # We do not assert on timestamp, only structure.
        ::util::assert {[string match {Hello $name *} $literal]} "braced string should be literal"
        ::util::assert {[string match "Hello World *" $interpolated]} "quoted string should substitute"
        return [list $literal $interpolated]
    }

    proc listDemo {} {
        set L [list a "b c" {d e} \{ \} \[ \] \$ $a]
        lappend L "line1\nline2" "\tTabbed"
        ::util::assert {[llength $L] >= 8} "list length"
        # {*} expansion
        proc _join {args} { join $args "|" }
        set j [{*}[list _join] {*}$L]
        rename _join {}
        return $j
    }

    proc substDemo {} {
        set template {X=@X@ Y=@Y@ Z=@Z@}
        set out [string map [list @X@ 10 @Y@ 20 @Z@ "a b"] $template]
        ::util::assertEq $out {X=10 Y=20 Z=a b}
        return $out
    }
}

# Dicts, arrays, globbing, file I/O
namespace eval ::data {
    proc dictDemo {} {
        set d [dict create \
            alpha 1 \
            beta  [dict create nested 1 items [list a b c]] \
            gamma [list 1 2 3] \
            meta  [dict create created [clock seconds] tag "tcl9"]]

        dict set d beta nested 2
        dict with d {
            # dict with binds keys as variables
            set alpha [expr {$alpha + 41}]
            lappend gamma 4
        }
        ::util::assert {[dict get $d alpha] == 42} "dict with mutation"
        ::util::assertEq [lindex [dict get $d gamma] end] 4
        return $d
    }

    proc arrayDemo {} {
        array set A {a 1 b 2 c 3}
        set keys [lsort [array names A]]
        ::util::assertEq $keys {a b c}
        set sum 0
        foreach k $keys { incr sum $A($k) }
        ::util::assert {$sum == 6} "array sum"
        return [array get A]
    }

    proc fileDemo {} {
        set dir [pwd]
        set fname [file join $dir "tcl9_parser_exerciser_tmp_[pid].txt"]
        set ch [open $fname w]
        fconfigure $ch -encoding utf-8 -translation lf
        puts $ch "alpha\nbeta\ngamma\n"
        close $ch

        set ch [open $fname r]
        set content [read $ch]
        close $ch

        ::util::assert {[string match "*beta*" $content]} "file read content"
        file delete -force $fname
        return [string length $content]
    }
}

# Control flow: if/elseif/else, switch, loops
namespace eval ::flow {
    proc classify {x} {
        if {$x < 0} {
            return negative
        } elseif {$x == 0} {
            return zero
        } else {
            return positive
        }
    }

    proc switchDemo {word} {
        switch -glob -- $word {
            a* { return "starts-with-a" }
            b* { return "starts-with-b" }
            *  { return "other" }
        }
    }

    proc loopsDemo {} {
        set out {}
        for {set i 0} {$i < 5} {incr i} {
            lappend out $i
        }
        set i 0
        while {$i < 3} {
            lappend out "w$i"
            incr i
        }
        foreach {k v} [list x 10 y 20] {
            lappend out "$k=$v"
        }
        foreach x [list A B C] y [list 1 2 3] {
            lappend out "$x$y"
        }
        # lmap and dict for
        set squares [lmap n [list 1 2 3 4] { expr {$n*$n} }]
        dict set M squares $squares
        dict for {k v} $M {
            lappend out "$k:[join $v ,]"
        }
        return $out
    }
}

# Regex, scan/format, binary operations
namespace eval ::text {
    proc regexDemo {s} {
        set re {(?i)\b([a-z]+)\b}
        set words {}
        set start 0
        while {[regexp -indices -start $start -- $re $s m g1]} {
            lassign $g1 a b
            lappend words [string range $s $a $b]
            set start [expr {$b + 1}]
        }
        set normalized [string tolower [join $words "-"]]
        set out [regsub -all {[^a-z\-]} $normalized {}]
        return $out
    }

    proc scanFormatDemo {} {
        set n 255
        set hex [format %02X $n]
        scan $hex %x roundtrip
        ::util::assert {$roundtrip == 255} "scan/format roundtrip"

        set b [binary format c* [list 1 2 3 4 5]]
        binary scan $b c* vals
        ::util::assertEq $vals {1 2 3 4 5}
        return [list $hex $vals]
    }

    proc encodingDemo {} {
        # Unicode literal, backslash escapes, and conversion.
        set s "Snowman: \u2603  Music: \u266B"
        set u8 [encoding convertto utf-8 $s]
        set back [encoding convertfrom utf-8 $u8]
        ::util::assertEq $s $back
        return [string length $u8]
    }
}

# Procedures, args, defaults, tailcall, apply (lambda)
namespace eval ::func {
    proc sum {args} {
        set total 0
        foreach x $args { incr total $x }
        return $total
    }

    proc greet {{name "Anonymous"} {title ""}} {
        if {$title ne ""} {
            return "Hello, $title $name"
        }
        return "Hello, $name"
    }

    proc tailSum {lst {acc 0}} {
        if {[llength $lst] == 0} {
            return $acc
        }
        set head [lindex $lst 0]
        set rest [lrange $lst 1 end]
        tailcall tailSum $rest [expr {$acc + $head}]
    }

    proc applyDemo {} {
        set doubler [list x { expr {$x * 2} }]
        set out {}
        foreach n {1 2 3 4} {
            lappend out [apply $doubler $n]
        }
        return $out
    }

    proc upvarDemo {} {
        set x 10
        proc bump {varName} {
            upvar 1 $varName v
            incr v
            return $v
        }
        set y [bump x]
        rename bump {}
        ::util::assert {$x == 11 && $y == 11} "upvar works"
        return $x
    }
}

# Namespaces, ensembles, and variable traces
namespace eval ::api {
    variable calls 0

    proc _trace {name1 name2 op} {
        variable calls
        incr calls
    }

    proc setupTrace {} {
        variable calls
        set calls 0
        trace add variable ::api::calls write ::api::_trace
        incr calls
        trace remove variable ::api::calls write ::api::_trace
        return $calls
    }

    namespace export ping add
    proc ping {} { return "pong" }
    proc add {a b} { expr {$a + $b} }

    namespace ensemble create -command math -map [dict create ping ping add add]
}

# TclOO: class, method, mixin, filters
namespace eval ::ooDemo {
    oo::class create Counter {
        variable value log
        constructor {{start 0}} {
            set value $start
            set log {}
        }
        method inc {{n 1}} {
            incr value $n
            lappend log [list inc $n $value]
            return $value
        }
        method dec {{n 1}} {
            incr value [expr {-$n}]
            lappend log [list dec $n $value]
            return $value
        }
        method get {} { return $value }
        method history {} { return $log }
        destructor {
            # touch destructor syntax, but keep it quiet
            set log {}
        }
    }

    oo::class create LoggerMixin {
        method logCall {args} {
            # Filter-like wrapper, record and pass through.
            set line [format "CALL %s" [join $args ,]]
            my variable _log
            lappend _log $line
            return [next {*}$args]
        }
        method logs {} { my variable _log; return $_log }
    }

    proc run {} {
        set c [Counter new 5]
        # Mixin demonstration
        oo::objdefine $c mixin LoggerMixin
        oo::objdefine $c filter logCall

        $c inc 2
        $c dec
        set v [$c get]
        ::util::assert {$v == 6} "oo counter result"
        set hist [$c history]
        set logs [$c logs]
        $c destroy
        return [list $v [llength $hist] [llength $logs]]
    }
}

# Coroutines: yield/resume
namespace eval ::co {
    proc generator {n} {
        for {set i 0} {$i < $n} {incr i} {
            yield $i
        }
        return -code break
    }

    proc run {} {
        set name "gen_[pid]"
        set first [coroutine $name ::co::generator 5]
        set out {}
        if {$first ne ""} {
            lappend out $first
        }
        while {1} {
            try {
                lappend out [$name]
            } trap {TCL BREAK} {} {
                break
            }
        }
        if {[llength [info commands $name]]} {
            rename $name {}
        }
        ::util::assertEq $out {0 1 2 3 4}
        return $out
    }
}

# Safe interpreters: create, alias, evaluate
namespace eval ::sandbox {
    proc run {} {
        set i [interp create -safe]
        # Expose only a safe-ish command via alias: a tiny math add
        interp alias $i add {} ::api::add
        set script {
            set a 7
            set b 35
            add $a $b
        }
        set r [interp eval $i $script]
        interp delete $i
        ::util::assert {$r == 42} "safe interp computed 42"
        return $r
    }
}

# try/catch/finally, errors, custom return options
namespace eval ::errors {
    proc mightFail {mode} {
        switch -- $mode {
            ok    { return "ok" }
            error { error "boom" "DETAILS" [list MYCODE 123] }
            return {
                return -code return -level 0 "forced return"
            }
            default { error "unknown mode: $mode" }
        }
    }

    proc run {} {
        set out {}
        try {
            lappend out [mightFail ok]
            mightFail error
            lappend out "unreached"
        } on error {msg opts} {
            # Extract structured info
            set ec [dict get $opts -errorcode]
            lappend out "caught:$msg"
            lappend out "code:[join $ec /]"
        } finally {
            lappend out "finally"
        }
        return $out
    }
}

# Event loop: after, vwait, fileevent (minimal)
namespace eval ::eventing {
    variable done 0
    proc run {} {
        variable done
        set done 0
        after 10 [list set ::eventing::done 1]
        vwait ::eventing::done
        ::util::assert {$done == 1} "vwait/after"
        return $done
    }
}

# Introspection: info, unknown, namespace introspection
namespace eval ::introspect {
    proc run {} {
        set procs {}
        set vars {}
        foreach ns [namespace children ::] {
            foreach p [info procs ${ns}::*] {
                lappend procs $p
            }
            foreach v [info vars ${ns}::*] {
                lappend vars $v
            }
        }
        set procs [lsort -unique $procs]
        set vars [lsort -unique $vars]
        # This is intentionally loose, to avoid depending on specific internal procs.
        ::util::assert {[llength $procs] > 10} "has many procs"
        ::util::assert {[llength $vars]  > 5}  "has some vars"
        return [list [llength $procs] [llength $vars]]
    }
}

# Main test runner
proc main {} {
    set results [dict create]

    # Syntax and substitution
    dict set results quoting   [::syntax::quotingDemo]
    dict set results listDemo  [::syntax::listDemo]
    dict set results substDemo [::syntax::substDemo]

    # Data structures and I/O
    dict set results dictDemo  [::data::dictDemo]
    dict set results arrayDemo [::data::arrayDemo]
    dict set results fileBytes [::data::fileDemo]

    # Control flow
    dict set results classify  [list \
        [::flow::classify -1] [::flow::classify 0] [::flow::classify 1]]
    dict set results switch    [list \
        [::flow::switchDemo apple] [::flow::switchDemo banana] [::flow::switchDemo kiwi]]
    dict set results loops     [::flow::loopsDemo]

    # Text and binary
    dict set results regex     [::text::regexDemo "Hello, HELLO! Tcl 9.0 parser; test-case #42."]
    dict set results scanFmt   [::text::scanFormatDemo]
    dict set results encBytes  [::text::encodingDemo]

    # Funcs and higher-order
    dict set results sum       [::func::sum 1 2 3 4 5]
    dict set results greet     [list [::func::greet] [::func::greet "Ada" "Dr."]]
    dict set results tailSum   [::func::tailSum [list 1 2 3 4 5]]
    dict set results apply     [::func::applyDemo]
    dict set results upvar     [::func::upvarDemo]

    # Namespaces, ensembles, traces
    dict set results ensemble1 [::api::math ping]
    dict set results ensemble2 [::api::math add 20 22]
    dict set results trace     [::api::setupTrace]

    # OO, coroutine, safe interp, errors, event loop
    dict set results oo        [::ooDemo::run]
    dict set results coroutine [::co::run]
    dict set results sandbox   [::sandbox::run]
    dict set results errors    [::errors::run]
    dict set results eventing  [::eventing::run]
    dict set results info      [::introspect::run]

    # A few sanity asserts over collected results
    ::util::assert {[dict get $results sum] == 15} "sum correct"
    ::util::assertEq [lindex [dict get $results greet] 0] "Hello, Anonymous"
    ::util::assert {[dict get $results ensemble2] == 42} "ensemble add"
    ::util::assert {[lindex [dict get $results classify] 0] eq "negative"} "classify negative"

    # Pretty print a summary that also exercises dict for + format
    puts ""
    puts "==== Tcl 9.0 Parser Exerciser Summary ===="
    dict for {k v} $results {
        set pv $v
        if {[string length $pv] > 120} {
            set pv "[string range $pv 0 117]..."
        }
        puts [format "%-12s : %s" $k $pv]
    }

    puts ""
    puts [format "Elapsed: %.2f ms" [::util::elapsedMs]]
    puts [format "Passes=%d Fails=%d Notes=%d" \
        [dict get $::util::stats passes] \
        [dict get $::util::stats fails] \
        [dict get $::util::stats notes]]
    puts "OK"
    return 0
}

# Run main with robust error reporting
try {
    exit [main]
} on error {msg opts} {
    puts stderr "FATAL: $msg"
    if {[dict exists $opts -errorinfo]} {
        puts stderr [dict get $opts -errorinfo]
    }
    exit 1
}