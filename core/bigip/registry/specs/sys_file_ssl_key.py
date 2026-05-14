from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_file_ssl_key",
            module="sys",
            object_types=("file ssl-key",),
        ),
        header_types=(("sys", "file ssl-key"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="source-path", value_type="unknown"),
            BigipPropertySpec(
                name="cache-path",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="revision",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="system-path",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
