proc test {} {
    set outer [catch {
        try {
            error "inner"
        } on error {msg} {
            set caught $msg
        }
    } result]
    list $outer $result
}
