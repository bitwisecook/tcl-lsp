# Fixture for #727: go-to-definition of a TclOO method parameter must resolve
# to the parameter name in the declaration, not the whole method body.
oo::class create C {
    method greet {name greeting} {
        return "$greeting, $name"
    }
}
