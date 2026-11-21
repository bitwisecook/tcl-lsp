oo::class create C {
    constructor {a b} { list }
}
puts [catch {C new 1} msg]:$msg
