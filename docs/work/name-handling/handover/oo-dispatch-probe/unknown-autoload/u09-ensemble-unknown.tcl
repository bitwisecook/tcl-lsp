# namespace ensemble -unknown: handler returns a command prefix that is then invoked.
namespace eval ens {
  proc a {} { return A }
  proc fallback {ens sub args} { return [list ::ens::catchall $sub] }
  proc catchall {sub args} { return "catchall $sub $args" }
  namespace export a
  namespace ensemble create -unknown ::ens::fallback
}
puts [ens a]
puts [ens zzz 1 2]
proc ::ens::adder {ens sub args} { proc ::ens::$sub {args} "return \"added $sub \$args\""; namespace eval ::ens [list namespace export $sub]; return {} }
namespace ensemble configure ens -unknown ::ens::adder
puts [ens newsub x]
puts [ens newsub y]
