import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { isReply } from "./protocol.js";
import type { EvalOptions, EvalResult, Reply } from "./protocol.js";

export class CalcError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly pos?: number,
  ) {
    super(message);
    this.name = "CalcError";
  }
}

/** A line-oriented duplex channel to one `calc serve` process. */
export interface Transport {
  write(line: string): void;
  onLine(callback: (line: string) => void): void;
  onExit(callback: () => void): void;
  close(): void;
}

export type TransportFactory = () => Transport;

interface Pending {
  resolve: (reply: Reply) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

export class CalcClient {
  private transport: Transport | null = null;
  private readonly pending = new Map<number, Pending>();
  private nextId = 1;

  constructor(
    private readonly factory: TransportFactory,
    private readonly timeoutMs = 5000,
  ) {}

  private ensureTransport(): Transport {
    if (this.transport) return this.transport;
    const transport = this.factory();
    transport.onLine((line) => this.handleLine(line));
    transport.onExit(() => this.handleExit(transport));
    this.transport = transport;
    return transport;
  }

  private handleLine(line: string): void {
    let reply: unknown;
    try {
      reply = JSON.parse(line);
    } catch {
      return;
    }
    if (!isReply(reply) || typeof reply.id !== "number") return;
    const pending = this.pending.get(reply.id);
    if (!pending) return;
    this.pending.delete(reply.id);
    clearTimeout(pending.timer);
    pending.resolve(reply);
  }

  private handleExit(transport: Transport): void {
    if (this.transport !== transport) return;
    this.transport = null;
    this.rejectPending(new CalcError("EXITED", "the calc process exited"));
  }

  private rejectPending(error: CalcError): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  private request(body: Record<string, unknown>): Promise<Reply> {
    const id = this.nextId++;
    const transport = this.ensureTransport();
    return new Promise<Reply>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new CalcError("TIMEOUT", `no reply within ${this.timeoutMs} ms`));
      }, this.timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      transport.write(JSON.stringify({ id, ...body }));
    });
  }

  async evaluate(expr: string, options: EvalOptions = {}): Promise<EvalResult> {
    const reply = await this.request({ expr, ...options });
    if (!reply.ok) throw new CalcError(reply.error.code, reply.error.message, reply.error.pos);
    if (typeof reply.value !== "string" || typeof reply.exact !== "string") {
      throw new CalcError("PROTOCOL", "reply is missing a value");
    }
    return { value: reply.value, exact: reply.exact };
  }

  async reset(): Promise<void> {
    const reply = await this.request({ cmd: "reset" });
    if (!reply.ok) throw new CalcError(reply.error.code, reply.error.message);
  }

  async functions(): Promise<string[]> {
    const reply = await this.request({ cmd: "functions" });
    if (!reply.ok) throw new CalcError(reply.error.code, reply.error.message);
    return reply.functions ?? [];
  }

  close(): void {
    const transport = this.transport;
    this.transport = null;
    this.rejectPending(new CalcError("CLOSED", "the calc client was closed"));
    transport?.close();
  }
}

/** Spawns `<binary> serve` and exposes it as a Transport. */
export function processTransport(binary: string, args: string[] = ["serve"]): TransportFactory {
  return () => {
    const child = spawn(binary, args, { stdio: ["pipe", "pipe", "inherit"] });
    const lines = createInterface({ input: child.stdout });
    const exitCallbacks: Array<() => void> = [];
    const notifyExit = () => exitCallbacks.splice(0).forEach((callback) => callback());
    child.on("exit", notifyExit);
    child.on("error", notifyExit);
    child.stdin.on("error", () => undefined);
    return {
      write: (line) => {
        child.stdin.write(line + "\n");
      },
      onLine: (callback) => {
        lines.on("line", callback);
      },
      onExit: (callback) => {
        exitCallbacks.push(callback);
      },
      close: () => {
        child.stdin.end();
        child.kill();
      },
    };
  };
}
