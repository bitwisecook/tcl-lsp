set name [binary format H* 6d69737300ff]; set c [catch {namespace origin $name} result]; list $c $result $::errorCode
