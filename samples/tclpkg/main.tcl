#!/usr/bin/env tclsh
# main.tcl — demo entry point for the tclpkg sample project.

package require json
package require http

proc demo::serve {port} {
    puts "Listening on port $port..."
    # (placeholder — real implementation would use the http package)
}

demo::serve 8080
