namespace eval ::M {
    namespace ensemble create -command path -subcommands {add remove}
    proc add {path} {return [list ADD [namespace current] $path]}
}
proc ::add {path} {return [list GLOBAL $path]}
proc ::remove {path} {return [list GLOBAL_REMOVE $path]}
namespace eval ::Caller {proc add {path} {return [list CALLER $path]}}
puts [list TARGETS [::M::path add x] [namespace eval ::Caller {::M::path add y}] [::add z]]
set code [catch {namespace eval ::Caller {::M::path remove q}} result]
puts [list MISSING $code $result]
