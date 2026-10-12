# Fixed original CLI grouping controls. ASCII source, LF lines, no NUL input.
proc r2286hex {value} {
    set bytes {}
    foreach char [split $value {}] {
        scan $char %c ordinal
        if {$ordinal < 1 || $ordinal > 127} {
            error {non-ASCII or zero native field projection unavailable}
        }
        append bytes [format %02x $ordinal]
    }
    return $bytes
}
proc r2286cmp {left right} {
    incr ::callbacks
    return [string compare $left $right]
}
set cases [list \
    {lsort -stride 2 -index 2 {2 A 1 B}} \
    {lsort -stride 2 -index -1 {2 A 1 B}} \
    {lsort -stride 1 -index 0 {2 A 1 B}} \
    {lsort -ascii -stride 2 -index 1 {2 A 1 B}} \
    {lsort -integer -stride 2 -index 0 {2 A 1}} \
    {lsort -command r2286cmp -stride 2 -index 2 {2 A 1 B}} \
    {lsort -command r2286cmp -stride 2 -index 1 {2 A 1 B}} \
    {lsort -command r2286cmp -stride 2 -index 0 {2 A 1}} \
    {lsort -command r2286cmp -stride 1 -index 0 {2 A 1 B}} \
    {lsort -stride 4294967296 -index 0 {2 A 1 B}} \
    {lsort -command r2286cmp -stride 4294967296 -index 0 {2 A 1 B}}]
set ids [list leading-index-outside-group leading-index-negative invalid-width-minimum valid-width-textual group-size-not-multiple command-leading-index-outside-group command-valid-width command-group-size-not-multiple command-invalid-width-minimum original-wide-stride-boundary command-original-wide-stride-boundary]
if {[llength $argv] != 1} {error {one fixed case index is required}}
set case [lindex $argv 0]
if {$case < 0 || $case >= [llength $cases]} {error {fixed case index out of range}}
set script [lindex $cases $case]
set ::callbacks 0
set code [catch $script result]
set error_code {}
set error_info {}
set code_present 0
set info_present 0
if {$code != 0} {
    if {[info exists ::errorCode]} {set error_code $::errorCode; set code_present 1}
    if {[info exists ::errorInfo]} {set error_info $::errorInfo; set info_present 1}
}
puts "case\t$case\t[lindex $ids $case]"
puts "script-hex\t[r2286hex $script]"
puts "guest-code\t$code"
puts "result-hex\t[r2286hex $result]"
puts "error-code-present\t$code_present"
puts "error-code-hex\t[r2286hex $error_code]"
puts "error-info-present\t$info_present"
puts "error-info-hex\t[r2286hex $error_info]"
puts "callback-count\t$::callbacks"
