# namespace unknown handler that returns a command prefix is not how it works: the handler's result is the result.
# A handler that defines the command and re-invokes it.
namespace eval lazy {
  namespace unknown ::lazy::define
  proc define {cmd args} {
    proc ::lazy::$cmd {args} "return \"defined $cmd: \$args\""
    return [uplevel 1 [list ::lazy::$cmd {*}$args]]
  }
  proc run {} { return [list [later 1] [later 2] [info procs ::lazy::later]] }
}
puts [lazy::run]
