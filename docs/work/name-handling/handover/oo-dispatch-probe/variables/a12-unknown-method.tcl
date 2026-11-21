oo::class create C {
    method unknown {name args} { return "unk:$name" }
}
puts [[C new] whatever]
