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
            BigipPropertySpec(name="source-path", value_type="string"),
            BigipPropertySpec(name="passphrase", value_type="string"),
        ),
    )
