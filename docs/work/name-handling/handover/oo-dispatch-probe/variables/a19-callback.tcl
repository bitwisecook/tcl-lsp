oo::class create C {
    variable hits
    constructor {} { set hits 0 }
    method cb {} { incr hits }
    method go {} { after 0 [callback cb]; update; return $hits }
}
puts [[C new] go]
