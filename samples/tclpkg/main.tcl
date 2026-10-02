#!/usr/bin/env tclsh
# main.tcl — demo entry point for the tclpkg sample project.

package require json
package require http

proc demo::serve {port} {
    puts "Listening on port $port..."
    # This demo only prints; a real server would use the http package here.
}

demo::serve 8080
