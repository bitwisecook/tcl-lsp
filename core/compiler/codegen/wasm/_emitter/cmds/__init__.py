"""Per-command WASM emit hooks — importing this package registers all hooks via REGISTRY.

Import order matters: specialised hooks must register BEFORE
``runtime_`` so the auto-registration loop skips commands that
already have a hook.  ``register_wasm_emitter`` is first-writer-wins.
"""

from . import (
    append_,
    catch_,
    concat_,
    dict_,
    fconfigure_,
    format_,
    info_,
    lappend_,
    linsert_,
    list_,
    lreplace_,
    lsort_,
    puts_,
    return_,
    scope_,
    set_,
    string_,
    uplevel_,
    # runtime_ must come last — it auto-registers generic runtime
    # hooks for every spec with a ``wasm_runtime_import`` that
    # doesn't already have a specialised hook above.
    runtime_,
)

__all__ = [
    "append_",
    "catch_",
    "concat_",
    "dict_",
    "fconfigure_",
    "format_",
    "info_",
    "lappend_",
    "linsert_",
    "list_",
    "lreplace_",
    "lsort_",
    "puts_",
    "return_",
    "runtime_",
    "scope_",
    "set_",
    "string_",
    "uplevel_",
]
