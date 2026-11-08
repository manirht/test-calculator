import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { CalcClient, CalcError, processTransport } from "../client.js";

const here = dirname(fileURLToPath(import.meta.url));
const binary = process.env.CALC_BIN ?? join(here, "..", "..", "..", "target", "debug", "calc");
const skip = existsSync(binary) ? false : `calc binary not built at ${binary} (run cargo build)`;

test("talks to the real engine", { skip }, async () => {
  const client = new CalcClient(processTransport(binary));
  try {
    assert.equal((await client.evaluate("1/3")).value, "0.3333333333");
    assert.equal((await client.evaluate("x = 6")).value, "6");
    assert.equal((await client.evaluate("x * 7")).value, "42");
    assert.equal((await client.evaluate("2/3", { places: 2 })).value, "0.67");
    await assert.rejects(client.evaluate("1/0"), (e: unknown) => e instanceof CalcError && e.code === "DIV_BY_ZERO");
    await client.reset();
    await assert.rejects(client.evaluate("x"), (e: unknown) => e instanceof CalcError && e.code === "UNKNOWN_VAR");
    assert.ok((await client.functions()).includes("sqrt"));
  } finally {
    client.close();
  }
});

test("python functions are reachable through the engine", { skip }, async () => {
  const client = new CalcClient(processTransport(binary));
  try {
    assert.equal((await client.evaluate("mean(1, 2, 4)", { places: 4 })).value, "2.3333");
  } catch (error) {
    if (error instanceof CalcError && error.code === "EXTERNAL") return; // python3 is not installed
    throw error;
  } finally {
    client.close();
  }
});
