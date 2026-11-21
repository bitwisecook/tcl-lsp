oo::class create C {
    forward say puts
    method go {} { my say hello }
}
[C new] go
