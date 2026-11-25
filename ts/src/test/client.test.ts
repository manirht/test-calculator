import assert from "node:assert/strict";
import { test } from "node:test";
import { CalcClient, CalcError } from "../client.js";
import { FakeTransport } from "./helpers.js";

function ok(request: Record<string, unknown>, value: string) {
  return { id: request.id, ok: true, value, exact: value };
}

test("evaluate sends the expression and options and returns the value", async () => {
  const transport = new FakeTransport((req) => ok(req, "3"));
  const client = new CalcClient(() => transport);
  const result = await client.evaluate("1+2", { places: 4, mode: "truncate" });
  assert.deepEqual(result, { value: "3", exact: "3" });
  const sent = JSON.parse(transport.written[0]);
  assert.equal(sent.expr, "1+2");
  assert.equal(sent.places, 4);
  assert.equal(sent.mode, "truncate");
  client.close();
});

test("replies are matched to requests by id", async () => {
  const transport = new FakeTransport();
  const client = new CalcClient(() => transport);
  const first = client.evaluate("a");
  const second = client.evaluate("b");
  transport.emit(JSON.stringify({ id: 2, ok: true, value: "B", exact: "B" }));
  transport.emit(JSON.stringify({ id: 1, ok: true, value: "A", exact: "A" }));
  assert.equal((await first).value, "A");
  assert.equal((await second).value, "B");
  client.close();
});

test("error replies become CalcError with code and position", async () => {
  const transport = new FakeTransport((req) => ({
    id: req.id,
    ok: false,
    error: { code: "SYNTAX", message: "unexpected '*' at 4", pos: 4 },
  }));
  const client = new CalcClient(() => transport);
  await assert.rejects(client.evaluate("1 + * 2"), (error: unknown) => {
    assert.ok(error instanceof CalcError);
    assert.equal(error.code, "SYNTAX");
    assert.equal(error.pos, 4);
    return true;
  });
  client.close();
});

test("a missing reply times out", async () => {
  const transport = new FakeTransport();
  const client = new CalcClient(() => transport, 20);
  await assert.rejects(client.evaluate("1"), (error: unknown) => error instanceof CalcError && error.code === "TIMEOUT");
  client.close();
});

test("garbage and unknown ids are ignored", async () => {
  const transport = new FakeTransport();
  const client = new CalcClient(() => transport, 500);
  const pending = client.evaluate("1");
  transport.emit("not json");
  transport.emit(JSON.stringify({ id: 99, ok: true, value: "x", exact: "x" }));
  transport.emit(JSON.stringify({ hello: "world" }));
  transport.emit(JSON.stringify({ id: 1, ok: true, value: "1", exact: "1" }));
  assert.equal((await pending).value, "1");
  client.close();
});

test("process exit rejects pending requests and the next call starts a new process", async () => {
  const transports: FakeTransport[] = [];
  const client = new CalcClient(() => {
    const transport = new FakeTransport((req) => (transports.length > 1 ? ok(req, "2") : undefined));
    transports.push(transport);
    return transport;
  }, 500);
  const pending = client.evaluate("1+1");
  transports[0].exit();
  await assert.rejects(pending, (error: unknown) => error instanceof CalcError && error.code === "EXITED");
  assert.equal((await client.evaluate("1+1")).value, "2");
  assert.equal(transports.length, 2);
  client.close();
});

test("reset and functions use commands", async () => {
  const transport = new FakeTransport((req) =>
    req.cmd === "functions" ? { id: req.id, ok: true, functions: ["abs", "mean"] } : { id: req.id, ok: true },
  );
  const client = new CalcClient(() => transport);
  await client.reset();
  assert.deepEqual(await client.functions(), ["abs", "mean"]);
  assert.equal(JSON.parse(transport.written[0]).cmd, "reset");
  client.close();
});

test("a reply without a value is a protocol error", async () => {
  const transport = new FakeTransport((req) => ({ id: req.id, ok: true }));
  const client = new CalcClient(() => transport);
  await assert.rejects(client.evaluate("1"), (error: unknown) => error instanceof CalcError && error.code === "PROTOCOL");
  client.close();
});

test("closing a client promptly rejects requests still waiting for a reply", async () => {
  const client = new CalcClient(() => new FakeTransport(), 100);
  const pending = client.evaluate("1+1");
  const rejected = assert.rejects(pending, (error: unknown) => error instanceof CalcError && error.code === "CLOSED");
  client.close();
  await rejected;
});

test("a late exit from a closed transport cannot reject a new transport's request", async () => {
  const oldTransport = new FakeTransport((request) => ok(request, "2"));
  const newTransport = new FakeTransport();
  const transports = [oldTransport, newTransport];
  const client = new CalcClient(() => transports.shift()!, 500);
  assert.equal((await client.evaluate("1+1")).value, "2");
  client.close();
  const next = client.evaluate("2+2");
  oldTransport.exit();
  newTransport.emit(JSON.stringify({ id: 2, ok: true, value: "4", exact: "4" }));
  try {
    assert.equal((await next).value, "4");
  } finally {
    client.close();
  }
});
