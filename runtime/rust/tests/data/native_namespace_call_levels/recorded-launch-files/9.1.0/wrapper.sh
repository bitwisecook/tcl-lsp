#!/bin/sh
export TCL_LIBRARY=/workspace/tcl-lsp/tmp/tcl9.1.0/library
exec /workspace/tcl-lsp/tmp/tcl9.1.0/unix/tclsh "$@"
