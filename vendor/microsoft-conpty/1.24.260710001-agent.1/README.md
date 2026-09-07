# Project ConPTY bundle

This bundle contains a patched x64 `OpenConsole.exe` built from Windows
Terminal release `v1.24.11911.0`, commit
`5a830b2bf7c053d5c7ac22208fe5a346cb5dd3dc`. The original Microsoft
`conpty.dll`, ARM64 host, and license come from the same release's ConPTY
NuGet package `1.24.260710001`.

The x64 host batches OSC parsing, removes an unused copy of incomplete
sequences, and avoids scanning image payloads for the unrelated SetMark
action. ARM64 continues to use the original native host; its performance
has not been changed or measured here. This is a project-maintained
derivative of the Microsoft package.

See the [source patch, build recipe, validation, and SHA256
manifest](../../microsoft-conpty-patches/README.md). The Windows executable
embeds these files and validates their bytes before loading. Runtime cache
directories are keyed by the bundle version and content, and a per-user
file lock serializes cache creation and repair.
