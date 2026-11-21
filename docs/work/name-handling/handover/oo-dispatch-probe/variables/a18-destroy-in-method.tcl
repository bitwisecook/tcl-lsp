oo::class create C {
    destructor { puts gone }
    method die {} { my destroy; return after }
}
puts [[C new] die]
