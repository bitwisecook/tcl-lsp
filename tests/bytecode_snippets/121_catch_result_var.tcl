proc foo {} {
    set fail [catch {
        return 1
    }]
    return $fail
}
