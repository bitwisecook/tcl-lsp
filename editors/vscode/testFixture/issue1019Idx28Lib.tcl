namespace eval ::Idx28 {
    oo::class create Utility {
        method ArgsPreprocess {switches params supp args} { return ok }
    }
    oo::class create Model {
        mixin Utility
    }
}
