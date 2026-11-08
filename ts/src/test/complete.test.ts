import assert from "node:assert/strict";
import { test } from "node:test";
import { complete } from "../complete.js";

const names = ["abs", "avg", "max", "mean", "median", "min", "mode", "sqrt", "stddev"];

test("completes the word at the end of the line", () => {
  assert.deepEqual(complete("1 + me", names), { start: 4, candidates: ["mean", "median"] });
  assert.deepEqual(complete("s", names).candidates, ["sqrt", "stddev"]);
});

test("matching ignores case and returns sorted unique names", () => {
  assert.deepEqual(complete("MA", [...names, "max"]).candidates, ["max"]);
  assert.deepEqual(complete("m", ["min", "max", "min"]).candidates, ["max", "min"]);
});

test("an exact match is not offered again", () => {
  assert.deepEqual(complete("max", names).candidates, []);
});

test("no word at the end means nothing to complete", () => {
  assert.deepEqual(complete("1 + ", names), { start: 4, candidates: [] });
  assert.deepEqual(complete("max(1,", names).candidates, []);
  assert.deepEqual(complete("", names), { start: 0, candidates: [] });
});

test("digits inside identifiers are part of the word", () => {
  assert.deepEqual(complete("x1 + y_", ["y_total", "x1"]), { start: 5, candidates: ["y_total"] });
});
