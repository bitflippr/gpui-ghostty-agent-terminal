# Terminal images and downloads

Kitty graphics and iTerm2 image transfers are handled inside libghostty-vt.
Terminal Session snapshots carry owned pixels and resolved placements to GPUI;
PTY output is never filtered or rewritten before the VT engine consumes it.

## Protocol coverage

Kitty supports RGB, RGBA and PNG, zlib compression, chunked transfers, queries,
image IDs and numbers, quiet replies, direct data, regular files, temporary
files and shared memory. Placements support cropping, scaling, cell offsets,
z ordering, cursor movement, Unicode placeholders and relative parents.
Animation supports frame transfers and edits, composition, playback timing,
looping and frame deletion. Image replacement removes previous placements.

iTerm2 supports `File` and `MultipartFile`/`FilePart`/`FileEnd`, BEL and ST
terminators, cell/pixel/percentage/automatic dimensions, aspect preservation,
and feature reporting. PNG, JPEG, GIF, APNG, WebP, BMP and TIFF decoding is
available; GIF, APNG and WebP animation retains frame timing and composition.
PDF, PICT and other macOS-specific image formats are not implemented.

Transfers with `inline=0` (the default) save files to Downloads. Names are
reduced to safe basenames, existing files are never overwritten, and the
terminal shows saving, completion or failure status. A bounded disk worker
keeps disk writes off the PTY reader. Completed files can be revealed in their
folder; receiving a file never runs it.

The implementation is bounded: 256 MiB of image storage per screen,
at most 4,096 animation frames, and 64 MiB per decoded raster frame or iTerm2
file. These limits can reject otherwise valid oversized transfers.

GIFs publish their first frame before decoding the rest of the animation.
A decoder worker queues at most two later frames, preserving the decoder's
frame disposal and timing. The terminal collects frames as playback advances;
deleting the image or dropping its Terminal Session cancels the worker. Retained
compressed input and decoder buffers are charged to the screen's image budget.
Later decoding or storage failures stop playback at the last valid frame.
At most eight GIF decoder workers run concurrently; additional GIFs use the
bounded synchronous decoder. APNG and WebP currently decode all frames up front.

## Rendering and lifetime

The VT engine owns placement anchors, scrollback, alternate-screen state,
relative-placement lifetime and animation clocks. Resizing updates the VT
engine and PTY transport with the same physical cell metrics. Standard terminal
size queries report those metrics to image clients.

GPUI clips placements to the viewport, draws images in Kitty's three z layers,
and paints the cursor last. Image generations share textures across placements;
retired textures are explicitly removed from the window atlas. Replicated edge
pixels prevent atlas padding from bleeding into enlarged images.
Decoded frames and uploaded textures retain source resolution; the GPU draws
only the requested destination area. Keeping the source preserves later crops
and larger placements. CPU downsampling was measured separately and added
texture-preparation work for the validation GIF, so it was not adopted.

Windows reads wait on output completion and an interrupt event, with no polling
sleep or idle spin. Resize, synchronization and shutdown wake pending reads;
cancellation completes before buffers are reused, and completed bytes precede
the reader barrier. ConPTY keeps a synchronous write endpoint. Optimized
development builds use Zig `ReleaseSafe`; unoptimized tests retain Zig `Debug`.

OSC payloads are collected in slices inside libghostty-vt. Controls still pass
through the VT state machine; the bulk path is checked against scalar parsing.
iTerm2 base64 decoding uses the existing SIMD decoder while retaining strict
alphabet, padding and trailing-bit validation. Multipart transfers decode each
part into the destination buffer, carrying incomplete quartets between parts.
Single-sequence transfers avoid copying the complete encoded payload.

Windows x64 uses a [patched ConPTY host](../vendor/microsoft-conpty-patches/README.md)
that batches printable OSC parsing, skips an unused incomplete-sequence copy,
and avoids scanning image bodies for the unrelated SetMark action. Controls
and raw passthrough retain their existing behavior. The original Microsoft
DLL and ARM64 host are preserved; ARM64 has no measured performance change.
The bundle's version and content identify its cache, and a per-user file lock
serializes cache publication and repair.

## Validation and reproduction

Run `python scripts/terminal-images-demo.py` in the terminal for a visual
fixture covering RGBA, transparency, cropping, iTerm2 PNG and Kitty animation.

Automated checks:

```text
cargo test --lib terminal_image::tests
cargo test --lib terminal_download::tests
cargo test --lib terminal_session::tests
```

Manual timing and client tests require local inputs. Set
`IMAGE_PROTOCOL_TEST_FILE` to an image and run
`cargo test --profile dev --lib profile_image_protocol_input -- --ignored --nocapture`.
On Windows, set `IMAGE_PROTOCOL_TEST_COMMAND` to a command that emits an
animation and run
`cargo test --profile dev --lib animated_image_client_round_trips_through_conpty -- --ignored --nocapture`.
The latter verifies changing pixels through a real ConPTY session for seven
seconds. It does not assert timing thresholds or modify client configuration.
It also reports command completion. Set `IMAGE_PROTOCOL_TEST_ANIMATION=0` to
measure completion alone, for example with the same client without its image.

For a comparison that waits for the command shell to become ready before
timing, set `IMAGE_PROTOCOL_BENCH_VARIANTS` to a JSON file such as:

```json
[
  {"label": "plain", "command": "image-client.exe", "expect_image": false},
  {"label": "image", "command": "image-client.exe fixture.gif", "expect_image": true}
]
```

Use commands and input paths valid from the terminal's working directory.
Run `cargo test --profile dev --lib compare_image_client_startup_through_conpty -- --ignored --nocapture`.
Each sample starts a fresh Terminal Session; each round rotates variant order.
`IMAGE_PROTOCOL_BENCH_ROUNDS` selects 1–100 rounds, defaulting to six. Output
records shell readiness, first image snapshot and command completion separately.
Tests can explicitly select another runtime with the absolute directory
`IMAGE_PROTOCOL_BENCH_CONPTY_DIR`; production builds ignore this variable.

### Windows performance checkpoint

The September 7 measurements used the original 22,813,430-byte, 498-by-498,
159-frame GIF, transferred without recompression. External test senders used
either a single iTerm2 sequence or optimized 64 KiB multipart sequences.
The sender implementations are not part of this repository. The before/after
comparison includes sender optimizations as well as terminal changes; it does
not isolate the terminal's contribution.

Twelve interleaved repetitions per variant, measured from command submission
immediately after spawning the shell through its completion marker:

| Configuration | Before median (range) | After median (range) |
| --- | --- | --- |
| Test client without GIF | 165.0 ms (162.6–195.1) | 165.1 ms (162.1–233.8) |
| Test client with GIF | 768.4 ms (758.5–811.9) | 259.5 ms (255.1–319.1) |

The earlier terminal build is `b95968c`, paired with the original sender; the
after measurement uses the optimized client and bundled x64 host. Total time
with the GIF fell 66%, and
its added delay over the run without the GIF fell from 603 ms to 94 ms, or 84%.

A separate twelve-round comparison waited for shell readiness before timing,
alternated runtime order, rotated client order, and used the same updated VT
engine for every sample:

| Client | Original Microsoft ConPTY | Patched bundled ConPTY |
| --- | --- | --- |
| Without GIF | 98.5 ms | 98.1 ms |
| Original image sender | 617.7 ms | 428.8 ms |
| Optimized multipart sender | 343.2 ms | 166.5 ms |

The optimized path's first image snapshot arrived at a median 154.5 ms; command
completion ranged from 161.4 to 189.2 ms. In-memory single-sequence VT input
through the first snapshot improved from a median 86 ms to 55 ms in three
paired runs. Preparing a 498-by-498 texture fell from about 0.86 ms to 0.38 ms
per frame; the check verifies identical channel order, alpha and replicated
borders. Higher Rust development optimization did not improve command timing,
so the normal development profile remains unchanged.

These are development-build observations, not startup guarantees or GPU
presentation timings. Shell creation, command submission, first snapshot and
visible presentation are distinct measurement boundaries. The complete client
test still observed all 159 distinct frames over seven seconds. Sanitized
per-run results are in [the benchmark data](benchmarks/terminal-images-2026-09-07.json).
ConPTY parsing and dispatch passed 761 upstream parser tests with one existing
skip, plus the targeted adapter test. The packaged host build recipe was also
executed successfully. Operator retesting confirmed the final GUI's visible
improvement. macOS/Linux runtime validation and GPU presentation measurements
remain open.

Windows visual validation includes alpha, cropped placements, scaled single
pixels, and a 498-by-498 animated GIF with 159 frames through an image client.
Client layout must account for cursor movement and scrolling caused by iTerm2
image output; restoring an old absolute screen row after a scroll is incorrect.
macOS/Linux runtime validation and non-raster format coverage remain open.

## Upstream extension

The pinned Ghostty submodule remains unchanged. The build applies
`vendor/ghostty-patches/terminal-images.patch` to a build-owned checkout; see
[the patch workflow](../vendor/ghostty-patches/README.md). External libghostty
libraries and headers must contain the same extensions if build overrides are
used.

Specifications: [Kitty graphics](https://sw.kovidgoyal.net/kitty/graphics-protocol/),
[iTerm2 images and file transfers](https://iterm2.com/documentation-images.html),
[iTerm2 feature reporting](https://iterm2.com/feature-reporting/).
