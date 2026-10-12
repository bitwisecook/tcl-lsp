namespace eval ::M {
    namespace ensemble create -command path -subcommands {add remove list}
}
proc ::M::add {path args} {return [list ADD [namespace current] $path]}
puts [list OBS [namespace eval ::M {path add /probe}]]
