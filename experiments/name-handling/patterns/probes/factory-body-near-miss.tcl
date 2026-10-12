namespace eval ::nh {
    foreach name {red blue} {proc $name {} {return $name}}
}
::nh::red

