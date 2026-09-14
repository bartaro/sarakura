# SARAKURA v0.9.1 baseline diagnostics

`baseline-delta` compares diagnostic sets against a baseline. In
`sarakura-core/src/baseline.rs`, `diagnostic_map()` builds a
`BTreeMap<String, &AiDiagnostic>`: each entry borrows its diagnostic rather than
owning a copy. Candidate selection dereferences the stored representative
when comparing diagnostics.
