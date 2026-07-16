oo::class create Counter {
    variable n
    method get {} { return $n }
    method bump {} { incr n }
}
