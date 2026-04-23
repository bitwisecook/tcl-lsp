"""Per-command WASM emit hooks — importing this package registers all hooks via REGISTRY."""

from . import dict_, info_, list_, return_, runtime_, set_, string_, uplevel_

__all__ = ["dict_", "info_", "list_", "return_", "runtime_", "set_", "string_", "uplevel_"]
