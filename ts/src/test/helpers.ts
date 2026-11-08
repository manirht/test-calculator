import type { Transport } from "../client.js";
import type { EvalOptions, EvalResult } from "../protocol.js";
import { CalcError } from "../client.js";
import type { Evaluator } from "../repl.js";

/** A transport that answers requests from a script instead of a process. */
export class FakeTransport implements Transport {
  readonly written: string[] = [];
  private lineCallback: (line: string) => void = () => undefined;
  private exitCallbacks: Array<() => void> = [];
  closed = false;

  constructor(private readonly respond: (request: Record<string, unknown>) => unknown | undefined = () => undefined) {}

  write(line: string): void {
    this.written.push(line);
    const reply = this.respond(JSON.parse(line) as Record<string, unknown>);
    if (reply !== undefined) queueMicrotask(() => this.lineCallback(JSON.stringify(reply)));
  }

  onLine(callback: (line: string) => void): void {
    this.lineCallback = callback;
  }

  onExit(callback: () => void): void {
    this.exitCallbacks.push(callback);
  }

  close(): void {
    this.closed = true;
  }

  emit(line: string): void {
    this.lineCallback(line);
  }

  exit(): void {
    this.exitCallbacks.forEach((callback) => callback());
  }
}

/** An evaluator that returns scripted values, or throws CalcError for scripted failures. */
export class FakeEvaluator implements Evaluator {
  readonly calls: Array<{ expr: string; options?: EvalOptions }> = [];
  resets = 0;

  constructor(private readonly table: Record<string, string | CalcError> = {}) {}

  async evaluate(expr: string, options?: EvalOptions): Promise<EvalResult> {
    this.calls.push({ expr, options });
    const entry = this.table[expr];
    if (entry instanceof CalcError) throw entry;
    const value = entry ?? "0";
    return { value, exact: value };
  }

  async reset(): Promise<void> {
    this.resets++;
  }
}
