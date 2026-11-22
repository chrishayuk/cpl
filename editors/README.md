# CPL in your editor

Two pieces give an editor an understanding of CPL:

- **`cpl-lsp`**, a language server in [`../lsp`](../lsp). It works with any editor that speaks the Language Server Protocol.
- **A Visual Studio Code extension** in [`vscode`](vscode), which bundles the server and adds syntax highlighting and a run button. Its [README](vscode/README.md) lists the features.

Both are built on [`src/analysis.rs`](../src/analysis.rs), the same code behind `cpl --check`. It asks the system's own lexer, parser and compiler, so the editor and the system cannot disagree about what is a mistake.

## The language server

```sh
cargo install --git https://github.com/chrishayuk/cpl cpl-lsp
```

or, in a clone of this repository, `cargo build --release -p cpl-lsp`.

It talks on standard input and output and takes no arguments. It provides diagnostics, hover, go to definition, references, document highlights, rename, completion, signature help, document symbols, folding ranges and semantic tokens. Semantic tokens mean an editor with no CPL grammar still gets highlighting.

### Neovim (0.11 or later)

```lua
vim.filetype.add({ extension = { cpl = "cpl" } })
vim.lsp.config("cpl", {
  cmd = { "/path/to/cpl-lsp" },
  filetypes = { "cpl" },
})
vim.lsp.enable("cpl")
```

### Helix

In `languages.toml`:

```toml
[language-server.cpl-lsp]
command = "/path/to/cpl-lsp"

[[language]]
name = "cpl"
scope = "source.cpl"
file-types = ["cpl"]
comment-token = "||"
language-servers = ["cpl-lsp"]
```

These two configurations follow each editor's documented format but have not been run here; the server itself is tested over standard input and output by `vscode/test/lsp.test.mjs`.

## Checking a program from the command line

```sh
cpl --check program.cpl
```

prints one line per problem, as `file:line:column: severity: message`, and exits with status 1 if any is an error. `-` in place of a file reads the program from standard input.
