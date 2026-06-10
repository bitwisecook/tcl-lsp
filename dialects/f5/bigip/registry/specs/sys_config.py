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
            "sys_config",
            module="sys",
            object_types=("config",),
        ),
        header_types=(("sys", "config"),),
        properties=(
            BigipPropertySpec(name="base", value_type="unknown"),
            BigipPropertySpec(name="binary", value_type="unknown"),
            BigipPropertySpec(name="current-partition", value_type="unknown"),
            BigipPropertySpec(name="exclude-gtm", value_type="unknown"),
            BigipPropertySpec(name="file", value_type="unknown"),
            BigipPropertySpec(name="files-folder", value_type="unknown"),
            BigipPropertySpec(name="from-terminal", value_type="unknown"),
            BigipPropertySpec(name="gtm-only", value_type="unknown"),
            BigipPropertySpec(name="load", value_type="unknown"),
            BigipPropertySpec(name="merge", value_type="unknown"),
            BigipPropertySpec(name="no-passphrase", value_type="unknown"),
            BigipPropertySpec(name="partitions", value_type="unknown"),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="replace", value_type="unknown"),
            BigipPropertySpec(name="save", value_type="unknown"),
            BigipPropertySpec(name="tar-file", value_type="unknown"),
            BigipPropertySpec(name="time-stamp", value_type="unknown"),
            BigipPropertySpec(name="user-only", value_type="unknown"),
            BigipPropertySpec(name="verify", value_type="unknown"),
            BigipPropertySpec(name="wait", value_type="unknown"),
        ),
    )
