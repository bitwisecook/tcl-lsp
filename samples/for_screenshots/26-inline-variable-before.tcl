proc fetch {url} {
    set timeout 30
    #   ^--- cursor on set
    set result [http::geturl $url -timeout $timeout]
    return $result
}
