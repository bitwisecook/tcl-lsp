proc classify {name} {
    switch -glob $name {
        *.tcl { return "tcl" }
        *.py  { return "python" }
        test* { return "test" }
        default { return "other" }
    }
}
