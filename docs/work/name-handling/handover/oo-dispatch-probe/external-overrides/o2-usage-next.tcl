package require overrides
namespace import overrides::withCmdOverrides overrides::nextOverride
proc load {path} { return loaded:$path }
proc main {} {
    set count 0
    withCmdOverrides {load {lambda {{path} {
        incr ::calls
        return "over:[nextOverride $path]" ;# CALL nextOverride
    }}}} {
        set result [load a.tcl] 
        incr count
    }
    return $result:$count
}
set ::calls 0
puts [main]:$::calls
