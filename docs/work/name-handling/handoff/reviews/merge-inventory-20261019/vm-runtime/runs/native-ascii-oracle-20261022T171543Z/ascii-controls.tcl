# Oracle-only execution authorized by coordinator during Rust source hold.
# Exact supported non-strict ASCII form and explicitly unmodeled controls.
proc ::ascii_plain {subject} {string is ascii $subject}
proc ::ascii_strict {subject} {string is ascii -strict $subject}
proc ::ascii_dynamic {class subject} {string is $class $subject}
proc ::ascii_failindex {subject} {
    set fi untouched
    set answer [string is ascii -failindex fi $subject]
    return [list $answer $fi]
}
proc ::expect {label actual expected} {
    if {![string equal $actual $expected]} {
        error [list mismatch $label actual $actual expected $expected]
    }
    puts [list PASS $label $actual]
}
puts [list version [info patchlevel]]
# label value plain strict failindex-result
foreach {label value plain strict failindex} [list \
    empty {} 1 0 {1 untouched} \
    ascii abcde 1 1 {1 untouched} \
    nonascii \u00e9 0 0 {0 0} \
    mixed a\u00e9 0 0 {0 1} \
    embeddednul a\u0000b 1 1 {1 untouched}] {
    binary scan [encoding convertto utf-8 $value] H* operandHex
    puts [list operand $label utf8hex $operandHex]
    ::expect [list plain $label] [::ascii_plain $value] $plain
    ::expect [list strict-unmodeled $label] [::ascii_strict $value] $strict
    ::expect [list dynamic-class-unmodeled $label] [::ascii_dynamic ascii $value] $plain
    ::expect [list failindex-unmodeled $label] [::ascii_failindex $value] $failindex
}
if {[package vcompare [info patchlevel] 8.6] >= 0} {
    if {[llength [info commands ::tcl::unsupported::disassemble]] != 1} {
        error {pinned modern interpreter lacks expected disassembler}
    }
    foreach name {::ascii_plain ::ascii_strict ::ascii_dynamic ::ascii_failindex} {
        puts [list disassembly $name]
        puts [::tcl::unsupported::disassemble proc $name]
    }
} else {
    puts [list disassembly semantic-only-old-release]
}
puts [list completed semantic-controls 20]
