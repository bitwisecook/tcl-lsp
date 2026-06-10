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
            "net_fdb_tunnel",
            module="net",
            object_types=("fdb tunnel",),
        ),
        header_types=(("net", "fdb tunnel"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="records",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("records",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("records",),
                    ),
                    BigipPropertySpec(
                        name="endpoint",
                        value_type="string",
                        in_sections=("records",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="endpoints",
                        value_type="list",
                        in_sections=("records",),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="replicators",
                        value_type="list",
                        in_sections=("records",),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("records",),
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("records",)),
            BigipPropertySpec(
                name="endpoint",
                value_type="string",
                in_sections=("records",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="endpoints",
                value_type="list",
                in_sections=("records",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="replicators",
                value_type="list",
                in_sections=("records",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
