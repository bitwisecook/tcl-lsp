#!/bin/sh
export TCL_LIBRARY=/workspace/tcl-lsp/tmp/tcl9.0.4/library
exec /workspace/tcl-lsp/tmp/tcl9.0.4/unix/tclsh "$@"
