# 9.x side: the legacy form is gone, so a modern registration fired by
# namespace teardown must still receive the full `unset` word.
set ::log {}
proc rec {label n1 n2 op} { lappend ::log "$label:$op" }
namespace eval ::modern {
    variable y 1
    trace add variable y unset {rec M}
}
namespace delete ::modern
puts "modern-teardown: $::log"
puts "legacy-absent: [catch {trace variable q u {rec L}} m]:$m"
