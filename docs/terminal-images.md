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

Windows reads wait on output completion and an interrupt event, with no polling
sleep or idle spin. Resize, synchronization and shutdown wake pending reads;
cancellation completes before buffers are reused, and completed bytes precede
the reader barrier. ConPTY keeps a synchronous write endpoint. Optimized
development builds use Zig `ReleaseSafe`; unoptimized tests retain Zig `Debug`.

OSC payloads are collected in slices inside libghostty-vt. Controls still pass
through the VT state machine; the bulk path is checked against scalar parsing.

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

For the 22.8 MB, 159-frame GIF used during Windows validation, in-memory VT
input through the first image snapshot fell from about 1.0 seconds to 82 ms.
Repeated native client runs measured 0.81–0.85 seconds, with an additional slower
run at 1.37 seconds, compared with the earlier 1.58 seconds. Paired runs without
the GIF measured 0.17–0.25 seconds. These are
development-build observations, not startup guarantees or GPU presentation
timings. A controlled sender spent about 330 ms writing the 30.4 MB encoded
payload through ConPTY. Instrumenting the image client measured 62 ms building
its sequence and 484 ms writing it; the cell-size query returned in 16 ms.
Minimizing the remaining client and transport overhead is still an open
performance goal. Upstream ConPTY's
[OSC parser](https://github.com/microsoft/terminal/blob/main/src/terminal/parser/stateMachine.cpp)
collects strings character by character; this is a candidate for further
profiling, not a confirmed attribution of the entire remaining delay.

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
