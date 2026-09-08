# Language server installation

`src/lib.rs` fetches the `pkl-lsp` jar from GitHub releases into the extension
work directory.

## Flow

1. Return the cached path if it still points at a valid jar.
2. Ask GitHub for the latest `apple/pkl-lsp` release.
3. If `pkl-lsp-<version>.jar` is already present and valid, use it.
4. Otherwise download the matching asset, validate it, and move it into place.
5. Delete every other `pkl-lsp-*` file in the work directory.

If step 2 fails — offline, rate limited, GitHub down — the newest valid
`pkl-lsp-*.jar` already on disk is used instead. Only when there is no usable jar
at all does the language server report `Failed`.

## Validity

A jar is valid when it opens and its first four bytes are the ZIP local file
header `PK\x03\x04`. Existence is not enough: Zed's `download_file` streams
directly onto the destination path with no temp file, so an interrupted transfer
leaves a truncated file that looks installed.

Downloads therefore land on `<name>.jar.part` first and are only renamed after
validation. A failed download is removed rather than left behind.

## Java

`worktree.which("java")` is not a usable check on macOS: `/usr/bin/java` is a
stub that exists with no JDK installed, prints "Unable to locate a Java Runtime",
and exits 1. The extension runs `java -version` and parses the major version out
of stderr, handling both modern (`"23.0.1"`) and legacy (`"1.8.0_281"`) formats.
Unparsable output is treated as no Java at all, which is exactly what the stub
produces.
