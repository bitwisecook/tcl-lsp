puts "VERSION|[info patchlevel]"
proc observe {label script} {
    set code [catch {uplevel 1 $script} result]
    binary scan $result H* bytes
    puts "CASE|$label|$code|$bytes"
}
observe EXISTING_ROOT {set ::x ROOT; namespace eval N {set x VALUE; list [set x] [catch {set ::N::x} local] $local [set ::x]}}
observe FRESH_NO_ROOT {unset ::x; namespace eval Fresh {set x VALUE; list [set x] [catch {set ::Fresh::x} local] $local [catch {set ::x} root] $root}}
observe EXPLICIT_VARIABLE {catch {unset ::x}; set ::x ROOT; namespace eval Explicit {variable x VALUE; list [set x] [catch {set ::Explicit::x} local] $local [set ::x]}}
