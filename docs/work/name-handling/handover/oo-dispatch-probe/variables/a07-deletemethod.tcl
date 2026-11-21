oo::class create C {
    method a {} { return a }
    method b {} { return b }
}
oo::define C deletemethod b
set o [C new]
puts [$o a]
puts [catch {$o b} msg]:$msg
