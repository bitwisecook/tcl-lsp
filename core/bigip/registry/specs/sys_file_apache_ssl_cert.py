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
            "sys_file_apache_ssl_cert",
            module="sys",
            object_types=("file apache-ssl-cert",),
        ),
        header_types=(("sys", "file apache-ssl-cert"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
