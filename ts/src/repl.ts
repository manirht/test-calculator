import { CalcError } from "./client.js";
import { HELP_TEXT, parseCommand } from "./commands.js";
import { formatNumber } from "./formatter.js";
import { History } from "./history.js";
import type { EvalOptions, EvalResult } from "./protocol.js";

/** What the REPL needs from the engine; CalcClient satisfies it. */
export interface Evaluator {
  evaluate(expr: string, options?: EvalOptions): Promise<EvalResult>;
  reset(): Promise<void>;
}

export interface ReplSettings {
  places: number;
  mode: string;
  thousands: boolean;
}

export interface ReplOutput {
  text: string;
  quit?: boolean;
  error?: boolean;
}

/** The engine reports byte offsets into the UTF-8 input; the caret needs a character column. */
export function columnOf(input: string, byteOffset: number): number {
  const bytes = new TextEncoder().encode(input);
  return new TextDecoder().decode(bytes.slice(0, byteOffset)).length;
}

export const DEFAULT_SETTINGS: ReplSettings = { places: 10, mode: "half-up", thousands: false };

export class Repl {
  constructor(
    private readonly evaluator: Evaluator,
    readonly history: History,
    readonly settings: ReplSettings = { ...DEFAULT_SETTINGS },
  ) {}

  async handleLine(line: string, now: () => number = Date.now): Promise<ReplOutput> {
    const input = line.trim();
    if (input === "") return { text: "" };
    const command = parseCommand(input);
    if (command) return this.runCommand(command);
    try {
      const result = await this.evaluator.evaluate(input, { places: this.settings.places, mode: this.settings.mode });
      const text = formatNumber(result.value, { thousands: this.settings.thousands });
      this.history.add({ expr: input, result: text, at: now() });
      return { text };
    } catch (error) {
      return { text: this.describeError(error, input), error: true };
    }
  }

  private async runCommand(command: ReturnType<typeof parseCommand> & object): Promise<ReplOutput> {
    switch (command.kind) {
      case "help":
        return { text: HELP_TEXT };
      case "quit":
        return { text: "", quit: true };
      case "clear":
        await this.evaluator.reset();
        return { text: "cleared variables and ans" };
      case "history": {
        const entries = this.history.list(command.count);
        if (entries.length === 0) return { text: "(no history)" };
        return { text: entries.map((e, i) => `${i + 1}  ${e.expr} = ${e.result}`).join("\n") };
      }
      case "places":
        this.settings.places = command.value;
        return { text: `places = ${command.value}` };
      case "mode":
        this.settings.mode = command.value;
        return { text: `mode = ${command.value}` };
      case "thousands":
        this.settings.thousands = command.value;
        return { text: `thousands = ${command.value ? "on" : "off"}` };
      case "error":
        return { text: command.message, error: true };
    }
  }

  private describeError(error: unknown, input: string): string {
    if (error instanceof CalcError) {
      const head = `error[${error.code}]: ${error.message}`;
      if (error.pos === undefined) return head;
      return `${head}\n  ${input}\n  ${" ".repeat(columnOf(input, error.pos))}^`;
    }
    return `error: ${error instanceof Error ? error.message : String(error)}`;
  }
}
