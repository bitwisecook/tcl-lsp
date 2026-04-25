# nextto dispatches to a specific class in the MRO chain
oo::class create A {
    method greet {} { return "A" }
}
oo::class create B {
    superclass A
    method greet {} { return "B-[next]" }
}
oo::class create C {
    superclass B
    method greet {} { return "C-[nextto A]" }
}
[C new] greet
