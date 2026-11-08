import assert from "node:assert/strict";
import { test } from "node:test";
import { CalcError } from "../client.js";
import { History } from "../history.js";
import { DEFAULT_SETTINGS, Repl } from "../repl.js";
import { FakeEvaluator } from "./helpers.js";

function make(table: Record<string, string | CalcError> = {}) {
  const evaluator = new FakeEvaluator(table);
  const repl = new Repl(evaluator, new History(), { ...DEFAULT_SETTINGS });
  return { evaluator, repl };
}

test("blank lines produce no output", async () => {
  const { repl, evaluator } = make();
  assert.deepEqual(await repl.handleLine("   "), { text: "" });
  assert.equal(evaluator.calls.length, 0);
});

test("evaluates with the current settings and records history", async () => {
  const { repl, evaluator } = make({ "1/3": "0.3333" });
  repl.settings.places = 4;
  const out = await repl.handleLine("1/3", () => 123);
  assert.equal(out.text, "0.3333");
  assert.deepEqual(evaluator.calls[0], { expr: "1/3", options: { places: 4, mode: "half-up" } });
  assert.deepEqual(repl.history.last(), { expr: "1/3", result: "0.3333", at: 123 });
});

test("results are formatted with grouping when enabled", async () => {
  const { repl } = make({ "1000*1000": "1000000" });
  await repl.handleLine(":thousands on");
  assert.equal((await repl.handleLine("1000*1000")).text, "1,000,000");
  assert.equal(repl.history.last()?.result, "1,000,000");
});

test("errors show the code and point at the position", async () => {
  const { repl } = make({ "1 + * 2": new CalcError("SYNTAX", "unexpected '*' at 4", 4) });
  const out = await repl.handleLine("1 + * 2");
  assert.equal(out.error, true);
  assert.equal(out.text, "error[SYNTAX]: unexpected '*' at 4\n  1 + * 2\n      ^");
  assert.equal(repl.history.size, 0);
});

test("errors without a position have no caret", async () => {
  const { repl } = make({ "1/0": new CalcError("DIV_BY_ZERO", "division by zero") });
  assert.equal((await repl.handleLine("1/0")).text, "error[DIV_BY_ZERO]: division by zero");
});

test("settings commands", async () => {
  const { repl } = make();
  assert.equal((await repl.handleLine(":places 3")).text, "places = 3");
  assert.equal((await repl.handleLine(":mode half-even")).text, "mode = half-even");
  assert.equal((await repl.handleLine(":thousands on")).text, "thousands = on");
  assert.deepEqual(repl.settings, { places: 3, mode: "half-even", thousands: true });
  const bad = await repl.handleLine(":places 99");
  assert.equal(bad.error, true);
});

test(":clear resets the engine and :quit ends the session", async () => {
  const { repl, evaluator } = make();
  await repl.handleLine(":clear");
  assert.equal(evaluator.resets, 1);
  assert.equal((await repl.handleLine(":quit")).quit, true);
});

test(":history lists entries", async () => {
  const { repl } = make({ a: "1", b: "2" });
  assert.equal((await repl.handleLine(":history")).text, "(no history)");
  await repl.handleLine("a");
  await repl.handleLine("b");
  assert.equal((await repl.handleLine(":history")).text, "1  a = 1\n2  b = 2");
  assert.equal((await repl.handleLine(":history 1")).text, "1  b = 2");
});
