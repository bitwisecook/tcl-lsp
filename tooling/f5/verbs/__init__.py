"""F5-specific CLI verbs for the ``f5`` command.

Verbs here use the verb registry in :mod:`tooling.f5.verbs._registry`,
which is independent of the ``tcl`` / ``irule`` registry under
:mod:`tooling.tcl.verbs._registry`.
"""


def load_verbs() -> None:
    """Import all ``@verb``-decorated f5 modules, triggering their registrations."""
    from . import (  # noqa: F401
        cleanup,
        completion,
        convert,
        diff,
        enrich_pcapng,
        enrich_wireshark,
        explain,
        explain_flow,
        extract,
        fetch,
        graph,
        grep,
        merge,
        pcap_remap,
        pull,
        push,
        query,
        redact,
        registry,
        rename,
        secrets,
        split,
        stats,
        tmsh,
        unredact,
        validate,
    )
