# Fixture for #727: go-to-def of a TclOO method parameter resolves to its name.
oo::class create C {
    method greet {name greeting} {

        return "$greeting, $name"
    }
}
