proc original {} {set longVariable {literal $value [missing] spaces}; return $longVariable}
proc rewritten {} {set a {literal $value [missing] spaces}; return $a}
puts [list literal [original] [rewritten]]
proc original {} {set longVariable {}; return $longVariable}
proc rewritten {} {set a {}; return $a}
puts [list empty [original] [rewritten]]
proc original {} {set longVariable "N\u0000tail"; return $longVariable}
proc rewritten {} {set a "N\u0000tail"; return $a}
puts [list zero [original] [rewritten]]
