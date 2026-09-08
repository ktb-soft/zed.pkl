# 2026-09-08 — Extension audit and repair

Prompted by two persistent failures: a pkl-lsp jar reported as corrupt on every
start, and "PKL CLI not found in PATH" on every file open despite `pkl` being on
`PATH`.

## Root causes

**Corrupt jar.** `language_server_path` treated file existence as proof of a good
install. Zed's `download_file` streams onto the destination with no temp file, so
one interrupted transfer left a truncated jar that was then trusted forever. No
integrity check, no atomic rename, no way to force a re-download.

**CLI not found.** Not an extension bug. pkl-lsp 0.8.0 sends an invalid
`scopeUri`, Zed rejects the request, and pkl-lsp's `PATH` fallback is downstream
of the reply it never gets. Confirmed in `~/Library/Logs/Zed/Zed.log`. See
[../adr/01-00-00-work-around-pkl-lsp-scope-uri.md](../adr/01-00-00-work-around-pkl-lsp-scope-uri.md).

## Also found

- `which("java")` can never fail on macOS, so the "install Java" error was
  unreachable and the real failure surfaced as a server crash.
- Required Java version was documented as 22; pkl-lsp 0.8.0 targets 23.
- Every server start hit the GitHub API with no offline fallback.
- No `binary.path` override, and Zed does not supply one for extension servers.
- No `language_server_workspace_configuration`, so no pkl-lsp setting was
  reachable at all.
- `@methodName` in `injections.scm` warned on every load.
- Grammar pin was nine months stale.

## Not addressed

`.pcf` is mapped to the Pkl grammar in `languages/pkl/config.toml`. Whether
tree-sitter-pkl parses `.pcf` correctly was not verified; left as-is.
