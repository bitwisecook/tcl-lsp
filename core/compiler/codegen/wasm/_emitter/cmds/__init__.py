"""Per-command WASM emit hooks — importing this package registers all hooks via REGISTRY."""

from . import (
    catch_,
    dict_,
    info_,
    list_,
    return_,
    runtime_,
    scope_,
    set_,
    string_,
    uplevel_,
)

__all__ = [
    "catch_",
    "dict_",
    "info_",
    "list_",
    "return_",
    "runtime_",
    "scope_",
    "set_",
    "string_",
    "uplevel_",
]
