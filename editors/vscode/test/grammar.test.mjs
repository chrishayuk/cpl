// Tokenises CPL with the same TextMate engine VS Code uses and checks the
// scopes the grammar assigns. What counts as a word, a name or a constant
// is decided by src/lexer.rs, so the lists there are read and compared.
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import oniguruma from "vscode-oniguruma";
import textmate from "vscode-textmate";

const require = createRequire(import.meta.url);
const grammarPath = new URL("../syntaxes/cpl.tmLanguage.json", import.meta.url).pathname;
const root = new URL("../../../", import.meta.url).pathname;

const wasm = readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
const registry = new textmate.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
    createOnigString: (text) => new oniguruma.OnigString(text),
  }),
  loadGrammar: async () => textmate.parseRawGrammar(readFileSync(grammarPath, "utf8"), grammarPath),
});
const grammar = await registry.loadGrammar("source.cpl");

/** Each non-blank piece of text with its most specific scope. */
function scopes(source) {
  const out = [];
  let state = textmate.INITIAL;
  for (const line of source.split("\n")) {
    const result = grammar.tokenizeLine(line, state);
    for (const token of result.tokens) {
      const text = line.slice(token.startIndex, token.endIndex);
      if (text.trim()) out.push([text, token.scopes.at(-1)]);
    }
    state = result.ruleStack;
  }
  return out;
}

test("reserved words are lower case, and small letters beside them are names", () => {
  assert.deepEqual(scopes("let rec f[x] = ax + letx"), [
    ["let", "storage.type.cpl"],
    ["rec", "storage.modifier.cpl"],
    ["f", "variable.other.small.cpl"],
    ["[", "punctuation.section.brackets.begin.cpl"],
    ["x", "variable.other.small.cpl"],
    ["]", "punctuation.section.brackets.end.cpl"],
    ["=", "keyword.operator.comparison.cpl"],
    // "ax" is a times x, and "letx" is not a word, so it is four names.
    ["a", "variable.other.small.cpl"],
    ["x", "variable.other.small.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["l", "variable.other.small.cpl"],
    ["e", "variable.other.small.cpl"],
    ["t", "variable.other.small.cpl"],
    ["x", "variable.other.small.cpl"],
  ]);
});

test("two words that make one symbol", () => {
  assert.deepEqual(scopes("x := value of $( result is 1 $)\nif x = 1 go to L\nx := x + 1 repeat until x > 9\nlet a, b both be long logical"), [
    ["x", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["value of", "keyword.control.result.cpl"],
    ["$(", "punctuation.section.block.begin.cpl"],
    ["result is", "keyword.control.result.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
    ["$)", "punctuation.section.block.end.cpl"],
    ["if", "keyword.control.conditional.cpl"],
    ["x", "variable.other.small.cpl"],
    ["=", "keyword.operator.comparison.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
    ["go to", "keyword.control.jump.cpl"],
    ["L", "variable.other.large.cpl"],
    ["x", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["x", "variable.other.small.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
    ["repeat until", "keyword.control.loop.cpl"],
    ["x", "variable.other.small.cpl"],
    [">", "keyword.operator.comparison.cpl"],
    ["9", "constant.numeric.decimal.cpl"],
    ["let", "storage.type.cpl"],
    ["a", "variable.other.small.cpl"],
    [",", "punctuation.separator.comma.cpl"],
    ["b", "variable.other.small.cpl"],
    ["both be", "keyword.other.definition.cpl"],
    ["long logical", "support.type.primitive.cpl"],
  ]);
});

test("large names, primes, basic functions and labels", () => {
  assert.deepEqual(scopes("Next: Total' := Sqrt[a'] + Digit.Dot + Fact2[n] + NIL"), [
    ["Next", "entity.name.label.cpl"],
    [":", "punctuation.separator.label.cpl"],
    ["Total'", "variable.other.large.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["Sqrt", "support.function.cpl"],
    ["[", "punctuation.section.brackets.begin.cpl"],
    // A prime after a name is part of the name, not the start of a string.
    ["a'", "variable.other.small.cpl"],
    ["]", "punctuation.section.brackets.end.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["Digit.Dot", "variable.other.large.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["Fact2", "entity.name.function.cpl"],
    ["[", "punctuation.section.brackets.begin.cpl"],
    ["n", "variable.other.small.cpl"],
    ["]", "punctuation.section.brackets.end.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["NIL", "constant.language.nil.cpl"],
  ]);
  // "Boolean" is the one basic symbol with a capital; "Booleans" is a name.
  assert.deepEqual(scopes("let b be Boolean; Booleans := Sqrtx"), [
    ["let", "storage.type.cpl"],
    ["b", "variable.other.small.cpl"],
    ["be", "keyword.other.cpl"],
    ["Boolean", "support.type.primitive.cpl"],
    [";", "punctuation.terminator.cpl"],
    ["Booleans", "variable.other.large.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["Sqrtx", "variable.other.large.cpl"],
  ]);
});

test("tagged section brackets", () => {
  assert.deepEqual(scopes("$(T $(1.2 { x := 1 } $)1.2 $)T"), [
    ["$(", "punctuation.section.block.begin.cpl"],
    ["T", "entity.name.tag.section.cpl"],
    ["$(", "punctuation.section.block.begin.cpl"],
    ["1.2", "entity.name.tag.section.cpl"],
    ["{", "punctuation.section.block.begin.cpl"],
    ["x", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
    ["}", "punctuation.section.block.end.cpl"],
    ["$)", "punctuation.section.block.end.cpl"],
    ["1.2", "entity.name.tag.section.cpl"],
    ["$)", "punctuation.section.block.end.cpl"],
    ["T", "entity.name.tag.section.cpl"],
  ]);
});

test("comments, strings and their escapes", () => {
  assert.deepEqual(scopes("Write['it*'s || here*n', '*q'] || a comment 'not a string'"), [
    ["Write", "support.function.cpl"],
    ["[", "punctuation.section.brackets.begin.cpl"],
    ["'", "punctuation.definition.string.begin.cpl"],
    ["it", "string.quoted.single.cpl"],
    ["*'", "constant.character.escape.cpl"],
    ["s || here", "string.quoted.single.cpl"],
    ["*n", "constant.character.escape.cpl"],
    ["'", "punctuation.definition.string.end.cpl"],
    [",", "punctuation.separator.comma.cpl"],
    ["'", "punctuation.definition.string.begin.cpl"],
    ["*q", "invalid.illegal.escape.cpl"],
    ["'", "punctuation.definition.string.end.cpl"],
    ["]", "punctuation.section.brackets.end.cpl"],
    ["||", "punctuation.definition.comment.cpl"],
    [" a comment 'not a string'", "comment.line.double-bar.cpl"],
  ]);
  // A string does not run on to the next line.
  assert.deepEqual(scopes("Write['open\nx := 1").slice(-3), [
    ["x", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
  ]);
});

test("numbers and logical constants, barred or not", () => {
  assert.deepEqual(scopes("p := #o|273 \\/ #b1010 /\\ ~#x1FF; x := 2.5@-8 + .5 + 12"), [
    ["p", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["#o", "storage.type.radix.cpl"],
    ["|", "keyword.operator.bar.cpl"],
    ["273", "constant.numeric.octal.cpl"],
    ["\\/", "keyword.operator.logical.cpl"],
    ["#b", "storage.type.radix.cpl"],
    ["1010", "constant.numeric.binary.cpl"],
    ["/\\", "keyword.operator.logical.cpl"],
    ["~", "keyword.operator.logical.cpl"],
    ["#x", "storage.type.radix.cpl"],
    ["1FF", "constant.numeric.hex.cpl"],
    [";", "punctuation.terminator.cpl"],
    ["x", "variable.other.small.cpl"],
    [":=", "keyword.operator.assignment.cpl"],
    ["2.5@-8", "constant.numeric.decimal.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    [".5", "constant.numeric.decimal.cpl"],
    ["+", "keyword.operator.arithmetic.cpl"],
    ["12", "constant.numeric.decimal.cpl"],
  ]);
});

test("operators, longest first", () => {
  const signs = "<=> ~== := -> <= >= << >> ~= ~~ == ** /\\ \\/ \\= + - * / = < > ~ ^ , ; . . .";
  const found = scopes(signs);
  assert.deepEqual(found.map(([text]) => text).join(" "), signs);
  for (const [text, scope] of found) assert.match(scope, /^(keyword\.operator|punctuation)\./, text);
});

test("publication CPL: underlined words and section signs", () => {
  const underlined = (word) => [...word].map((c) => c + "̲").join("");
  const source = `${underlined("let")} x = 1\n§1 ${underlined("if")} x ≠ 2 ${underlined("go")} ${underlined("to")} L §̸1`;
  assert.deepEqual(scopes(source), [
    [underlined("let"), "keyword.other.underlined.cpl"],
    ["x", "variable.other.small.cpl"],
    ["=", "keyword.operator.comparison.cpl"],
    ["1", "constant.numeric.decimal.cpl"],
    ["§", "punctuation.section.block.begin.cpl"],
    ["1", "entity.name.tag.section.cpl"],
    [underlined("if"), "keyword.other.underlined.cpl"],
    ["x", "variable.other.small.cpl"],
    ["≠", "keyword.operator.comparison.cpl"],
    ["2", "constant.numeric.decimal.cpl"],
    [underlined("go"), "keyword.other.underlined.cpl"],
    [underlined("to"), "keyword.other.underlined.cpl"],
    ["L", "variable.other.large.cpl"],
    ["§̸", "punctuation.section.block.end.cpl"],
    ["1", "entity.name.tag.section.cpl"],
  ]);
});

test("every basic symbol of the lexer is a word here, and every basic function is known", () => {
  const lexer = readFileSync(root + "src/lexer.rs", "utf8");
  const words = /pub const WORDS: &\[&str\] = &\[([^\]]*)\]/.exec(lexer)[1].match(/[a-z]+/g);
  assert.ok(words.length >= 90);
  for (const word of words) {
    const [[text, scope]] = scopes(word);
    assert.equal(text, word);
    assert.doesNotMatch(scope, /^variable\.other/, word);
  }
  const table = readFileSync(root + "src/builtins.rs", "utf8").split("pub static TABLE")[1];
  const names = [...table.matchAll(/\(\s*"([A-Za-z0-9]+)",\s*BDef/g)].map((m) => m[1]);
  assert.ok(names.length >= 75);
  for (const name of names) {
    const [[text, scope]] = scopes(name);
    assert.equal(text, name);
    assert.match(scope, /^(support\.function|constant\.language\.nil|support\.type\.primitive)\./, name);
  }
});

test("every program tokenises with no text left unscoped", () => {
  const files = [
    ...readdirSync(root + "examples").filter((f) => f.endsWith(".cpl")).map((f) => `examples/${f}`),
    ...readdirSync(root + "appendix1").filter((f) => f.endsWith(".cpl") || f.endsWith(".pub")).map((f) => `appendix1/${f}`),
  ];
  assert.ok(files.length >= 70);
  for (const file of files) {
    for (const [text, scope] of scopes(readFileSync(root + file, "utf8"))) {
      assert.notEqual(scope, "source.cpl", `${file}: unscoped "${text}"`);
      assert.doesNotMatch(scope, /^invalid\./, `${file}: "${text}"`);
    }
  }
});
