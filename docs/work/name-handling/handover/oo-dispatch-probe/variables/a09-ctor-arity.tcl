oo::class create C {
    constructor {a b} {}
}
puts [catch {C new 1} msg]:$msg
