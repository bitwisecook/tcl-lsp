proc original {unused longVariable} {return $longVariable}
proc rewritten {unused a} {return $a}
set payload {literal $value [missing] {brace} spaces}
puts [list scalar [original UNUSED $payload] [rewritten UNUSED $payload]]
set payload ""
puts [list empty [original UNUSED $payload] [rewritten UNUSED $payload]]
set payload "N\u0000tail"
puts [list zero [original UNUSED $payload] [rewritten UNUSED $payload]]
