# The sibling the e2e never opens.
#
# It exists only on the host filesystem, reachable through the preopened
# directory the harness passes to `wasmtime --dir`. Every answer the session
# gets about `helper` therefore had to come from `vfs::NativeStore` reading a
# real file inside the sandbox — which is the whole point of the WASI transport
# over the browser one.

proc helper {value} {
    return [expr {$value * 2}]
}

proc unused_helper {} {
    return {}
}
