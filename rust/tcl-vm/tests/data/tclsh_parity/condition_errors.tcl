# The errorInfo an error in an if, while or for test leaves: the command the
# test belongs to is logged, after the expression's parse frame.
proc info_of {script} {
    catch {uplevel 1 $script}
    string map [list \n { | }] $::errorInfo
}
puts [info_of {expr {1 +}}]
puts [info_of {if {1 +} {}}]
puts [info_of {while {1 +} {}}]
puts [info_of {for {} {1 +} {} {}}]
puts [info_of {if 1 {} elseif {1 +} {}}]
puts [info_of {set e {1 +}; expr $e}]
puts [info_of {if {$nosuch} {}}]
proc p1 {} { if {1 +} {} }
proc p2 {} { expr {1 +} }
proc p3 {} { while {1 +} {} }
proc p4 {} { for {} {1 +} {} {} }
proc p5 {} { if {$nosuch} {} }
proc p6 {} { if {[error x]} {} }
foreach p {p1 p2 p3 p4 p5 p6} {
    puts [info_of $p]
}
