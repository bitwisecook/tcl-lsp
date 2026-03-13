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
            "sys_management_ovsdb",
            module="sys",
            object_types=("management-ovsdb",),
        ),
        header_types=(("sys", "management-ovsdb"),),
        properties=(
            BigipPropertySpec(name="bfd-route-domain", value_type="string"),
            BigipPropertySpec(name="ca-cert-file", value_type="string"),
            BigipPropertySpec(name="cert-file", value_type="string"),
            BigipPropertySpec(name="cert-key-file", value_type="string"),
            BigipPropertySpec(name="controller-addresses", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="flooding-type", value_type="enum", enum_values=("multipoint", "replicator")
            ),
            BigipPropertySpec(name="log-level", value_type="string"),
            BigipPropertySpec(
                name="logical-routing-type",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "backhaul"),
            ),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="tunnel-floating-addresses", value_type="string"),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="tunnel-maintenance-mode", value_type="enum", enum_values=("active", "passive")
            ),
        ),
    )
