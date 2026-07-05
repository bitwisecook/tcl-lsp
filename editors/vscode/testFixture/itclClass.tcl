itcl::class Dog {
    inherit Animal
    variable barks
    common total
    public method bark {volume} {
        set barks $volume
    }
    private variable secret 0
    constructor {args} {
        set barks 0
    }
}
