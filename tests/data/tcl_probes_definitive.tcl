#!/usr/bin/env tclsh
# Definitive validation of Tcl 9.0.3 variable-name handling for special
# characters: } \ ( ) " $ [ ]
# Each test runs in a fresh namespace and reports:
#   - the exact name(s) created (via info vars + hex)
#   - the result of each read attempt
# Use ``info vars`` and ``set`` to confirm hypotheses.

proc hexdump {s} {
    set out ""
    foreach ch [split $s ""] {
        scan $ch %c codepoint
        append out [format {%02x} $codepoint] " "
    }
    string trim $out
}

proc dump_vars {ns} {
    foreach v [lsort [info vars ${ns}::*]] {
        set short [string range $v [string length ${ns}::] end]
        if {[array exists $v]} {
            puts "    name=[list $short]  hex=[hexdump $short]  len=[string length $short]  array=[array names $v]"
            foreach k [lsort [array names $v]] {
                puts "      [list $short]([list $k]) = [list [set ${v}($k)]] (key hex=[hexdump $k])"
            }
        } else {
            puts "    name=[list $short]  hex=[hexdump $short]  len=[string length $short]  value=[list [set $v]]"
        }
    }
}

set ::TEST_NUM 0
proc T {label create_script alt_reads} {
    incr ::TEST_NUM
    set ns "::tn$::TEST_NUM"
    namespace eval $ns {}
    namespace eval $ns $create_script
    puts "================================================================"
    puts "TEST: $label"
    puts "  create: $create_script"
    set vars [info vars ${ns}::*]
    if {[llength $vars] == 0} {
        puts "  (no vars created in this namespace)"
    } else {
        dump_vars $ns
    }
    foreach r $alt_reads {
        # ``r`` is a snippet to evaluate in the namespace context.
        set rc [catch {namespace eval $ns $r} val]
        if {$rc == 0} {
            puts "  read [list $r] -> OK [list $val]"
        } else {
            puts "  read [list $r] -> ERR [list [lindex [split $val \n] 0]]"
        }
    }
    puts ""
}

# ============================================================
# §1. } in variable names
# ============================================================

T "1a quoted close-brace" \
   {set "weird\}name" 1} \
   [list \
       {set v [set "weird\}name"]; set v} \
       {set v [info exists "weird\}name"]; set v} \
       {set v $weird; set v}]

T "1b bare backslash close-brace" \
   {set weird\}name 1} \
   [list \
       {set v [set "weird\}name"]; set v} \
       {set v [info exists "weird\}name"]; set v}]

T "1d via list indirection" \
   {set name "weird\}name"; set $name 1} \
   [list \
       {set v [set "weird\}name"]; set v}]

# ============================================================
# §2. \ in variable names
# ============================================================

T "2a quoted backslash double" \
   {set "back\\slash" 1} \
   [list \
       {set v [set "back\\slash"]; set v} \
       {set v [info exists "back\\slash"]; set v}]

T "2b braced backslash literal" \
   {set "back\\slash" 1} \
   [list \
       {set v [info exists "back\\slash"]; set v}]

T "2c bare backslash" \
   {set back\\slash 1} \
   [list \
       {set v [info exists "back\\slash"]; set v}]

# ============================================================
# §3. ( and ) in variable names (NOT array element)
# ============================================================

T "3a quoted open-paren only" \
   {set "open(noidx" 1} \
   [list \
       {set v [info exists "open(noidx"]; set v} \
       {set v [info exists open]; set v}]

T "3b quoted balanced parens" \
   {set "(both)parens" 1} \
   [list \
       {set v [info exists "(both)parens"]; set v}]

T "3c array idx with close paren via escape" \
   {set arr(weird\)idx) 1} \
   [list \
       {set v [info exists "arr(weird\)idx)"]; set v} \
       {set v [array names arr]; set v}]

T "3d array idx empty" \
   {set arr() 1} \
   [list \
       {set v $arr(); set v}]

# ============================================================
# §4. " in variable names
# ============================================================

T "4a quoted name with quote escape" \
   {set "with\"quote" 1} \
   [list \
       {set v [info exists "with\"quote"]; set v}]

# ============================================================
# §5. $ in variable names (literal, not substitution)
# ============================================================

T "5a quoted name with dollar escape" \
   {set "dollar\$inside" 1} \
   [list \
       {set v [info exists "dollar\$inside"]; set v}]

# ============================================================
# §6. [ and ] in variable names
# ============================================================

T "6a quoted name with bracket escape" \
   {set "bracket\[inside\]" 1} \
   [list \
       {set v [info exists "bracket\[inside\]"]; set v}]

# ============================================================
# §7. Brace-form ${name} reachability tests
# ============================================================

T "7a-setup var named 'a}b' (3 chars: a cb b)" \
   {set "a\}b" "REACHABLE-AcbB"} \
   [list \
       {set v [info exists "a\}b"]; set v} \
       {set v [set "a\}b"]; set v}]

T "7b setup var named 'a\\}b' (4 chars)" \
   {set "a\\\}b" "REACHABLE-A-bs-cb-b"} \
   [list \
       {set v [info exists "a\\\}b"]; set v} \
       {set v [set "a\\\}b"]; set v}]

T "7c setup var named 'a{b}c' (5 chars)" \
   {set "a{b}c" "REACHABLE-AbBcC"} \
   [list \
       {set v [info exists "a{b}c"]; set v} \
       {set v [set "a{b}c"]; set v}]

# ============================================================
# §8. Now test ${...} reads against the §7 vars
# ============================================================

# 8a. Set up all three §7 vars in a single namespace and try every read.
namespace eval ::section8 {
    set "a\}b" "VAL-AcbB"
    set "a\\\}b" "VAL-A-bs-cb-b"
    set "a{b}c" "VAL-AbBcC"
}
puts "================================================================"
puts "§8 setup: ::section8 has three vars"
foreach v [info vars ::section8::*] {
    set short [string range $v 12 end]
    puts "  '[list $short]'  hex=[hexdump $short]  value=[set $v]"
}
puts ""

# 8a-1. ${a\}b} -- our hypothesis: Tcl 9.0.3 brace-form treats \X as
# 2-char escape, so name = 'a\}b' (4 chars: a, bs, cb, b).  This should
# NOT reach 'a}b' (3 chars without backslash).
puts "Read \${a\\}b} from inside ::section8:"
set rc [catch {namespace eval ::section8 {set v ${a\}b}; set v}} val]
puts "  rc=$rc result_or_err=[list $val]"
puts ""

# 8a-2. Confirm by also reading via [set "..."].
puts "Read \[set \"a\\\\\\\}b\"] from inside ::section8 (literal name 'a\\}b'):"
set rc [catch {namespace eval ::section8 {set v [set "a\\\}b"]; set v}} val]
puts "  rc=$rc result_or_err=[list $val]"
puts ""

# 8b-1. ${a{b}c} -- our hypothesis: brace-form tracks inner {...} via
# brace counting, so name = 'a{b}c' (5 chars).  Should reach the
# matching var.
puts "Read \${a{b}c} from inside ::section8:"
set rc [catch {namespace eval ::section8 {set v ${a{b}c}; set v}} val]
puts "  rc=$rc result_or_err=[list $val]"
puts ""

# 8c. Demonstrate the W215 unreachable-name claim is correct:
# the var literally named 'a}b' (3 chars) cannot be read by ANY $-form.
puts "Try EVERY \$-form to read var 'a\}b' (3 chars, literal close-brace):"
puts "  [list \${a\}b}]:"
set rc [catch {namespace eval ::section8 {set v ${a\}b}; set v}} val]
puts "    rc=$rc -> [list $val]"
puts "  bare \$ form ('\$a\\\\}b' -- bare stops at non-word):"
set rc [catch {namespace eval ::section8 {set v $a\}b; set v}} val]
puts "    rc=$rc -> [list $val]"
puts "  via \[set \"a\\\\}b\"]:"
set rc [catch {namespace eval ::section8 {set v [set "a\}b"]; set v}} val]
puts "    rc=$rc -> [list $val]"
puts ""

# ============================================================
# §9. Brace-form parser internal behaviour matrix
# ============================================================
# What variable name does Tcl 9.0.3 LOOK UP for each ${...} source?
# Answer: error message tells us the name.

puts "================================================================"
puts "§9 -- definitive brace-form name extraction"
puts ""
proc lookup_name_via_brace {src} {
    # Source must already be substituted by command parser.
    # We use a fresh interp eval to get a clean error.
    set rc [catch [list eval "set v $src"] err]
    if {$rc == 0} {
        return "OK: $err"
    }
    if {[regexp {can't read "([^"]*)"} $err -> name]} {
        return "lookup-name=[list $name] hex=[hexdump $name] len=[string length $name]"
    }
    return "ERR: $err"
}

# Each source is what would appear after the command parser has done
# its own backslash subst.  We pass them as Tcl strings.

puts "src=\${foo} -> [lookup_name_via_brace \${foo}]"
puts "src=\${a\\}b} -> [lookup_name_via_brace {${a\}b}}]"
puts "src=\${a{b}c} -> [lookup_name_via_brace {${a{b}c}}]"
puts "src=\${a\\\\b} -> [lookup_name_via_brace {${a\\b}}]"
puts "src=\${a\\nb} (with backslash + n) -> [lookup_name_via_brace {${a\nb}}]"
puts "src=\${\\}} -- single backslash inside -> [lookup_name_via_brace {${\}}}]"
puts ""

puts "DONE"
