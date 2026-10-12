
        proc unknown {head args} {return [linsert $args 0 $head]}
        list [info commands absent] [absent fixed]
    
