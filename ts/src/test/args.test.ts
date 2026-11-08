import assert from "node:assert/strict";
import { test } from "node:test";
import { parseArgs } from "../args.js";

test("defaults", () => {
  const args = parseArgs([]);
  assert.equal(args.bin, "calc");
  assert.equal(args.expr, undefined);
  assert.equal(args.noHistory, false);
  assert.equal(args.help, false);
});

test("CALC_BIN sets the binary and --bin overrides it", () => {
  assert.equal(parseArgs([], { CALC_BIN: "/opt/calc" }).bin, "/opt/calc");
  assert.equal(parseArgs(["--bin", "/x/calc"], { CALC_BIN: "/opt/calc" }).bin, "/x/calc");
});

test("expression and options", () => {
  const args = parseArgs(["--places", "3", "--mode", "truncate", "2/3"]);
  assert.equal(args.expr, "2/3");
  assert.equal(args.places, 3);
  assert.equal(args.mode, "truncate");
});

test("a leading minus is part of the expression, not an option", () => {
  assert.equal(parseArgs(["-5+1"]).expr, "-5+1");
});

test("flags", () => {
  assert.equal(parseArgs(["-h"]).help, true);
  assert.equal(parseArgs(["--no-history"]).noHistory, true);
  assert.equal(parseArgs(["--history-file", "h.json"]).historyFile, "h.json");
});

test("errors", () => {
  assert.match(parseArgs(["--places"]).error ?? "", /--places/);
  assert.match(parseArgs(["--places", "-2"]).error ?? "", /--places/);
  assert.match(parseArgs(["--places", "x"]).error ?? "", /--places/);
  assert.match(parseArgs(["--bin"]).error ?? "", /--bin/);
  assert.match(parseArgs(["--mode"]).error ?? "", /--mode/);
  assert.match(parseArgs(["--nope"]).error ?? "", /unknown option/);
  assert.match(parseArgs(["1", "2"]).error ?? "", /unexpected argument/);
});
