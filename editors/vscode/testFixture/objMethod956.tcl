oo::class create Bar956 {
    method get {key} { return $key }
}
set b [Bar956 new]
puts [$b get foo]
