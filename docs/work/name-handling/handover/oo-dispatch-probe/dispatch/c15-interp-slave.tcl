set i [interp create]
$i eval {proc p {a} { return $a }}
puts [$i eval {p 1}]
puts [catch {$i eval {p}} m]:$m ;# BAD
