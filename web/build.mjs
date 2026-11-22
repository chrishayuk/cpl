// Builds the files the playground needs:  node web/build.mjs
//   web/cpl.wasm       the CPL system, compiled for WASI
//   web/cpl-lsp.wasm   the language server, compiled for WASI
//   web/examples.json  the example programs, with their input
import { execSync } from "node:child_process";
import { copyFileSync, existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";

const root = new URL("..", import.meta.url).pathname;
execSync("cargo build --release --target wasm32-wasip1 -p cpl -p cpl-lsp", { cwd: root, stdio: "inherit" });
copyFileSync(`${root}target/wasm32-wasip1/release/cpl.wasm`, `${root}web/cpl.wasm`);
copyFileSync(`${root}target/wasm32-wasip1/release/cpl-lsp.wasm`, `${root}web/cpl-lsp.wasm`);

// Shown first, in this order; the rest follow alphabetically.
const first = ["hello", "factorial", "manual-hcf", "fibonacci", "lvalues", "fixed-and-free", "lh-functions",
  "general", "labels", "lists", "product", "streams", "queens", "mandelbrot", "calculator"];

const names = readdirSync(`${root}examples`).filter((f) => f.endsWith(".cpl")).map((f) => f.slice(0, -4));
names.sort((a, b) => {
  const [i, j] = [first.indexOf(a), first.indexOf(b)];
  if (i !== -1 || j !== -1) return (i === -1 ? 99 : i) - (j === -1 ? 99 : j);
  return a.localeCompare(b);
});
const examples = names.map((name) => {
  const input = `${root}examples/${name}.in`;
  return {
    name,
    program: readFileSync(`${root}examples/${name}.cpl`, "utf8"),
    input: existsSync(input) ? readFileSync(input, "utf8") : "",
  };
});
// The preprocessor of Appendix 1, with a program for it to read.
examples.splice(first.length, 0, {
  name: "appendix-1-preprocessor",
  program: readFileSync(`${root}appendix1/preprocessor.cpl`, "utf8"),
  input: readFileSync(`${root}appendix1/sample.pub`, "utf8"),
});
writeFileSync(`${root}web/examples.json`, JSON.stringify(examples));
console.log(`web/cpl.wasm, web/cpl-lsp.wasm and web/examples.json (${examples.length} programs) written`);
