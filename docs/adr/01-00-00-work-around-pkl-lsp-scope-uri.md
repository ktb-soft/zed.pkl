# Work around pkl-lsp's invalid `scopeUri`

## Context

pkl-lsp through 0.8.0 requests its configuration with an invalid `scopeUri`:

```kotlin
ConfigurationItem().apply {
  scopeUri = "Pkl"
  section = "pkl.cli.path"
}
```

LSP types `scopeUri` as a `DocumentUri`. Zed parses it with `url::Url::parse`,
which rejects `"Pkl"` as a relative URL without a base, failing deserialization
of the whole `workspace/configuration` request:

```
ERROR [lsp] error deserializing workspace/configuration request: Error("relative URL without a base", ...)
```

pkl-lsp catches the resulting failure, leaves `pklCliPath` null, and reports
"Could not find Pkl CLI in PATH" on every file open. Its `PATH` fallback,
`findPklCliOnPath()`, is only reachable from the handler for the reply that never
arrives, so `PATH` is never actually searched.

VS Code is unaffected: `vscode-languageclient` runs `scopeUri` through
`Uri.parse()` in non-strict mode, which tolerates a scheme-less string.

Fixed upstream by `apple/pkl-lsp@24ecb45`, unreleased as of 0.8.0.

## Decision

Do not attempt to deliver `pkl.cli.path` by another route. All four channels were
checked against 0.8.0 and none work:

- `workspace/configuration` — the broken request itself.
- `workspace/didChangeConfiguration` — `PklWorkspaceService` discards
  `params.settings` and its subscriber merely re-issues the broken request.
- `initializationOptions` — decodes only `PklClientOptions`; no CLI path field.
- Process environment and CLI flags — the released entrypoint accepts only
  `--verbose`, and the sole `System.getenv("PATH")` read is in the unreachable
  fallback.

Instead:

1. Implement `language_server_workspace_configuration` correctly anyway. It costs
   nothing and starts working the moment 0.9.0 lands.
2. Advertise `actionableRuntimeNotifications` and `pklConfigureCommand` in
   `initializationOptions`. This redirects the nag from `window/showMessage` to
   the custom `pkl/actionableNotification`, which Zed has no handler for and
   drops. The spurious popup stops; nothing else changes.
3. Support `lsp.pkl.binary.path` / `.arguments` so users can run a build from
   pkl-lsp `main` and get full functionality today.

## Consequences

Until pkl-lsp 0.9.0, users on the bundled jar lose `PklProject` dependency
resolution, `package://` downloads, and the download-package code action, because
all of them route through `PklCli`, which stays unavailable. Parsing, hover,
go-to-definition, completion, formatting, and diagnostics are unaffected.

Item 2 is a workaround for a bug and should be reconsidered when 0.9.0 ships —
the capabilities are truthful in themselves (Zed genuinely ignores those
messages harmlessly), but the reason for setting them disappears.
