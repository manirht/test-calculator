/** REPL commands: lines that start with a colon. */

export type Command =
  | { kind: "help" }
  | { kind: "quit" }
  | { kind: "clear" }
  | { kind: "history"; count?: number }
  | { kind: "places"; value: number }
  | { kind: "mode"; value: string }
  | { kind: "thousands"; value: boolean }
  | { kind: "error"; message: string };

export const MODES = ["half-up", "half-even", "truncate"] as const;
export const MAX_PLACES = 30;

export function parseCommand(line: string): Command | null {
  const trimmed = line.trim();
  if (!trimmed.startsWith(":")) return null;
  const [name, ...rest] = trimmed.slice(1).split(/\s+/);
  const arg = rest[0];
  switch (name) {
    case "help":
    case "h":
      return { kind: "help" };
    case "quit":
    case "exit":
    case "q":
      return { kind: "quit" };
    case "clear":
      return { kind: "clear" };
    case "history": {
      if (arg === undefined) return { kind: "history" };
      const count = Number(arg);
      if (!Number.isInteger(count) || count < 1) return { kind: "error", message: "usage: :history [count]" };
      return { kind: "history", count };
    }
    case "places": {
      const value = Number(arg);
      if (arg === undefined || !Number.isInteger(value) || value < 0 || value > MAX_PLACES) {
        return { kind: "error", message: `usage: :places 0..${MAX_PLACES}` };
      }
      return { kind: "places", value };
    }
    case "mode":
      if (arg === undefined || !(MODES as readonly string[]).includes(arg)) {
        return { kind: "error", message: `usage: :mode ${MODES.join("|")}` };
      }
      return { kind: "mode", value: arg };
    case "thousands":
      if (arg !== "on" && arg !== "off") return { kind: "error", message: "usage: :thousands on|off" };
      return { kind: "thousands", value: arg === "on" };
    default:
      return { kind: "error", message: `unknown command ':${name}' (try :help)` };
  }
}

export const HELP_TEXT = [
  "Enter an expression, e.g. 2^10, sqrt(2), x = 5, mean(1,2,3), pmt(5/1200, 360, 200000).",
  "  ans            the previous result",
  "  :history [n]   show the last n entries",
  "  :places N      digits after the decimal point (0-30)",
  "  :mode M        rounding: half-up, half-even or truncate",
  "  :thousands on|off   group digits in results",
  "  :clear         forget variables and ans",
  "  :quit          leave",
].join("\n");
