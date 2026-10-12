namespace eval ::nh {
    foreach name {red blue} {proc $name {} [list return $name]}
}
list [::nh::red] [::nh::blue]

