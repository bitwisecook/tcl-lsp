# An error folding a constant subexpression C raises as it compiles: the
# message is in errorInfo before the command holding it is logged.
proc info_of {script} {
    catch {uplevel 1 $script}
    string map [list \n { | }] $::errorInfo
}
proc p {} { while {"a" + 1} {} }
puts [info_of p]
puts [info_of {while {"a" + 1} {}}]
