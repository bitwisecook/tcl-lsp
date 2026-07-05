foreach item $lst {
    puts $item
}
foreach {key val} $pairs {
    puts "$key=$val"
}
dict for {dkey dval} $d {
    puts $dkey
}
