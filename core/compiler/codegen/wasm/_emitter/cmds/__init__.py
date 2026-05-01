"""Per-command WASM emit hooks — importing this package registers all hooks via REGISTRY.

Each ``cmds/<cmd>_.py`` module registers its specialised hook at import
time via ``REGISTRY.register_wasm_emitter``.  After every specialised
module has run, :func:`runtime_.register_generic_runtime_hooks` fills
in generic runtime-dispatch hooks for every remaining spec with a
``wasm_runtime_import`` that lacks a bespoke hook.  The explicit
function call removes any dependency on module-import order, so the
import list below can stay alphabetical (ruff/isort-friendly).
"""

from . import (
    append_,
    apply_,
    catch_,
    concat_,
    dict_,
    fconfigure_,
    fcopy_,
    format_,
    info_,
    lappend_,
    linsert_,
    list_,
    lreplace_,
    lsort_,
    puts_,
    regexp_,
    return_,
    runtime_,
    scope_,
    set_,
    string_,
    uplevel_,
)

# Fill in generic hooks after every specialised hook has registered.
runtime_.register_generic_runtime_hooks()

__all__ = [
    "append_",
    "apply_",
    "catch_",
    "concat_",
    "dict_",
    "fconfigure_",
    "fcopy_",
    "format_",
    "info_",
    "lappend_",
    "linsert_",
    "list_",
    "lreplace_",
    "lsort_",
    "puts_",
    "regexp_",
    "return_",
    "runtime_",
    "scope_",
    "set_",
    "string_",
    "uplevel_",
]
