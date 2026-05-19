"""Entry point for ``python -m explorer`` and the CLI zipapp."""

from __future__ import annotations

import sys

from tooling.explorer.cli import main

sys.exit(main())
