; Synced from https://raw.githubusercontent.com/apple/tree-sitter-pkl/main/queries/injections.scm (Apache-2.0)

; this definition is imprecise in that
; * any qualified or unqualified call to a method named "Regex" is considered a regex
; * string delimiters are considered part of the regex
(
  ((unqualifiedAccessExpr (identifier) @_methodName (argumentList (slStringLiteralExpr) @injection.content))
    (#set! injection.language "regex"))
  (#eq? @_methodName "Regex"))

; TODO: inject markdown into doc comments
