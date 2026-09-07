# libghostty-vt image extension

`terminal-images.patch` extends the pinned Ghostty revision
`4c725242b7dbe8c77c6e227ef1f9540c5ef17921`. It supplies iTerm2 transfers,
decoding/download callbacks, Kitty animation and relative placements, and
the C APIs used by the terminal renderer.

Normal Cargo builds clone the local pinned submodule into `OUT_DIR`, check out
the pinned revision, and apply this patch. The prepared source is keyed by the
revision and patch contents. Builds never require a dirty submodule or an
unpublished fork commit.

To edit the extension, start with a clean submodule at the pinned revision:

```text
git -C vendor/ghostty apply ../ghostty-patches/terminal-images.patch
```

Make changes in the submodule, then capture them in the parent repository:

```text
python scripts/update-ghostty-image-patch.py
```

The capture script includes new source/header files and rejects a different
upstream revision. Cargo watches the patch, so regenerate it before testing
changes. Validate protocol integration with the commands in
[`docs/terminal-images.md`](../../docs/terminal-images.md). Library-level tests
can also be run from the patched submodule:

```text
zig build test-lib-vt -Demit-lib-vt=true -Demit-xcframework=false -Dapp-runtime=none -Dtest-filter=kitty
```

After capturing and validating the patch, restore the submodule using the
exact patch in reverse:

```text
git -C vendor/ghostty apply --reverse --check ../ghostty-patches/terminal-images.patch
git -C vendor/ghostty apply --reverse ../ghostty-patches/terminal-images.patch
```

Commit the parent patch and integration changes, keeping the submodule pointer
and working tree clean. Do not silently update the pinned upstream revision.
