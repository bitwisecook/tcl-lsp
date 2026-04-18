try {
    set x 42
} on ok {result} {
    set y $result
} on error {msg opts} {
    set y "error: $msg"
}
