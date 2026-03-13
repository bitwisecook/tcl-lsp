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
            "sys_core",
            module="sys",
            object_types=("core",),
        ),
        header_types=(("sys", "core"),),
        properties=(
            BigipPropertySpec(name="tmm-manage", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="tmm-max", value_type="string"),
            BigipPropertySpec(name="tmm-action", value_type="enum", enum_values=("skip", "rotate")),
            BigipPropertySpec(name="mcpd-manage", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="mcpd-max", value_type="string"),
            BigipPropertySpec(
                name="mcpd-action", value_type="enum", enum_values=("skip", "rotate")
            ),
            BigipPropertySpec(name="bigd-manage", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="bigd-max", value_type="string"),
            BigipPropertySpec(
                name="bigd-action", value_type="enum", enum_values=("skip", "rotate")
            ),
            BigipPropertySpec(name="retention", value_type="string"),
        ),
    )
