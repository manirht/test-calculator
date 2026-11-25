import { MAX_PLACES, MODES } from "./commands.js";

export interface Args {
  expr?: string;
  places?: number;
  mode?: string;
  bin: string;
  historyFile?: string;
  noHistory: boolean;
  help: boolean;
  error?: string;
}

export const DEFAULT_BIN = "calc";

export function parseArgs(argv: string[], env: Record<string, string | undefined> = {}): Args {
  const args: Args = { bin: env.CALC_BIN ?? DEFAULT_BIN, noHistory: false, help: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = (): string | undefined => argv[++i];
    switch (arg) {
      case "--help":
      case "-h":
        args.help = true;
        break;
      case "--no-history":
        args.noHistory = true;
        break;
      case "--bin": {
        const v = value();
        if (v === undefined) return { ...args, error: "--bin needs a path" };
        args.bin = v;
        break;
      }
      case "--history-file": {
        const v = value();
        if (v === undefined) return { ...args, error: "--history-file needs a path" };
        args.historyFile = v;
        break;
      }
      case "--places": {
        const v = value();
        const places = Number(v);
        if (v === undefined || !Number.isInteger(places) || places < 0) {
          return { ...args, error: "--places needs a non-negative integer" };
        }
        if (places > MAX_PLACES) return { ...args, error: `--places must be at most ${MAX_PLACES}` };
        args.places = places;
        break;
      }
      case "--mode": {
        const v = value();
        if (v === undefined) return { ...args, error: "--mode needs a value" };
        if (!(MODES as readonly string[]).includes(v)) {
          return { ...args, error: `--mode must be ${MODES.join("|")}` };
        }
        args.mode = v;
        break;
      }
      default:
        if (arg.startsWith("--")) return { ...args, error: `unknown option ${arg}` };
        if (args.expr !== undefined) return { ...args, error: `unexpected argument ${arg}` };
        args.expr = arg;
    }
  }
  return args;
}

export const USAGE = [
  "usage: calc-ui [options] [expression]",
  "  --bin PATH          calc binary (default: $CALC_BIN or `calc` on PATH)",
  "  --places N          digits after the decimal point",
  "  --mode MODE         half-up | half-even | truncate",
  "  --history-file F    where to keep REPL history (default: ./.calc_history.json)",
  "  --no-history        do not read or write history",
  "  -h, --help          show this help",
  "Without an expression, starts an interactive session.",
].join("\n");
