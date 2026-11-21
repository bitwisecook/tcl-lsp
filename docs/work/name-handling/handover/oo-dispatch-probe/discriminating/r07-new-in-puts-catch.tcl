oo::class create C { method m {a} {} }
set o [C new]
puts [catch {$o m} msg]
