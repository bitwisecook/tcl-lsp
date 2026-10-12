
        oo::class create Meta {
            superclass oo::class
            method method {name params body} {::oo::define [self] method $name $params $body}
        }
        Meta create C
        C method value {} {return generated}
        set obj [C new]
        $obj value
    
