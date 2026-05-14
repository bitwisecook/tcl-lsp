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
            "sys_cluster",
            module="sys",
            object_types=("cluster",),
        ),
        header_types=(("sys", "cluster"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="alt-address",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="members",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="address",
                        value_type="enum",
                        in_sections=("members",),
                        allow_none=True,
                        enum_values=("none",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="alt-address",
                        value_type="enum",
                        in_sections=("members",),
                        allow_none=True,
                        enum_values=("none",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="priming",
                        value_type="enum",
                        in_sections=("members",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="address",
                value_type="enum",
                in_sections=("members",),
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="alt-address",
                value_type="enum",
                in_sections=("members",),
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="priming",
                value_type="enum",
                in_sections=("members",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="min-up-members", value_type="integer"),
            BigipPropertySpec(
                name="min-up-members-enabled",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
            ),
        ),
    )
