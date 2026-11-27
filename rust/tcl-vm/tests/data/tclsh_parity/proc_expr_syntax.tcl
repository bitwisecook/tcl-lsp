# A braced expression C cannot parse compiles, in a proc body, to a `syntax`
# instruction: defining and compiling the proc succeed, and the error is
# raised, catchably, only when the bad expression runs.
proc is_api {uri} { return [expr {$uri starts_with "/api"}] }
puts [catch {is_api /api/v1} m]:[string map [list \n { | }] $m]
puts $::errorCode
puts [string map [list \n { | }] $::errorInfo]
proc guarded {x} {
    if {$x} { return [expr {1 +}] }
    return ok
}
puts [guarded 0]
puts [catch {guarded 1} m]:[string map [list \n { | }] $m]
puts [catch {expr {"a" starts_with "b"}} m]:[string map [list \n { | }] $m]
# The folded `return [expr …]` logs its inner `expr`, at run time too.
proc ratio {x} { return [expr {1/$x}] }
catch {ratio 0}
puts [string map [list \n { | }] $::errorInfo]
