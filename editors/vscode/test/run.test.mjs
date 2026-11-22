// Runs every example through the runner the "Run Program" command uses, and
// compares what it prints with the expected output.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const runner = new URL("../server/run.js", import.meta.url).pathname;
const examples = new URL("../../../examples/", import.meta.url).pathname;

// The .out files record an error as a last line beginning "! ", after a
// line break if the output did not end with one.
function transcript(result) {
  const out = result.stdout.toString();
  const err = result.stderr.toString();
  if (!err) return out;
  const message = err.trimEnd().replace(/^program: /, "");
  return out + (out && !out.endsWith("\n") ? "\n" : "") + `! ${message}\n`;
}

test("the built-in system runs every example", () => {
  const names = readdirSync(examples).filter((f) => f.endsWith(".cpl"));
  assert.ok(names.length >= 60);
  for (const file of names) {
    const base = examples + file.slice(0, -4);
    const input = existsSync(`${base}.in`) ? readFileSync(`${base}.in`) : "";
    const result = spawnSync(process.execPath, [runner, examples + file], { input });
    assert.equal(transcript(result), readFileSync(`${base}.out`, "utf8"), file);
    assert.equal(result.status, result.stderr.length ? 1 : 0, file);
  }
});

test("a program in publication CPL runs too", () => {
  const underlined = (word) => [...word].map((c) => c + "\u0332").join("");
  const file = join(mkdtempSync(join(tmpdir(), "cpl-")), "printed.pub");
  writeFileSync(file, `${underlined("let")} x = 2\n\u00a71 ${underlined("if")} x \u2260 1 ${underlined("do")} Write['yes'] \u00a7\u03381\n`);
  const result = spawnSync(process.execPath, [runner, file]);
  assert.equal(result.status, 0, result.stderr.toString());
  assert.equal(result.stdout.toString(), "yes");
});

test("an error is reported and the exit code is not zero", () => {
  const file = join(mkdtempSync(join(tmpdir(), "cpl-")), "broken.cpl");
  writeFileSync(file, "Write['before ']\nlet x be integer\nx := 1/3\n");
  const result = spawnSync(process.execPath, [runner, file]);
  assert.equal(result.status, 1);
  assert.equal(result.stdout.toString(), "before ");
  assert.match(result.stderr.toString(), /line 3: alarm: the real number 0\.333333333333 is not integral/);
});

test("a program reads its input from standard input", () => {
  const file = join(mkdtempSync(join(tmpdir(), "cpl-")), "double.cpl");
  writeFileSync(file, "let n = 0\nRead[n]\nWrite[2n]\n");
  const result = spawnSync(process.execPath, [runner, file], { input: "21\n" });
  assert.equal(result.stdout.toString(), "42.0");
});

test("a file that is not there is reported", () => {
  const result = spawnSync(process.execPath, [runner, "/no/such/program.cpl"]);
  assert.equal(result.status, 2);
  assert.match(result.stderr.toString(), /cannot read/);
});
