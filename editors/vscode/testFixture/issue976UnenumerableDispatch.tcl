# FP guard for issue #976: a dispatch word whose value cannot be enumerated
# may name any proc with any argument, so no seed in the file can be
# claimed complete. Must draw no I230.
proc i976opaqueHelper {mode} {
    if {$mode eq "prod"} {
        set x 1
    } else {
        set x 2
    }
}
i976opaqueHelper prod
i976opaqueHelper prod
set cmd [gets stdin]
$cmd dev
