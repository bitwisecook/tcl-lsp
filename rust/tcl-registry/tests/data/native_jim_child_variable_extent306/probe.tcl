puts [list version [info patchlevel]]
set ::argv "A\u0000B"
set ::argc "COUNT"
set ::argv0 "\u00e9\u20ac"
namespace eval ::N {
    proc make {} {
        set argv LOCAL
        return [interp]
    }
}
set child [::N::make]
puts [list copied_variable argv [string length $::argv] [$child eval {string length $::argv}] [$child eval {set ::argv}]]
puts [list copied_variable argc [$child eval {set ::argc}]]
puts [list copied_variable argv0 [string length $::argv0] [$child eval {string length $::argv0}] [string equal $::argv0 [$child eval {set ::argv0}]]]
$child delete
unset ::argv
set child [interp]
puts [list absent_parent_variable [$child eval {info exists ::argv}]]
$child delete
