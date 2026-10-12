
        set x global
        proc link {name} {uplevel 1 [list global $name]}
        proc caller {} {link x; return $x}
        caller
    
