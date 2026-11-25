#!/usr/bin/env node
import { createInterface } from "node:readline";
import { USAGE, parseArgs } from "./args.js";
import { CalcClient, processTransport } from "./client.js";
import { complete } from "./complete.js";
import { History, loadHistory, saveHistory } from "./history.js";
import { DEFAULT_SETTINGS, Repl } from "./repl.js";

const DEFAULT_HISTORY_FILE = ".calc_history.json";

async function main(argv: string[]): Promise<number> {
  const args = parseArgs(argv, process.env);
  if (args.error) {
    console.error(`calc-ui: ${args.error}\n${USAGE}`);
    return 2;
  }
  if (args.help) {
    console.log(USAGE);
    return 0;
  }

  const client = new CalcClient(processTransport(args.bin));
  const historyFile = args.historyFile ?? DEFAULT_HISTORY_FILE;
  const history = args.noHistory || args.expr !== undefined ? new History() : loadHistory(historyFile);
  const settings = { ...DEFAULT_SETTINGS };
  if (args.places !== undefined) settings.places = args.places;
  if (args.mode !== undefined) settings.mode = args.mode;
  const repl = new Repl(client, history, settings);

  try {
    if (args.expr !== undefined) {
      const output = await repl.handleLine(args.expr);
      (output.error ? console.error : console.log)(output.text);
      return output.error ? 1 : 0;
    }

    let names: string[] = [];
    try {
      names = await client.functions();
    } catch {
      // Completion is a convenience; the session still works without it.
    }
    const interactive = Boolean(process.stdin.isTTY && process.stdout.isTTY);
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
      terminal: interactive,
      prompt: "> ",
      completer: (line: string) => {
        const { start, candidates } = complete(line, names);
        return [candidates, line.slice(start)];
      },
    });
    let inputClosed = false;
    rl.once("close", () => { inputClosed = true; });
    const prompt = () => { if (interactive && !inputClosed) rl.prompt(); };
    prompt();
    for await (const line of rl) {
      const output = await repl.handleLine(line);
      if (output.text) console.log(output.text);
      if (output.quit) break;
      prompt();
    }
    rl.close();
    if (!args.noHistory) saveHistory(historyFile, history);
    return 0;
  } finally {
    client.close();
  }
}

main(process.argv.slice(2)).then(
  (code) => process.exit(code),
  (error) => {
    console.error(error);
    process.exit(1);
  },
);
