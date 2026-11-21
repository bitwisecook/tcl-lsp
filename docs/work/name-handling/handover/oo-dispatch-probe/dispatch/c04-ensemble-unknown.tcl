namespace eval ::u {
    proc fallback {ens sub args} { return [list ::u::any] }
    proc any args { return any:$args }
    namespace ensemble create -subcommands {} -unknown ::u::fallback
}
puts [u whatever 1]
