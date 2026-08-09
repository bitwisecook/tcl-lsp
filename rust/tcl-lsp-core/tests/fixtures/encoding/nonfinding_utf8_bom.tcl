when HTTP_REQUEST {
    if { [HTTP::uri] eq "/a" } {
        log local0. "hit"
    }
}
