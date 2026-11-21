oo::class create C {
    method m {a b} { list }
    method e {a b} {}
}
set o [C new]
puts [catch {$o m 1} msg]:$msg
puts [catch {$o e 1} msg]:$msg
