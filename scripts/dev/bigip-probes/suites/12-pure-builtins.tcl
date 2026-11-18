# Check 33 of docs/work/name-handling/bigip-name-checks.md (issue #1712).
#
# `tclsh8.4 suites/12-pure-builtins.tcl` prints, first, the reference lines
# (`ref <id> <result>`), then a line `---`, then an iRule that logs the same
# calls as `got <id> <result>` with `log local0.`. Load the rule, send one
# HTTP request, and compare the `got` lines against the `ref` lines: a builtin
# is evidenced pure for f5-irules when every line matches and its
# `vars=`/`procs=` lines are unchanged by the calls.

set probes {
    concat-1 {concat a {b c} {}}
    concat-2 {concat " a " "\{b\}"}
    format-1 {format %s-%d-%05.2f x 7 3.14159}
    format-2 {format %c 233}
    format-3 {format %q 1}
    join-1 {join {a b {c d}} -}
    join-2 {join {}}
    list-1 {list a {b c} "d e" \{}
    llength-1 {llength {a {b c} d}}
    llength-2 {llength "a \{"}
    lindex-1 {lindex {a b c} end-1}
    lindex-2 {lindex {a b c} 5}
    lindex-3 {lindex {a {b c}} 1 0}
    linsert-1 {linsert {a b} 1 x y}
    linsert-2 {linsert {a b} end+1 x}
    lrange-1 {lrange {a b c d} 1 end}
    lrange-2 {lrange {a b} 3 1}
    lreplace-1 {lreplace {a b c} 1 1 x y}
    lreplace-2 {lreplace {a b c} 5 6}
    split-1 {split a.b..c .}
    split-2 {split "\xe9x" {}}
    info-complete-1 {info complete "set x \{"}
    info-sharedlibextension-1 {info sharedlibextension}
    namespace-qualifiers-1 {namespace qualifiers ::a::b::c}
    namespace-tail-1 {namespace tail ::a::b::c}
    file-extension-1 {file extension /a/b.c.d}
    file-rootname-1 {file rootname /a/b.c.d}
    string-bytelength-1 {string bytelength "\xe9"}
    string-compare-1 {string compare -nocase -length 2 ABc abd}
    string-equal-1 {string equal -nocase A a}
    string-first-1 {string first b abcb 2}
    string-index-1 {string index abc end-1}
    string-last-1 {string last b abcb 2}
    string-length-1 {string length "\xe9x"}
    string-map-1 {string map -nocase {A 1 b 2} aAbB}
    string-match-1 {string match -nocase {[a-c]*} Bx}
    string-range-1 {string range abcdef 2 end-1}
    string-repeat-1 {string repeat ab 3}
    string-replace-1 {string replace abcdef 1 2 XY}
    string-tolower-1 {string tolower "\xc9AB"}
    string-totitle-1 {string totitle "hELLO world"}
    string-toupper-1 {string toupper "\xe9ab" 1 1}
    string-trim-1 {string trim "  xx  "}
    string-trimleft-1 {string trimleft "xxabxx" x}
    string-trimright-1 {string trimright "xxabxx" x}
    string-wordend-1 {string wordend "ab cd" 0}
    string-wordstart-1 {string wordstart "ab cd" 4}
    string-bad-option {string length -bogus x}
}

# The state a call could change, read where the call runs; the helper
# variables exist before the first snapshot so they never count as a change.
set before {}; set after {}; set result {}; set id {}; set script {}
foreach {id script} $probes {
    set before [list [lsort [info vars]] [lsort [info procs]]]
    if {[catch $script result]} {set result "error: $result"}
    set after [list [lsort [info vars]] [lsort [info procs]]]
    if {$before ne $after} {append result " CHANGED"}
    puts "ref $id $result"
}
puts ---
puts "when HTTP_REQUEST {"
puts "    set before {}; set after {}; set result {}"
foreach {id script} $probes {
    puts "    set before \[list \[lsort \[info vars\]\] \[lsort \[info procs\]\]\]"
    puts "    if {\[catch {$script} result\]} {set result \"error: \$result\"}"
    puts "    set after \[list \[lsort \[info vars\]\] \[lsort \[info procs\]\]\]"
    puts "    if {\$before ne \$after} {append result \" CHANGED\"}"
    puts "    log local0. \"got $id \$result\""
}
puts "}"
