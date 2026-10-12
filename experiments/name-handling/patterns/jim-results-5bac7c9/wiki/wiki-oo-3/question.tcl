
        proc take x {list old $x}
        oo::class create C {forward send ::take fixed}
        set obj [C new]
        rename take saved
        proc take x {list replacement $x}
        $obj send
    
