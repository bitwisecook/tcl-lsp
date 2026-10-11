oo::define ::MethodDerived method m {} {
    ::methodObserve derived-before-delete
    oo::define ::MethodBase deletemethod m
    ::methodObserve derived-after-delete
    set answer [next]
    ::methodObserve derived-after-next
    return $answer
}
