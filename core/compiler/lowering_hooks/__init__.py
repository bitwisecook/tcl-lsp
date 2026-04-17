"""Per-command IR lowering hooks."""


def register_all() -> None:
    """Register all per-command lowering hooks on the REGISTRY."""
    from . import _control, _var

    _var.register()
    _control.register()
