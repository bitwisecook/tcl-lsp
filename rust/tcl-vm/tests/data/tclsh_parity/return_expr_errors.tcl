# The errorInfo an error in `return [expr {…}]` leaves names the expr command
# inside the brackets, whether the expression fails to compile or at run time.
proc info_of {script} {
    set code [catch {uplevel 1 $script} m]
    list $code [string map [list \n { | }] $m] $::errorCode \
        [string map [list \n { | }] $::errorInfo]
}
proc is_api {uri} { return [expr {$uri starts_with "/api"}] }
proc r1 {} { return [expr {1 +}] }
proc r2 {x} { return [expr {$x + 1}] }
proc r3 {} {
    set a 1
    return [expr {1 +}]
}
proc r4 {x} {
    if {$x} {
        return [expr {1 +}]
    }
    return ok
}
foreach script {{is_api /api/v1} r1 {r2 a} r3 {r4 1} {return [expr {1 +}]}} {
    puts [info_of $script]
}
