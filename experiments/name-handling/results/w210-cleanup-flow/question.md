# Cleanup read after an incompletely modelled setup write

Do the existing direct/alias Tcltest controls stop reporting `prepared` in cleanup while retaining their original `$absent` body warning?

The upstream selected C8.4.20 `tmp/tcl8.4.20/library/tcltest/tcltest.tcl` lifecycle catches setup, enters body only for successful setup, and always catches cleanup. The existing direct and alias negative controls use setup `{set prepared 1}`, body `{puts $prepared; puts $absent}`, and cleanup `{puts $prepared}`. The provider source input repair observed two warnings instead of one, including `prepared` in cleanup.

The shared original declaration graph previously routed the write handler's abrupt completion to cleanup with its pre-store state. Successful output roles describe only the normal continuation; they do not establish the failed handler's partial stores. The repair carries scoped per-target uncertainty only on an abrupt handler edge entering the authenticated lifecycle cleanup. The lifecycle owner supplies that destination; ordinary loop and command exits supply none. Joins retain it, a described normal store or removal settles it, and it withdraws absence/value advice for those targets. Other names retain their independent facts. No command-name filter, source rewrite, Native entry, completed store, normal execution or dispatch permission is manufactured.

The original oracle program's zero versus two warnings is a separate admission question. This repair does not claim that boundary is fixed. Exact commands, frozen source inventories and raw streams record execution under the coordinator's source freeze and exclusive issues-cache lease. The original graph controls and direct/alias negative controls are rerun after restricting uncertainty to cleanup edges.
