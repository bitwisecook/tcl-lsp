proc bysubst {} {return $::seen}
proc bycommand {} {return [set ::seen]}
proc bydecl {} {global seen; return $seen}
set seen FIRST
puts [list [bysubst] [bycommand] [bydecl]]
set seen SECOND
puts [list [bysubst] [bycommand] [bydecl]]
