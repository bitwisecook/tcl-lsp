oo::class create C {
    method Hidden {} { return h }
    method pub {} { return [my Hidden] }
}
set o [C new]
puts [$o pub]
puts [catch {$o Hidden} msg]:$msg
