# tailcall from a proc, from a lambda, from a method, and name resolution in the caller's namespace.
namespace eval ns {
    proc target {x} { return "ns-target $x [info level]" }
    proc p {} { tailcall target p }
}
proc target {x} { return "global-target $x [info level]" }
puts [ns::p]
puts [apply {{} { tailcall target lambda }}]
puts [apply {{} { tailcall target lambda-ns } ns}]
if {[catch {package require TclOO}] == 0} {
    oo::class create C { method m {} { tailcall target method } }
    puts [[C new] m]
}
puts [list [catch {tailcall target top} m] $m]
