# capl-ls

Experimental language server for Vector CAPL.

The first goal is useful offline editor intelligence for proprietary CAPL code without requiring CANoe/CANalyzer at development time. This server should parse and index user-owned `.can` and `.cin` files, then provide navigation and structural editing features that do not depend on proprietary compiler access.

## Scope

Planned early features:

- Document symbols for `includes`, `variables`, functions, `testcase` blocks, event handlers, types, and macros.
- Go to definition for user functions, globals, types, macros, and include files.
- Hover for macro definitions and known local symbols.
- A conservative preprocessor layer for `#include`, object-like macros, and simple function-like macros.
- Parser fixtures based on original examples and permissively licensed learning material.

Explicit non-goals for the first version:

- No bundled Vector documentation or scraped proprietary API database.
- No claim of CANoe compiler compatibility.
- No dependency on a licensed Vector installation for core language-server features.

## Development

Install a Rust toolchain, then run:

```sh
cargo test
cargo run
```

The binary speaks LSP over stdio.

## References

- CAPL tips, CC0-1.0: https://github.com/comstantin/CAPL-tips
- Language Server Protocol: https://microsoft.github.io/language-server-protocol/

