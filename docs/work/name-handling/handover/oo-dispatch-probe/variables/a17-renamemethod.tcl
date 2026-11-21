oo::class create C { method a {} { return a } }
oo::define C renamemethod a z
set o [C new]
puts [$o z]
puts [catch {$o a} msg]:$msg
