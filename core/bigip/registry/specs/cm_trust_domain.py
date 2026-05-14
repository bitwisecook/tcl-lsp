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
            "cm_trust_domain",
            module="cm",
            object_types=("trust-domain",),
        ),
        header_types=(("cm", "trust-domain"),),
        properties=(
            BigipPropertySpec(name="add-device", value_type="unknown"),
            BigipPropertySpec(name="device-ip", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(name="device-name", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="device-port",
                value_type="unknown",
                in_sections=("add-device",),
            ),
            BigipPropertySpec(name="password", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="sha1-fingerprint",
                value_type="string",
                in_sections=("add-device",),
            ),
            BigipPropertySpec(name="username", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="deprecated", value_type="unknown"),
            BigipPropertySpec(
                name="devices",
                value_type="list",
                list_operators=frozenset(("delete",)),
            ),
            BigipPropertySpec(name="md5-fingerprint", value_type="string"),
            BigipPropertySpec(
                name="non-ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="remove-device", value_type="string"),
            BigipPropertySpec(name="serial", value_type="string"),
            BigipPropertySpec(name="sha1-fingerprint", value_type="string"),
            BigipPropertySpec(name="username", value_type="string"),
        ),
    )
