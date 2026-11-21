oo::class create C {
    method Helper {} { return helped }
    method go {} { link Helper; return [Helper] }
}
puts [[C new] go]
