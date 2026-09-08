# zed-pkl

A Zed extension providing Pkl language support. Two independent halves:

## Grammar and queries

`languages/pkl/` holds the Zed language configuration and Tree-sitter queries.
The grammar itself is `apple/tree-sitter-pkl`, pinned by commit in
`extension.toml`; Zed clones and builds it.

`highlights.scm` and `injections.scm` originate upstream but are adapted —
upstream targets Neovim capture names (`@escape`, `@function.method.builtin`,
`@variable.builtin`), Zed uses its own (`@string.escape`, `@function`,
`@variable.special`). Re-syncing means translating, not copying. The additions
Zed needs but upstream lacks are grouped under a `; Zed additions` heading.

## Language server

`src/lib.rs` compiles to WebAssembly and runs in Zed's extension host. It has
one job: produce the command Zed uses to spawn `pkl-lsp`, plus the LSP
initialization options and workspace configuration.

See [components/01-00-00-language-server-installation.md](components/01-00-00-language-server-installation.md)
for how the jar is fetched and validated.

Zed calls, in order:

| Export                                     | Purpose                                             |
| ------------------------------------------ | --------------------------------------------------- |
| `language_server_command`                  | `java -jar <pkl-lsp.jar>`, or the user's override    |
| `language_server_initialization_options`   | Extended client capabilities sent at `initialize`    |
| `language_server_workspace_configuration`  | Answers `workspace/configuration` requests           |

All three honour user settings under the `pkl` language server key. Defaults are
computed from the worktree (`which("java")`, `which("pkl")`) and merged under any
user-supplied object, so an explicit setting always wins.

## Constraints worth knowing

- The extension runs under WASI with the extension work directory preopened as
  `.`. Relative paths resolve there. `std::path::absolute` works because Zed sets
  `PWD` to the real host path.
- Running a subprocess requires a `process:exec` capability in `extension.toml`.
  The declared one permits any command with exactly `-version`, which is all the
  Java probe needs.
- Zed does **not** apply `lsp.<id>.binary` to extension-provided language servers.
  `LanguageServerBinaryOptions` is discarded in
  `crates/language_extension/src/extension_lsp_adapter.rs`. The extension must read
  those settings itself.
