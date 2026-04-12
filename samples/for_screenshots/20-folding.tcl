proc render_cookie {name value} {
    return "$name=$value; Path=/; HttpOnly"
}

# <<--- cursor on left margin
proc classify_path {host path} {
    if {[string match "*/admin/*" $path]} {
        return admin
    }
    if {[string match "api.*" $host]} {
        return api
    }
    return web
}

set class [classify_path [HTTP::host] [HTTP::path]]
log local0.notice "class=$class"
