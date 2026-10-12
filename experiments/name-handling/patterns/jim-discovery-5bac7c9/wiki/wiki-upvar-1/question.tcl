
        proc assign {name} {upvar 1 $name value; set value changed}
        proc caller {} {set 0a old; assign 0a; set 0a}
        caller
    
