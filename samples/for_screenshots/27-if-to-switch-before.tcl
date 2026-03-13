proc handle_method {method} {
    if {$method eq "GET"} {
    #   ^--- cursor on if
        set action read
    } elseif {$method eq "POST"} {
        set action create
    } elseif {$method eq "PUT"} {
        set action update
    } elseif {$method eq "DELETE"} {
        set action remove
    } else {
        set action unknown
    }
    return $action
}
