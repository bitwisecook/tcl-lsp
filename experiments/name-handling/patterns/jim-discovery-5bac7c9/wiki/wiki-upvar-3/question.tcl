
        proc link {name local} {uplevel 1 [list upvar 1 $name $local]}
        proc assign {name} {link $name value; set value changed}
        proc caller {} {set x old; assign x; return $x}
        caller
    
