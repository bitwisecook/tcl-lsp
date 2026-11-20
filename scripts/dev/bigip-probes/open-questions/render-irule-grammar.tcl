# Render the BIG-IP rule checker's per-command argument grammar as text.
#
# Usage: tclsh render-irule-grammar.tcl <jet_tcl.tcl> <outdir>
#
# <jet_tcl.tcl> is the JET checker table extracted by extract-tclcheck.py. It
# is data only (namespace eval + array set); this script sources it into a
# fresh interpreter and walks bigip_rule_syntax and jet_macros. It writes
#   commands.txt  NAME <tab> signature <tab> constraints (events, profiles,
#                 licence, proxy) <tab> semantic checker <tab> object
#                 dependencies <tab> placeholder value checks
#   macros.txt    PLACEHOLDER <tab> definition
#   grammar.json  the same as JSON
# The output is derived from F5 proprietary source: keep it with the
# extraction (outside this repository).
#
# Notation: literal words as-is; PLACEHOLDER for a macro; [x] optional;
# [x ...] zero or more; (a | b) alternatives; {a | b}* an option table whose
# entries may appear in any order; LIST(x y) a sequence given as separate
# words or as one list word; !(X) a word that is not X. Character-level
# macros use regex-like notation.

set src [lindex $argv 0]
set out [lindex $argv 1]
file mkdir $out

interp create data
data eval [list source $src]
array set syntax [data eval {array get JET_BIGIP::bigip_rule_syntax}]
array set macros [data eval {array get JET_BIGIP::jet_macros}]
interp delete data

set used {}

proc list2json {s} {
    set map [list \\ \\\\ \" \\\"]
    for {set c 0} {$c < 32} {incr c} { lappend map [format %c $c] [format \\u%04x $c] }
    return "\"[string map $map $s]\""
}

proc words {l} {
    set r {}
    foreach x $l { if {$x ne ""} { lappend r $x } }
    join $r " "
}

proc alt {parts} {
    set parts [lsearch -all -inline -not -exact $parts ""]
    if {[llength $parts] == 1} { return [lindex $parts 0] }
    return "([join $parts { | }])"
}

proc r {node} {
    global used
    set node [string trim $node]
    if {$node eq ""} { return "" }
    set head [lindex $node 0]
    switch -- $head {
        cmd_literal_word { return [lindex $node 1] }
        cmd_abbreviate_word { return [lindex $node 1] }
        cmd_noop_word { return "" }
        cmd_sequence_word {
            set p {}
            foreach item [lindex $node 1] { lappend p [r $item] }
            return [words $p]
        }
        cmd_alternative_word {
            set p {}
            foreach item [lindex $node 1] { lappend p [r $item] }
            return [alt $p]
        }
        cmd_sequence_list_word {
            # the sequence may be written as separate words or as one list word
            set p {}
            foreach item [lindex $node 1] { lappend p [r $item] }
            return "LIST([words $p])"
        }
        cmd_table_word {
            set p {}
            foreach item [lindex $node 1] { lappend p [r $item] }
            return "{[join [lsearch -all -inline -not -exact $p ""] { | }]}*"
        }
        cmd_optional_word {
            set x [r [lindex $node 1]]
            return [expr {$x eq "" ? "" : "\[$x\]"}]
        }
        cmd_zero_more_word {
            set x [r [lindex $node 1]]
            return [expr {$x eq "" ? "" : "\[$x ...\]"}]
        }
        run_jet_macro {
            dict incr used [lindex $node 1]
            set d [lindex $node 2]
            if {$d eq ""} { set d [regsub {^cmd_macro_} [lindex $node 1] ""] }
            return $d
        }
        cmd_not_arg { return "!([rc [lindex $node 1]])" }
        default {
            if {[string match cmd_*_char $head]} { return [rc $node] }
            return "<$head>"
        }
    }
}

# character level
proc rc {node} {
    set node [string trim $node]
    if {$node eq ""} { return "" }
    set head [lindex $node 0]
    switch -- $head {
        cmd_literal_char { return [lindex $node 1] }
        cmd_abbreviate_char { return [lindex $node 1] }
        cmd_rng_char { lassign [lindex $node 1] a b; return "\[$a-$b\]" }
        cmd_sequence_char {
            set p {}
            foreach item [lindex $node 1] { append p [rc $item] }
            return $p
        }
        cmd_alternative_char {
            set p {}
            foreach item [lindex $node 1] { lappend p [rc $item] }
            return "([join $p |])"
        }
        cmd_optional_char { return "([rc [lindex $node 1]])?" }
        cmd_zero_more_char { return "([rc [lindex $node 1]])*" }
        cmd_one_more_char { return "([rc [lindex $node 1]])+" }
        cmd_not_char { return "\[^[rc [lindex $node 1]]\]" }
        default {
            if {[string match cmd_*_char $head]} { return "<$head>" }
            return [r $node]
        }
    }
}

# constraint scripts carried by grammar nodes in a subtree
proc node_constraints {node} {
    set node [string trim $node]
    set head [lindex $node 0]
    set out {}
    switch -- $head {
        cmd_noop_word { set out [lindex $node 1] }
        cmd_literal_word - cmd_abbreviate_word { set out [lindex $node end] }
        cmd_sequence_word - cmd_sequence_list_word - cmd_table_word {
            foreach item [lindex $node 1] { set out [concat $out [node_constraints $item]] }
            set out [concat $out [lindex $node 2]]
        }
        cmd_alternative_word {
            foreach item [lindex $node 1] { set out [concat $out [node_constraints $item]] }
            set out [concat $out [lindex $node 2]]
        }
        cmd_optional_word - cmd_zero_more_word {
            set out [concat [node_constraints [lindex $node 1]] [lindex $node 3]]
        }
    }
    set r {}
    foreach c $out { set c [string trim [regsub -all {\s+} $c " "]]; if {$c ne "" && $c ni $r} { lappend r $c } }
    return $r
}
# top-level forms: each alternative of the command grammar with its constraints
proc forms {g} {
    set g [string trim $g]
    if {[lindex $g 0] eq "cmd_alternative_word"} {
        set f {}
        foreach alt [lindex $g 1] { lappend f [list [r $alt] [node_constraints $alt]] }
        return $f
    }
    return [list [list [r $g] [node_constraints $g]]]
}

# word-count range a grammar node accepts: {min max}, max -1 = unbounded
proc span {node} {
    set node [string trim $node]
    if {$node eq ""} { return {0 0} }
    set head [lindex $node 0]
    switch -- $head {
        cmd_noop_word { return {0 0} }
        cmd_literal_word - cmd_abbreviate_word - run_jet_macro - cmd_not_arg { return {1 1} }
        cmd_sequence_word {
            set lo 0; set hi 0
            foreach item [lindex $node 1] {
                lassign [span $item] a b
                incr lo $a
                if {$hi >= 0} { set hi [expr {$b < 0 ? -1 : $hi + $b}] }
            }
            return [list $lo $hi]
        }
        cmd_sequence_list_word {
            # one list word, or the elements as separate words
            set lo 0; set hi 0
            foreach item [lindex $node 1] {
                lassign [span $item] a b
                incr lo $a
                if {$hi >= 0} { set hi [expr {$b < 0 ? -1 : $hi + $b}] }
            }
            return [list [expr {min(1, $lo)}] $hi]
        }
        cmd_alternative_word {
            set lo -1; set hi 0
            foreach item [lindex $node 1] {
                lassign [span $item] a b
                if {$lo < 0 || $a < $lo} { set lo $a }
                if {$hi >= 0} { set hi [expr {$b < 0 ? -1 : max($hi, $b)}] }
            }
            return [list [expr {max($lo, 0)}] $hi]
        }
        cmd_optional_word { lassign [span [lindex $node 1]] a b; return [list 0 $b] }
        cmd_zero_more_word - cmd_table_word { return {0 -1} }
        default { return {0 -1} }
    }
}
proc forms_json {fl} {
    set parts {}
    foreach f $fl {
        set cs {}
        foreach c [lindex $f 1] { lappend cs [list2json $c] }
        lappend parts [format {{"signature": %s, "constraints": [%s]}} [list2json [lindex $f 0]] [join $cs ", "]]
    }
    return [format {[%s]} [join $parts ", "]]
}

proc events {script} {
    set head [lindex $script 0]
    set args [lrange $script 1 end]
    return "$head [string map {"\n" " "} [regsub -all {\s+} [join $args " "] " "]]"
}

set cf [open $out/commands.txt w]
set json "\{\"commands\": \{"
set first 1
foreach name [lsort [array names syntax]] {
    set sig ""; set ev {}; set sem ""; set fl {}; set ar {}
    set ::used {}
    foreach part $syntax($name) {
        set part [string trim $part]
        set head [lindex $part 0]
        if {$head eq "jetCheck::run_cmd_checker"} {
            set g [string trim [lindex $part 1]]
            set sem [string trim [lindex $part 2]]
            if {[lindex $g 0] eq "cmd_noop_word"} {
                set sig "(no arguments)"
            } elseif {$g eq ""} {
                set sig [expr {$sem ne "" ? "(checked by $sem)" : "(unchecked)"}]
            } else {
                set sig [r $g]
            }
            set fl [forms $g]
            set ar [expr {$g eq "" ? "" : [span $g]}]
        } elseif {$head ne ""} {
            lappend ev [events $part]
        }
    }
    if {$sig eq "" && ![string match *::* $name]} { set sig "(namespace entry, not a command)" }
    set evs [join $ev " ; "]
    # load-time effects carried by the placeholders this command uses
    set deps {}; set checks {}
    foreach m [dict keys $::used] {
        if {![info exists macros($m)]} continue
        set md $macros($m)
        foreach c [dict get $md constraint] {
            set c [string trim [regsub -all {\s+} $c " "]]
            if {$c ne "" && $c ni $deps} { lappend deps $c }
        }
        set chk [string trim [regsub -all {\s+} [dict get $md syntaxer] " "]]
        if {$chk ne "" && $chk ni $checks} { lappend checks "[regsub {^cmd_macro_} $m {}]=$chk" }
    }
    set depstr [join $deps " ; "]
    set chkstr [join $checks " ; "]
    puts $cf "$name\t$sig\t$evs\t$sem\t$depstr\t$chkstr"
    if {!$first} { append json "," }
    set first 0
    append json "\n  [list2json $name]: \{\"signature\": [list2json $sig], \"constraints\": [list2json $evs], \"semantic\": [list2json $sem], \"object_dependencies\": [list2json $depstr], \"value_checks\": [list2json $chkstr], \"forms\": [forms_json $fl], \"arity\": [expr {$ar eq "" ? "null" : "\[[join $ar {, }]\]"}]\}"
}
close $cf

append json "\n\}, \"macros\": \{"

# macros: PLACEHOLDER <tab> character grammar <tab> value checker <tab> constraint
set mf [open $out/macros.txt w]
set first 1
foreach name [lsort [array names macros]] {
    set m $macros($name)
    set short [regsub {^cmd_macro_} $name ""]
    set chars [string trim [dict get $m jet_char_cmd]]
    set def [expr {$chars eq "" ? "" : [rc $chars]}]
    set chk [string trim [regsub -all {\s+} [dict get $m syntaxer] " "]]
    set con [string trim [regsub -all {\s+} [join [dict get $m constraint] " "] " "]]
    puts $mf "$short\t$def\t$chk\t$con"
    if {!$first} { append json "," }
    set first 0
    append json "\n  [list2json $short]: \{\"chars\": [list2json $def], \"checker\": [list2json $chk], \"constraint\": [list2json $con]\}"
}
close $mf
append json "\n\}\}\n"
set jf [open $out/grammar.json w]
puts -nonewline $jf $json
close $jf
puts "[array size syntax] commands, [array size macros] macros"
