proc outer {} {
    set result [some_unknown_cmd [another_unknown $x]]
    foreach item $result {
        also_unknown $item
    }
    return $result
}
