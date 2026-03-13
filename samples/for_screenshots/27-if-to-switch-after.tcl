proc handle_method {method} {
    switch -exact -- $method {
        "GET" {
            set action read
        }
        "POST" {
            set action create
        }
        "PUT" {
            set action update
        }
        "DELETE" {
            set action remove
        }
        default {
            set action unknown
        }
    }
    return $action
}
