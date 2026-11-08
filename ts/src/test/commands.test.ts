import assert from "node:assert/strict";
import { test } from "node:test";
import { parseCommand } from "../commands.js";

test("non-commands return null", () => {
  assert.equal(parseCommand("1+2"), null);
  assert.equal(parseCommand(""), null);
});

test("simple commands", () => {
  assert.deepEqual(parseCommand(":help"), { kind: "help" });
  assert.deepEqual(parseCommand(" :q "), { kind: "quit" });
  assert.deepEqual(parseCommand(":exit"), { kind: "quit" });
  assert.deepEqual(parseCommand(":clear"), { kind: "clear" });
});

test("history takes an optional count", () => {
  assert.deepEqual(parseCommand(":history"), { kind: "history" });
  assert.deepEqual(parseCommand(":history 5"), { kind: "history", count: 5 });
  assert.equal(parseCommand(":history x")?.kind, "error");
});

test("places is validated", () => {
  assert.deepEqual(parseCommand(":places 4"), { kind: "places", value: 4 });
  assert.deepEqual(parseCommand(":places 0"), { kind: "places", value: 0 });
  assert.deepEqual(parseCommand(":places 30"), { kind: "places", value: 30 });
  for (const bad of [":places", ":places -1", ":places 31", ":places 2.5", ":places x"]) {
    assert.equal(parseCommand(bad)?.kind, "error", bad);
  }
});

test("mode accepts only known modes", () => {
  assert.deepEqual(parseCommand(":mode half-even"), { kind: "mode", value: "half-even" });
  assert.equal(parseCommand(":mode sideways")?.kind, "error");
  assert.equal(parseCommand(":mode")?.kind, "error");
});

test("thousands toggles", () => {
  assert.deepEqual(parseCommand(":thousands on"), { kind: "thousands", value: true });
  assert.deepEqual(parseCommand(":thousands off"), { kind: "thousands", value: false });
  assert.equal(parseCommand(":thousands maybe")?.kind, "error");
});

test("unknown commands are errors", () => {
  const command = parseCommand(":frobnicate");
  assert.equal(command?.kind, "error");
});
