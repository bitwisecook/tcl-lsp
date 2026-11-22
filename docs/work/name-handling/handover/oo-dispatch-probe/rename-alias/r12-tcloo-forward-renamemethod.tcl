proc target {args} {return target:$args}
oo::class create C { forward f target a; method m {} {return m} }
set o [C new]
puts [$o f b]
rename target target2
puts [catch {$o f c} m]:$m
proc target {args} {return again:$args}
puts [$o f d]
oo::define C renamemethod m n
puts [list [$o n] [catch {$o m} e] $e]
