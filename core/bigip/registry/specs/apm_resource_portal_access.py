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
            "apm_resource_portal_access",
            module="apm",
            object_types=("resource portal-access",),
        ),
        header_types=(("apm", "resource portal-access"),),
        properties=(
            BigipPropertySpec(name="acl-order", value_type="integer", required=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="application-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="css-patching",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="flash-patching",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="host-replace-string", value_type="string", allow_none=True),
            BigipPropertySpec(name="host-search-strings", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="html-patching",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="items",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
            ),
            BigipPropertySpec(
                name="javascript-patching",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="patching-type",
                value_type="enum",
                enum_values=("full-patch", "min-patch"),
            ),
            BigipPropertySpec(
                name="path-match-case",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="proxy-host",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="proxy-port",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="publish-on-webtop",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="scheme-patching",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
        ),
    )
