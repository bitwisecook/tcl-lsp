proc classify {s} {
    switch -regexp $s {
        {^\d+$}    { return "number" }
        {^[a-z]+$} { return "lower" }
        {^[A-Z]+$} { return "upper" }
        default    { return "mixed" }
    }
}
