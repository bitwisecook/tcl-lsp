"""Shared locations for the validation tools: repository root and a git-ignored work directory."""
from pathlib import Path
import os, subprocess

HERE = Path(__file__).resolve().parent
R = Path(subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], cwd=HERE, text=True).strip())
W = Path(os.environ.get('NH_VALIDATION_DIR', R / 'tmp/name-handling-validation'))
W.mkdir(parents=True, exist_ok=True)
JIMSH = R / 'tmp/jimtcl-oracle/jimsh'
CHECKPOINT09 = R / 'experiments/name-handling/results/owner-adoption-20261017/lsp-generic-consumers-09/source-validation.tar.gz'
