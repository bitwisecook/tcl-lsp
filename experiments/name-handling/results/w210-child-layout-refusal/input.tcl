package require tcltest
namespace import -force ::tcltest::*
proc exercise {} {
test t shared -setup {set prepared 1} -body {puts $prepared; puts $absent} -cleanup {puts $prepared} -result {[puts $data_only]}
}
exercise
