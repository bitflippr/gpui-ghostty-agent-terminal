# Draconis++ image transfer test client

This patch adds a Windows image interoperability client with bounded iTerm2 multipart transfers. Clang builds on Windows x64 select AVX2 base64 encoding after checking CPU and operating-system support; other configurations use the portable scalar encoder. It reads the original file into memory once, then encodes and writes blocks using reusable buffers. On Windows it writes ASCII protocol data through `WriteConsoleW` when stdout is a console and uses `WriteFile` when redirected; other platforms retain standard stream output. Each complete FilePart, including framing, fits in 64 KiB.

Source: [skulldogged/draconisplusplus-monorepo](https://github.com/skulldogged/draconisplusplus-monorepo), commit `edfbccb85bed14c7325f4841a42c84221b99b29c`. The patch includes GIF dimension detection, a Windows terminal cell-size query, and image/text alignment. Windows inline-image support remains explicitly gated by `DRAC_TEST_INLINE_IMAGES=1`.

The original GIF is transferred unchanged, preserving its frames, timing, and looping.

Draconis++ is copyright 2025 pupbrained and other contributors, under the
[MIT license](LICENSE.draconis).

## Build

Use a clean Draconis++ checkout at the source commit and its normal configured native Windows Meson/Ninja build. Keep `draconis-image-client.patch` outside that checkout and set `$patch` to its resolved full path. Run these commands from the Draconis++ checkout:

```powershell
git apply --check $patch
if ($LASTEXITCODE -ne 0) { throw 'Patch does not apply to this checkout' }
git apply $patch
if ($LASTEXITCODE -ne 0) { throw 'Patch application failed' }
ninja -C build core/src/CLI/draconis++.exe
if ($LASTEXITCODE -ne 0) { throw 'Test client build failed' }
Copy-Item 'build/core/src/CLI/draconis++.exe' 'build/core/src/CLI/draconis-image-fast-test.exe' -ErrorAction Stop
$client = (Resolve-Path 'build/core/src/CLI/draconis-image-fast-test.exe' -ErrorAction Stop).Path
```

After retaining the test executable, reverse the patch and rebuild to restore the ordinary executable:

```powershell
git apply --reverse $patch
if ($LASTEXITCODE -ne 0) { throw 'Could not restore the original source' }
ninja -C build core/src/CLI/draconis++.exe
if ($LASTEXITCODE -ne 0) { throw 'Ordinary executable rebuild failed' }
```

Launch Agent Terminal with `--development` and `NO_COLOR` removed from its environment. Inside that terminal, set `$client` to the retained executable and `$gif` to the chosen GIF's resolved full path, then run:

```powershell
$env:DRAC_TEST_INLINE_IMAGES = '1'
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
& $client --logo-path $gif --logo-protocol iterm2 --logo-width 240 --logo-height 240
```

No other experiment variables are required.

## Validation

The encoder matched an independent bitstream reference for 131072 random length/alignment combinations, 1024 inputs ending immediately before an inaccessible guard page, and the complete test GIF. The standalone proof targets Windows x64 and requires LLVM Clang plus the Visual Studio C++ toolchain and Windows SDK. Run it from this artifact directory:

```powershell
clang++ --target=x86_64-pc-windows-msvc -std=c++23 -O2 -DNOMINMAX base64-performance-test.cpp -o base64-performance-test.exe
if ($LASTEXITCODE -ne 0) { throw 'Encoder proof build failed' }
& ./base64-performance-test.exe $gif
```

`verify-fast-image-wire.py` checks the multipart header's file size, ordered contiguous FileParts, final FileEnd, the 64 KiB framing limit, and exact decoded source bytes. With Python 3 available, capture stdout as bytes and verify it from this directory:

```powershell
$capture = @'
import os, subprocess, sys
environment = os.environ.copy()
environment.pop("NO_COLOR", None)
environment["DRAC_TEST_INLINE_IMAGES"] = "1"
with open(sys.argv[3], "wb") as output:
    subprocess.run(
        [sys.argv[1], "--logo-path", sys.argv[2], "--logo-protocol", "iterm2",
         "--logo-width", "240", "--logo-height", "240"],
        stdin=subprocess.DEVNULL, stdout=output, env=environment, check=True)
'@
$capture | python - $client $gif capture.bin
if ($LASTEXITCODE -ne 0) { throw 'Client output capture failed' }
python verify-fast-image-wire.py capture.bin $gif
```

This capture exercises the redirected `WriteFile` path; console output and visible rendering require terminal validation. The final client's 465-part transfer decoded byte-for-byte to the 22813430-byte input and produced no stderr output. Exact input bytes also preserve all 159 composited frames, delays, and looping.

End-to-end runtime comparisons belong to the terminal benchmark report. The isolated encoder comparison is not a GPU-presentation or command-completion benchmark.
