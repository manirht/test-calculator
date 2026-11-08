/** Tab completion for function and variable names. */

export interface Completion {
  /** Index in the line where the completed word starts. */
  start: number;
  candidates: string[];
}

export function complete(line: string, names: readonly string[]): Completion {
  const match = /[A-Za-z_][A-Za-z0-9_]*$/.exec(line);
  if (!match) return { start: line.length, candidates: [] };
  const prefix = match[0].toLowerCase();
  const candidates = [...new Set(names)]
    .filter((name) => name.toLowerCase().startsWith(prefix) && name.toLowerCase() !== prefix)
    .sort((a, b) => a.localeCompare(b));
  return { start: match.index, candidates };
}
