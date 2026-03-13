proc fetch {url} {
    set result [http::geturl $url -timeout 30]
    return $result
}
