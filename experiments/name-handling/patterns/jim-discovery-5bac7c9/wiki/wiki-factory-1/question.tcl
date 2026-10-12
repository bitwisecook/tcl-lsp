
        namespace eval n {
            foreach name {alpha beta} {proc $name {} [list return $name]}
        }
        list [n::alpha] [n::beta]
    
