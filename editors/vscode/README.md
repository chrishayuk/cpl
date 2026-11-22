# CPL for Visual Studio Code

Editor support for [CPL](https://github.com/chrishayuk/cpl), the Combined Programming Language designed at Cambridge and London between 1962 and 1966, and the ancestor of BCPL, B and C. In CPL a small letter is a whole name, `ax` is a product, and a word in small letters may be a name or a basic symbol, so an editor that can tell them apart helps a great deal.

Nothing else needs installing: the language server and the CPL system are built into the extension as WebAssembly.

## What you get

- **Syntax highlighting** for `.cpl` files and for publication CPL in `.pub` files: basic symbols (`let`, `value of`, `go to`, or underlined words), small names and large names, tagged section brackets (`$(1.2 ... $)1.2`, `§` and its struck-through twin), `||` comments, strings with their star escapes, numbers, and logical constants such as `#o777` and `#o|273`. Functions, parameters, labels and the basic functions are told apart once the language server has read the program.
- **Hover** on a basic symbol to see what it does, with its other spellings; on a basic function such as `Rem` to see its arguments; on any other name to see its definition, the comment above it and how often it is used.
- **Problems as you type:** every mistake the system itself would report when asked to run the program, placed on the symbol that is wrong. A string with no closing quote, a section bracket that is never closed, a tag that matches nothing, a mistake of syntax, a name that is not defined, an assignment that a fixed function may not make. A name that is defined and never used is faded.
- **Go to definition** and **find all references** for variables, functions, routines, parameters, labels and names defined by `where`, following the scope rules: `rec`, `and`, `in`, `where`, blocks and parameters.
- **Rename** a name everywhere it is in scope. A space is put in where the new name would otherwise run into its neighbours.
- **Completion:** the names in scope, the basic functions, the basic symbols, and templates for a function, a routine, a cycle and a switch.
- **Signature help** inside the square brackets of a call, an **outline** of the definitions and labels, and **folding** for sections, functions and comment blocks.
- **Run Program:** the play button at the top of the editor, or Cmd+Enter (Ctrl+Enter on Windows and Linux). The program runs in a terminal, so `Read` and `Readchar` can take what you type.

Files ending in `.pub` are treated as publication CPL. If you keep other files with that ending, such as SSH public keys, set their language yourself or add a `files.associations` entry.

## Settings

| Setting | What it does |
| --- | --- |
| `cpl.server.path` | Use a `cpl-lsp` program you built yourself as the language server. |
| `cpl.interpreter.path` | Run programs with a `cpl` you built yourself. |
| `cpl.trace.server` | Log the messages exchanged with the language server. |

## Building it

From this folder, with Rust (and its `wasm32-wasip1` target) and Node installed:

```sh
npm install
npm run build        # compiles the server and the system to WebAssembly, bundles the extension
npm test             # grammar, language server and runner tests
npm run test:vscode  # opens a real VS Code and drives the extension
npm run package      # writes cpl-language-0.1.0.vsix
code --install-extension cpl-language-0.1.0.vsix
```

To try it without installing, open this folder in VS Code and press F5.

## How it fits together

| File | What it is |
| --- | --- |
| `syntaxes/cpl.tmLanguage.json` | The highlighting grammar. |
| `language-configuration.json` | Comment characters, brackets and auto-closing pairs. |
| `src/extension.js` | Starts the language server and provides Run Program. |
| `server/lsp.js`, `server/cpl-lsp.wasm` | The language server from `../../lsp`, compiled for WASI. |
| `server/run.js`, `server/cpl.wasm` | The CPL system, compiled for WASI. |
| `server/wasi.js` | A small WASI host for Node, with no dependencies. |
