# ConPTY performance patch and build recipe

The project bundle `1.24.260710001-agent.1` deliberately combines a patched
x64 host with the original Microsoft `conpty.dll` and ARM64 host. It is a
project-maintained derivative, not an unmodified Microsoft NuGet package.

The base redistributable is Microsoft.Windows.Console.ConPTY
`1.24.260710001`, published with Windows Terminal `v1.24.11911.0`:

https://github.com/microsoft/terminal/releases/download/v1.24.11911.0/Microsoft.Windows.Console.ConPTY.1.24.260710001.nupkg

Its package SHA256 is
`9382ad7becb7e4d84e300578d8e4f4df28f43d979d9055d978c42913c47e0e9d`.
The patched x64 host is built from that release's source tag, pinned to
`5a830b2bf7c053d5c7ac22208fe5a346cb5dd3dc`, plus `conpty-performance.patch`.
The NuGet metadata does not identify its exact source commit; this is a
same-release source build, not a claim of byte-for-byte reconstruction.

The original package's rule to keep its three runtime files together applies
to ordinary upstream updates. This bundle is an explicit, documented
exception: only the x64 implementation is rebuilt. The DLL's API and both
host locations remain unchanged. An x64 application on ARM64 still receives
the original native ARM64 host. No ARM64 performance improvement is claimed.

## Changes

The parser processes printable OSC data in runs using its existing SSE2/NEON
control-character scan. Controls, cancellation, termination, C1 handling, and
trace output retain their existing paths. Generic and input parser clients
continue to cache incomplete sequences. The console host output parser opts
out because its passthrough callback is a no-op and `WriteCharsVT` separately
forwards the original output. Finally, iTerm2 dispatch compares the supported
`SetMark` token directly, avoiding a scan and token allocation for image data.

These changes do not alter console input/output API handling, the PTY stream,
resize behavior, transport handles, or process lifetime code.

## Rebuilding x64

Use PowerShell 7, Git, VS 2022 Build Tools with the x64 C++ toolchain, and the
Windows SDK. The shipped host used MSVC 14.44.35207, SDK 10.0.26100.0, and x64
Release (`/O2 /GL`, without PGO). Allow approximately 10 GB for the source,
dependency cache, and build output. Choose a dedicated build directory on a
drive with sufficient space.

```powershell
./build-runtime.ps1 -BuildRoot ./conpty-build -RunTests
```

`-SourceDirectory` can reuse a checkout at the pinned revision. Resuming is
allowed only when its tracked diff exactly matches the shipped patch.
`-OutputFile` selects where to place the resulting x64 `OpenConsole.exe`.
`-WindowsSdkVersion` selects a different installed SDK; changing toolchain
versions may change the resulting binary hash. This recipe builds the x64
host only. Preserve the original DLL, ARM64 host, and license when assembling
the project bundle.

The script restores the two native NuGet dependencies needed for the host,
uses the upstream pinned vcpkg baseline, and optionally restores TAEF and runs
the parser suite plus the targeted iTerm2 dispatch test. Dependency caches are
placed under the supplied build directory where supported, and environment
changes are limited to the script's process and restored on exit.

`portable-paths.targets` applies MSVC `/d1trimfile` to remove the checkout root
from source references and `/PDBALTPATH:%_PDB%` to store a filename-only debug
record. Microsoft uses the former option in its
[WindowsAppSDK build](https://github.com/microsoft/WindowsAppSDK/blob/main/Directory.Build.props);
the latter is documented in
[MSVC's linker reference](https://learn.microsoft.com/en-us/cpp/build/reference/pdbaltpath-use-alternate-pdb-path).
`verify-host.ps1` rejects an executable containing the supplied build roots or
current user profile in ASCII/UTF-16, or a CodeView record other than
`OpenConsole.pdb`. The shipped host passed these checks.

The upstream repository contains both LF and CRLF source files. Keep the
patch's exact bytes; the accompanying `.gitattributes` disables conversion.

## Validation

The complete upstream parser suite passed: 762 total, 761 passed, 0 failed,
1 skipped. The skipped input round-trip test explicitly skips itself in the
unmodified upstream source because of upstream issue 4405.

New differential coverage checks all 65,536 UTF-16 code units inside OSC
payloads with C1 enabled and disabled, varied input splits, and sequence
caching disabled, against the existing scalar parser. Separate checks assert
exact passthrough bytes for BEL and ESC-ST termination at chunk sizes from
1 through 131072 code units.

The added real-adapter test passed for exact, empty, prefix, and malformed
SetMark values, all 65,536 UTF-16 suffix values, and a 1 MiB image body. It
checks the original contract: the first semicolon-delimited token must equal
`SetMark`.

The terminal project separately measures end-to-end timing, raw output, and
GUI behavior. Parser tests do not establish performance or GPU presentation
latency.

## Artifact SHA256

| File | SHA256 |
| --- | --- |
| Patched x64/OpenConsole.exe | `34a4fee566473926fb414c554eb70a8a05ff3fbaefb8bda414d27d45fba331a4` |
| Original conpty.dll | `39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8` |
| Original arm64/OpenConsole.exe | `ed7622fd0d3bedc9ab9f122f5e58edf0def9e7999224f52dd395ba9f54edbe09` |
| conpty-performance.patch | `1c41a94c7b83456c734c8df0f6811627498ac90c6cabf4be6ef29d9e05aac4da` |

Upstream source and the redistributable are copyright Microsoft Corporation
under the MIT license. See `LICENSE.txt`.
