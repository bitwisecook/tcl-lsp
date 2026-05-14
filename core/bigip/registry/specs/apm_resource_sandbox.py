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
            "apm_resource_sandbox",
            module="apm",
            object_types=("resource sandbox",),
        ),
        header_types=(("apm", "resource sandbox"),),
        properties=(
            BigipPropertySpec(name="base-uri", value_type="string"),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="files",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="content-type", value_type="string", in_sections=("files",)
                    ),
                    BigipPropertySpec(
                        name="file-type",
                        value_type="enum",
                        in_sections=("files",),
                        required=True,
                        enum_values=("citrix-bundle", "customization", "unknown"),
                    ),
                    BigipPropertySpec(name="filename", value_type="string", in_sections=("files",)),
                    BigipPropertySpec(name="folder", value_type="string", in_sections=("files",)),
                    BigipPropertySpec(
                        name="local-path", value_type="string", in_sections=("files",)
                    ),
                ),
            ),
            BigipPropertySpec(name="content-type", value_type="string", in_sections=("files",)),
            BigipPropertySpec(
                name="file-type",
                value_type="enum",
                in_sections=("files",),
                required=True,
                enum_values=("citrix-bundle", "customization", "unknown"),
            ),
            BigipPropertySpec(name="filename", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="folder", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="local-path", value_type="string", in_sections=("files",)),
            BigipPropertySpec(name="options", value_type="unknown"),
        ),
    )
