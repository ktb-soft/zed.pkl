# Pkl for Zed

[Pkl](https://pkl-lang.org) language support for Zed: syntax highlighting,
indentation, outline, text objects, and the [pkl-lsp](https://github.com/apple/pkl-lsp)
language server.

## Requirements

- **Java 23 or newer**, for language server features. The extension checks the
  version of the `java` it finds and tells you if it is missing or too old.
  On macOS, `/usr/bin/java` exists even with no JDK installed, so "found java"
  is not the same as "have a JDK".
- **The `pkl` CLI** on your `PATH`, for `PklProject` dependency resolution and
  `package://` imports. Everything else works without it.

The pkl-lsp jar is downloaded automatically.

## Known issue: `Could not find Pkl CLI in PATH`

pkl-lsp up to and including 0.8.0 asks the editor for its configuration using an
invalid `scopeUri` of `"Pkl"`. Zed rejects the malformed request, so pkl-lsp
never receives `pkl.cli.path` — and because its `PATH` fallback lives in the code
that handles the reply, it never searches your `PATH` either. The resulting
message is misleading: your `PATH` is fine.

This is fixed upstream but unreleased. Until pkl-lsp 0.9.0 ships, `PklProject`
dependency resolution, `package://` imports, and the download-package code action
are unavailable on the bundled jar. Everything else — highlighting, hover,
go-to-definition, completion, formatting, diagnostics — works normally.

### Using the patched jar

A build of pkl-lsp `main` with the fix is attached to this repository's releases
as `pkl-lsp-0.8.0-dev-47d5d55.jar`. It is an unmodified build of
[apple/pkl-lsp](https://github.com/apple/pkl-lsp) at commit `47d5d55`,
redistributed under Apache-2.0; `LICENSE.txt` and `NOTICE.txt` are attached
alongside it.

Download it to a directory of your choice — **not** the extension's work
directory, whose contents this extension prunes — and point Zed at it:

```jsonc
{
  "lsp": {
    "pkl": {
      "binary": {
        "path": "/path/to/java",
        "arguments": ["-jar", "/path/to/pkl-lsp-0.8.0-dev-47d5d55.jar"]
      }
    }
  }
}
```

Or build it yourself. Requires JDK 25:

```sh
git clone https://github.com/apple/pkl-lsp && cd pkl-lsp && ./gradlew shadowJar
# build/libs/pkl-lsp-<version>-SNAPSHOT.jar
```

**Once pkl-lsp 0.9.0 is released, delete the `binary` setting.** The bundled
downloader picks up the real release and the workaround becomes unnecessary.

See [docs/adr/01-00-00-work-around-pkl-lsp-scope-uri.md](docs/adr/01-00-00-work-around-pkl-lsp-scope-uri.md).

## Settings

Configured under the `pkl` language server key in your Zed `settings.json`.

Run a different server binary or jar:

```jsonc
{
  "lsp": {
    "pkl": {
      "binary": {
        "path": "/path/to/java",
        "arguments": ["-jar", "/path/to/pkl-lsp-all.jar"]
      }
    }
  }
}
```

Override the workspace configuration sent to pkl-lsp. Keys are flat and dotted,
because that is how Zed looks up configuration sections:

```jsonc
{
  "lsp": {
    "pkl": {
      "settings": {
        "pkl.cli.path": "/opt/homebrew/bin/pkl",
        "pkl.formatter.grammarVersion": "2",
        "pkl.projects.excludedDirectories": [".vendir"]
      }
    }
  }
}
```

`pkl.cli.path` defaults to whatever `pkl` resolves to on your `PATH`.

## Development

```sh
mise install          # Rust toolchain
cargo test            # unit tests, host target
cargo clippy --target wasm32-wasip2 --all-targets -- -D warnings
cargo build --target wasm32-wasip2
```

Install the working tree via `zed: install dev extension`.

## License

MIT. See [LICENSE](LICENSE).
