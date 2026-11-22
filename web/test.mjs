// Runs every example through the WebAssembly build and the browser's WASI
// host, and compares the output with the .out file. Then asks the language
// server, through the code the page's editor uses, about the same examples.
//   node web/test.mjs
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { answer } from "./lsp-worker.js";
import { argsFor, run, stdinFor } from "./wasi.js";

const root = new URL("..", import.meta.url).pathname;
const module = new WebAssembly.Module(readFileSync(root + "web/cpl.wasm"));
let failed = 0;

function cpl(program, input = "", mode = "run") {
  const text = { 1: "", 2: "" };
  const decoders = { 1: new TextDecoder(), 2: new TextDecoder() };
  const code = run(module, argsFor(program, mode), stdinFor(input), (fd, bytes) => {
    text[fd] += decoders[fd].decode(bytes, { stream: true });
  });
  return { code, out: text[1], err: text[2] };
}

function check(what, ok, detail = "") {
  if (ok) return;
  failed++;
  console.log(`FAIL ${what} ${detail}`);
}

// The .out files record an error as a last line beginning "! ", after a
// line break if the output did not end with one.
function transcript({ out, err }) {
  if (!err) return out;
  const message = err.trimEnd().replace(/^program: /, "");
  return out + (out && !out.endsWith("\n") ? "\n" : "") + `! ${message}\n`;
}

let checked = 0;
for (const file of readdirSync(root + "examples").filter((f) => f.endsWith(".cpl")).sort()) {
  const name = file.slice(0, -4);
  const program = readFileSync(`${root}examples/${file}`, "utf8");
  const inFile = `${root}examples/${name}.in`;
  const input = existsSync(inFile) ? readFileSync(inFile, "utf8") : "";
  const expected = readFileSync(`${root}examples/${name}.out`, "utf8");
  const result = cpl(program, input);
  checked++;
  check(name, transcript(result) === expected && result.code === (result.err ? 1 : 0), `(exit ${result.code})`);
}

// The examples the page puts first must exist.
const shown = JSON.parse(readFileSync(root + "web/examples.json", "utf8"));
check("examples.json", shown.length === checked + 1, `has ${shown.length} programs, examples/ has ${checked} and appendix1/ one`);

// An error must come back as a message and a non-zero exit code.
let r = cpl("let x = 1\nx := 'one'");
check("run-time error", r.code === 1 && r.err.includes("line 2"), JSON.stringify(r));
r = cpl("let x = ");
check("syntax error", r.code === 1 && r.err.includes("line 1"), JSON.stringify(r));

// Recursion is not carried on the engine's stack: this is 50,000 deep.
r = cpl("let rec Sum[integer n] = (n = 0 -> 0, n + Sum[n - 1])\nWrite[Sum[50000]]");
check("deep recursion", r.code === 0 && r.out === "1250025000", JSON.stringify(r));
r = cpl("let rec Forever[integer n] = Forever[n + 1] + 1\nWrite[Forever[1]]");
check("runaway recursion", r.code === 1 && r.err.includes("runaway recursion"), JSON.stringify(r));

// A list of sixty thousand members is made, measured and freed.
r = cpl("let rec Upto[integer n, general l] = (n = 0 -> l, Upto[n - 1, Cons[n, l]])\nWrite[Length[Upto[60000, NIL]]]");
check("long list", r.code === 0 && r.out === "60000", JSON.stringify(r));

// The other things the page can show.
r = cpl("let x = 2\nWrite[x x]", "", "code");
check("machine code", r.code === 0 && r.out.includes("LoadLocal"), JSON.stringify(r));
r = cpl("let x = 2", "", "tree");
check("syntax tree", r.code === 0 && r.out.includes("Simple"), JSON.stringify(r));

r = cpl("let x = 2\nif x > 1 do Write['yes']", "", "printed");
check("publication form", r.code === 0 && r.out.startsWith("l\u0332e\u0332t\u0332 x = 2"), JSON.stringify(r));
// ... which is itself a program.
r = cpl(r.out);
check("publication form runs", r.code === 0 && r.out === "yes", JSON.stringify(r));

// Appendix 1: the preprocessor of 1966, reading a program in the printed alphabet.
const pre = readFileSync(root + "appendix1/preprocessor.cpl", "utf8");
r = cpl(pre, readFileSync(root + "appendix1/sample.pub", "utf8"));
check("appendix 1", r.code === 0 && r.out === readFileSync(root + "appendix1/sample.out", "utf8"), JSON.stringify(r).slice(0, 300));

// The language server, as the editor in the page asks it (lsp-worker.js).
const server = new WebAssembly.Module(readFileSync(root + "web/cpl-lsp.wasm"));
const at = (line, character) => ({ position: { line, character } });
const hcf = readFileSync(root + "examples/manual-hcf.cpl", "utf8");
let a = answer(server, hcf, [
  { method: "textDocument/semanticTokens/full" },
  { method: "textDocument/hover", params: at(7, 28) },   // HCF, where it is called
  { method: "textDocument/hover", params: at(2, 5) },    // rec
  { method: "textDocument/hover", params: at(5, 16) },   // Rem
  { method: "textDocument/completion", params: at(7, 0) },
  { method: "textDocument/definition", params: at(7, 28) },
  { method: "textDocument/noSuchThing" },
]);
check("server: a good program has no problems", a.diagnostics.length === 0, JSON.stringify(a.diagnostics));
const data = a.results[0]?.data ?? [];
// "||" comment first; then "let" (keyword, type 7), "rec", and "HCF" being defined (function, type 4, modifier 1).
check("server: semantic tokens", data.length % 5 === 0 && data.length >= 200
  && data.slice(0, 5).join() === "0,0,70,0,0" && data.slice(5, 10).join() === "2,0,3,7,0" && data.slice(15, 20).join() === "0,4,3,4,1",
  JSON.stringify(data.slice(0, 20)));
check("server: hover on a function", /Function `HCF`/.test(a.results[1]?.contents.value) && /let rec HCF\[integer n, m\] =/.test(a.results[1].contents.value),
  JSON.stringify(a.results[1]));
check("server: hover on a basic symbol", /^\*\*`rec`\*\* recursive definition/.test(a.results[2]?.contents.value), JSON.stringify(a.results[2]));
check("server: hover on a basic function", /^\*\*`Rem\[x, y\]`\*\* basic function/.test(a.results[3]?.contents.value), JSON.stringify(a.results[3]));
const labels = (a.results[4] ?? []).map((item) => item.label);
check("server: completion", labels[0] === "HCF" && ["Sqrt", "Write", "let", "value of", "routine"].every((l) => labels.includes(l)), labels.slice(0, 5).join());
check("server: definition", a.results[5]?.range.start.line === 2 && a.results[5].range.start.character === 8, JSON.stringify(a.results[5]));
check("server: a request it does not know is answered with nothing", a.results[6] === null, JSON.stringify(a.results[6]));

// A mistake is found where it is, with the system's own message.
a = answer(server, "let count = 0\n", []);
check("server: a mistake of syntax", a.diagnostics.length === 1 && a.diagnostics[0].severity === 1
  && a.diagnostics[0].message === "'=', '~~', '==' or 'be' expected in a definition (found the name o)"
  && a.diagnostics[0].range.start.line === 0 && a.diagnostics[0].range.start.character === 5 && a.diagnostics[0].range.end.character === 6,
  JSON.stringify(a.diagnostics));
a = answer(server, "let Total = 0\nlet routine Add[x] be\n$(A Total := Total + x\n", [{ method: "textDocument/completion", params: at(2, 22) }]);
check("server: a bracket left open", a.diagnostics.map((d) => d.code).join() === "unclosed-bracket"
  && a.diagnostics[0].message === "the section bracket tagged A is never closed", JSON.stringify(a.diagnostics));
// Total and the parameter x are in scope there. Add is not: it is not recursive.
const offered = a.results[0].map((item) => item.label);
check("server: and what is defined is still known", offered[0] === "Total" && offered[1] === "x" && !offered.includes("Add"), offered.slice(0, 4).join());

// Publication CPL: positions count UTF-16 units, as the editor in the page does.
a = answer(server, "l\u0332e\u0332t\u0332 x = 1\n\u00a71 Write[x] \u00a7\u03381\n", [{ method: "textDocument/hover", params: at(1, 12) }]);
check("server: publication CPL", a.diagnostics.length === 0 && /Closing section bracket, tagged `1`/.test(a.results[0]?.contents.value), JSON.stringify(a));

// The server's verdict on every example agrees with running it: the one
// example it faults is the one that the compiler refuses.
let faulted = [];
for (const example of shown) {
  const found = answer(server, example.program, []).diagnostics.filter((d) => d.severity === 1);
  if (found.length) faulted.push(example.name);
}
check("server: every example", faulted.join() === "fixed-and-free", faulted.join());

console.log(`${checked} examples checked through WebAssembly, ${failed} checks failed`);
process.exit(failed ? 1 : 0);
