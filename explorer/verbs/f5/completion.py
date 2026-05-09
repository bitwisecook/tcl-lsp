"""``f5 completion`` — print a shell completion script for bash/fish/zsh."""

from __future__ import annotations

import argparse
import sys
from importlib import resources

from ._registry import verb

_SHELLS: tuple[str, ...] = ("bash", "fish", "zsh")

_INSTALL_HINTS: dict[str, str] = {
    "bash": (
        "Install (user):\n"
        "  mkdir -p ~/.local/share/bash-completion/completions\n"
        "  f5 completion bash > ~/.local/share/bash-completion/completions/f5\n"
        "Or eagerly source it from ~/.bashrc:\n"
        "  source <(f5 completion bash)"
    ),
    "fish": (
        "Install:\n"
        "  mkdir -p ~/.config/fish/completions\n"
        "  f5 completion fish > ~/.config/fish/completions/f5.fish\n"
        "Then start a new fish session."
    ),
    "zsh": (
        "Install (per-user):\n"
        '  mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"\n'
        '  f5 completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"\n'
        "Then add this to ~/.zshrc before `compinit`:\n"
        '  fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)\n'
        "Or place it in any directory already on $fpath."
    ),
}


@verb(
    "completion",
    aliases=(),
    help="Print a bash / fish / zsh completion script for the f5 CLI.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Print a shell completion script for the f5 CLI.  Pipe the output "
        "into the appropriate completion directory for your shell so that "
        "verb names, flags, and `*.conf` / `*.scf` paths complete on Tab."
    )
    p.epilog = "Examples:\n  " + "\n  ".join(
        f"f5 completion {shell:<4}  # see install hints with --hint" for shell in _SHELLS
    )
    p.formatter_class = argparse.RawDescriptionHelpFormatter
    p.add_argument(
        "shell",
        choices=_SHELLS,
        help="Shell to print a completion script for.",
    )
    p.add_argument(
        "--hint",
        action="store_true",
        help="Print install instructions for the chosen shell to stderr.",
    )
    p.set_defaults(handler=_run_completion)


def _run_completion(args: argparse.Namespace) -> int:
    try:
        script = (
            resources.files("explorer.completions")
            .joinpath(f"f5.{args.shell}")
            .read_text(encoding="utf-8")
        )
    except FileNotFoundError:
        print(
            f"error: completion script for {args.shell} not bundled with this build",
            file=sys.stderr,
        )
        return 2

    sys.stdout.write(script)
    if args.hint:
        print(_INSTALL_HINTS[args.shell], file=sys.stderr)
    return 0
