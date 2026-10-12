
        proc take {} {return main}
        interp create helper
        interp eval helper {proc take {} {return helper}}
        interp alias {} alias helper take
        alias
    
