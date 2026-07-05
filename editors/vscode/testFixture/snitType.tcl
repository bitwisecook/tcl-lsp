snit::type Dog {
    variable barks
    typevariable count
    method bark {volume} {
        return $barks
    }
    typemethod tally {} {
        return $count
    }
    constructor {args} {
        set barks 0
    }
}
