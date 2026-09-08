# Declare a narrow `process:exec` capability

## Context

Detecting a usable Java runtime requires actually running it. On macOS
`/usr/bin/java` always exists, so `which` proves nothing, and there is no way to
read a JDK's version without executing it.

Zed gates `process::run_command` behind a `process:exec` capability declared in
`extension.toml`. The capability matches on command and argument list; `*` is a
single wildcard and `**` matches any trailing arguments.

The command cannot be named exactly, because it is whatever `which("java")`
resolves to, which differs per machine and per toolchain manager.

## Decision

```toml
[[capabilities]]
kind = "process:exec"
command = "*"
args = ["-version"]
```

Wildcard the command, pin the arguments. Any binary may be run, but only with
exactly one argument, `-version`.

Rejected: `args = ["**"]`, which would permit arbitrary commands with arbitrary
arguments. Also rejected: `-XshowSettings:properties -version`, which parses more
cleanly but widens the argument allowance for no real gain — the quoted version
string in `java -version` output is unambiguous enough.

## Consequences

The version probe is one process spawn per language server start. If a future
change needs a different subprocess, this capability must widen, and that should
be a deliberate decision rather than an incidental one.
