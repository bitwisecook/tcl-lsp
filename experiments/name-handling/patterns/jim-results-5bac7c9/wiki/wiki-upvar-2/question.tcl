
        proc assign {name} {upvar $name value; set value changed}
        proc caller {} {set 0a old; catch {assign 0a}}
        caller
    
