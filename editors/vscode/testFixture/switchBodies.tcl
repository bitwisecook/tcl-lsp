switch -exact -- $var {
    "value1" {
        set x 1
        puts hello
    }
    "value2" {
        return 2
    }
    default {
        puts other
    }
}
