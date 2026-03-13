try {
    error "test error"
} on error {msg opts} {
    set caught $msg
} on ok {result} {
    set caught ""
}
