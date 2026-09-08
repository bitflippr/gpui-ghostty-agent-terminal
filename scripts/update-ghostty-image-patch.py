#!/usr/bin/env python3
"""Capture image-extension changes made in the pinned Ghostty submodule."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "vendor/ghostty"
REVISION = "4c725242b7dbe8c77c6e227ef1f9540c5ef17921"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=SOURCE)


if git("rev-parse", "HEAD").decode().strip() != REVISION:
    raise SystemExit("The Ghostty checkout must match the pinned revision")
parts = [git("diff", "--binary", "HEAD")]
for name in git("ls-files", "--others", "--exclude-standard").decode().splitlines():
    if not name.startswith(("src/", "include/")):
        raise SystemExit(f"Unexpected untracked file in Ghostty: {name}")
    lines = (SOURCE / name).read_text(encoding="utf-8").splitlines(keepends=True)
    patch = (f"diff --git a/{name} b/{name}\nnew file mode 100644\n"
             f"--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{len(lines)} @@\n")
    patch += "".join("+" + line for line in lines)
    if not patch.endswith("\n"):
        patch += "\n\\ No newline at end of file\n"
    parts.append(patch.encode("utf-8"))
destination = ROOT / "vendor/ghostty-patches/terminal-images.patch"
destination.parent.mkdir(exist_ok=True)
temporary = destination.with_suffix(".patch.tmp")
temporary.write_bytes(b"".join(parts))
temporary.replace(destination)
print(f"Updated {destination.relative_to(ROOT)}")
