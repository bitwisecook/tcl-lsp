"""Per-command bytecoded codegen hooks."""


def register_all() -> None:
    """Register all per-command codegen hooks on the REGISTRY."""
    from . import _array, _control, _dict, _info, _list, _misc, _namespace, _regexp, _string, _var

    _var.register()
    _string.register()
    _list.register()
    _dict.register()
    _regexp.register()
    _control.register()
    _info.register()
    _namespace.register()
    _array.register()
    _misc.register()
