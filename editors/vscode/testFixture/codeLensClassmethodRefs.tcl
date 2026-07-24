oo::class create Factory {
    classmethod make {} {
        return [Factory new]
    }

    classmethod idle {} {
        return {}
    }
}
Factory make
