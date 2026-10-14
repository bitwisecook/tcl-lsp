foreach provided {name_probe :name_probe ::name_probe} {
    package provide $provided 1.0
    foreach requested {name_probe :name_probe ::name_probe ::::name_probe} {
        set code [catch {package require $requested} result]
        puts [list PACKAGE_IDENTITY $provided $requested $code $result]
    }
    package forget $provided
}
