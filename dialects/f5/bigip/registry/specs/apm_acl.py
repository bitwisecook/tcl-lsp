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
            "apm_acl",
            module="apm",
            object_types=("acl",),
        ),
        header_types=(("apm", "acl"),),
        properties=(
            BigipPropertySpec(name="acl-order", value_type="integer", required=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="entries",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="action",
                        value_type="enum",
                        in_sections=("entries",),
                        required=True,
                        enum_values=("allow", "continue", "discard", "reject", "unspec"),
                    ),
                    BigipPropertySpec(
                        name="dst-end-port",
                        value_type="unknown",
                        in_sections=("entries",),
                        allow_none=True,
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="dst-start-port",
                        value_type="unknown",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="dst-subnet",
                        value_type="unknown",
                        in_sections=("entries",),
                    ),
                    BigipPropertySpec(
                        name="host",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="log",
                        value_type="enum",
                        in_sections=("entries",),
                        allow_none=True,
                        enum_values=("config", "none", "packet", "summary", "verbose"),
                    ),
                    BigipPropertySpec(
                        name="paths",
                        value_type="string",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="protocol",
                        value_type="integer",
                        in_sections=("entries",),
                        default="0",
                    ),
                    BigipPropertySpec(
                        name="scheme", value_type="unknown", in_sections=("entries",)
                    ),
                    BigipPropertySpec(
                        name="src-end-port",
                        value_type="unknown",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="src-start-port",
                        value_type="unknown",
                        in_sections=("entries",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="src-subnet",
                        value_type="unknown",
                        in_sections=("entries",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("entries",),
                required=True,
                enum_values=("allow", "continue", "discard", "reject", "unspec"),
            ),
            BigipPropertySpec(
                name="dst-end-port",
                value_type="unknown",
                in_sections=("entries",),
                allow_none=True,
                default="0",
            ),
            BigipPropertySpec(
                name="dst-start-port",
                value_type="unknown",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(name="dst-subnet", value_type="unknown", in_sections=("entries",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="log",
                value_type="enum",
                in_sections=("entries",),
                allow_none=True,
                enum_values=("config", "none", "packet", "summary", "verbose"),
            ),
            BigipPropertySpec(
                name="paths",
                value_type="string",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="integer",
                in_sections=("entries",),
                default="0",
            ),
            BigipPropertySpec(name="scheme", value_type="unknown", in_sections=("entries",)),
            BigipPropertySpec(
                name="src-end-port",
                value_type="unknown",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="src-start-port",
                value_type="unknown",
                in_sections=("entries",),
                allow_none=True,
            ),
            BigipPropertySpec(name="src-subnet", value_type="unknown", in_sections=("entries",)),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="path-match-case",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("dynamic", "static"),
                default="static",
            ),
        ),
    )
