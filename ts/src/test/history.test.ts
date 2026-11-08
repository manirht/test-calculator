import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { History, loadHistory, saveHistory } from "../history.js";

const entry = (expr: string, result = "0", at = 0) => ({ expr, result, at });

test("add keeps order and list returns copies", () => {
  const history = new History();
  history.add(entry("1+1", "2"));
  history.add(entry("2+2", "4"));
  assert.deepEqual(history.list().map((e) => e.expr), ["1+1", "2+2"]);
  history.list().pop();
  assert.equal(history.size, 2);
  assert.equal(history.last()?.expr, "2+2");
});

test("repeating the previous expression replaces it", () => {
  const history = new History();
  history.add(entry("x", "1", 1));
  history.add(entry("x", "2", 2));
  assert.equal(history.size, 1);
  assert.equal(history.last()?.result, "2");
  history.add(entry("y"));
  history.add(entry("x"));
  assert.equal(history.size, 3);
});

test("the oldest entries are dropped past the limit", () => {
  const history = new History(3);
  for (const expr of ["a", "b", "c", "d", "e"]) history.add(entry(expr));
  assert.deepEqual(history.list().map((e) => e.expr), ["c", "d", "e"]);
});

test("list(count) returns the most recent entries", () => {
  const history = new History();
  for (const expr of ["a", "b", "c", "d"]) history.add(entry(expr));
  assert.deepEqual(history.list(2).map((e) => e.expr), ["c", "d"]);
  assert.deepEqual(history.list(10).map((e) => e.expr), ["a", "b", "c", "d"]);
  assert.deepEqual(history.list(0), []);
});

test("search is case-insensitive", () => {
  const history = new History();
  history.add(entry("Sqrt(16)"));
  history.add(entry("1+2"));
  assert.deepEqual(history.search("sqrt").map((e) => e.expr), ["Sqrt(16)"]);
  assert.deepEqual(history.search("zzz"), []);
});

test("clear empties the history", () => {
  const history = new History();
  history.add(entry("a"));
  history.clear();
  assert.equal(history.size, 0);
  assert.equal(history.last(), undefined);
});

test("save and load round trip", () => {
  const dir = mkdtempSync(join(tmpdir(), "calc-history-"));
  const path = join(dir, "h.json");
  const history = new History();
  history.add(entry("1+1", "2", 5));
  saveHistory(path, history);
  const loaded = loadHistory(path);
  assert.deepEqual(loaded.list(), [{ expr: "1+1", result: "2", at: 5 }]);
});

test("missing or corrupt files give an empty history", () => {
  const dir = mkdtempSync(join(tmpdir(), "calc-history-"));
  assert.equal(loadHistory(join(dir, "missing.json")).size, 0);
  const bad = join(dir, "bad.json");
  writeFileSync(bad, "{not json");
  assert.equal(loadHistory(bad).size, 0);
  const odd = join(dir, "odd.json");
  writeFileSync(odd, JSON.stringify([{ expr: "ok", result: "1", at: 1 }, { expr: 5 }, "junk"]));
  assert.deepEqual(loadHistory(odd).list().map((e) => e.expr), ["ok"]);
});
