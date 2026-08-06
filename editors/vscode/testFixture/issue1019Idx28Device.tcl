namespace eval ::Idx28::Xyce {
    oo::class create RModel {
        superclass ::Idx28::Model
        constructor {args} {
            my ArgsPreprocess {defw r} {name type} type {*}$args
        }
    }
}
