
        proc build {} {
            foreach name {alpha beta} {proc ::$name {} {return $name}}
        }
        build
        catch {alpha}
    
