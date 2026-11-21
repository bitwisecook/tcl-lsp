oo::class create C {
    method who {} { return [list [self] [self class] [self method] [self namespace]] }
}
puts [llength [[C new] who]]
