proc catchreturn {} {
    set fail [catch {
        return 1
    }]
    return 2
}
