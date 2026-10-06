foreach source {{namespace eval a {}} {variable ::a::x 5} {set ::a::x} {set x} {unset ::a::x}} {
    set code [catch {uplevel #0 $source} result]
    puts [list $code $result]
}
set code [catch {
    namespace eval a {}
    proc qualified {} {variable ::a::x 5; set x 7; set ::a::x}
    qualified
} result]
puts [list procedure $code $result]
unset -nocomplain ::a::x
