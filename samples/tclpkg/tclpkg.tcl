# tclpkg.tcl — sample manifest for demonstrating the tclpkg package manager.
# Used by the README screenshots and the docs/kcs/ smoke test walkthrough.

package     demo-app
version     0.1.0
description "A demo Tcl application showing tclpkg in action"
license     MIT
author      "Tcl Developer <dev@example.org>"

tcl >=8.6

# Production dependencies
require json    1.0.0
require http    2.9.0

# Development-only dependencies
dev-require tcltest 2.5.0

provides demo::serve
entry    main.tcl
