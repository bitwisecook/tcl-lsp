
        proc take args {return $args}
        proc extra {} {return script}
        set prefix "take\nextra"
        list [{*}$prefix] [eval $prefix]
    
