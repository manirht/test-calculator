import { readFileSync, writeFileSync } from "node:fs";

export interface Entry {
  expr: string;
  result: string;
  at: number;
}

export const DEFAULT_MAX_ENTRIES = 100;

export class History {
  private entries: Entry[] = [];

  constructor(private readonly max = DEFAULT_MAX_ENTRIES) {}

  get size(): number {
    return this.entries.length;
  }

  /** Records an entry. Repeating the previous expression updates it instead of adding another. */
  add(entry: Entry): void {
    const last = this.entries[this.entries.length - 1];
    if (last && last.expr === entry.expr) {
      this.entries[this.entries.length - 1] = entry;
      return;
    }
    this.entries.push(entry);
    while (this.entries.length > this.max) this.entries.shift();
  }

  list(count?: number): Entry[] {
    if (count === undefined) return [...this.entries];
    return count <= 0 ? [] : this.entries.slice(-count);
  }

  last(): Entry | undefined {
    return this.entries[this.entries.length - 1];
  }

  search(term: string): Entry[] {
    const needle = term.toLowerCase();
    return this.entries.filter((entry) => entry.expr.toLowerCase().includes(needle));
  }

  clear(): void {
    this.entries = [];
  }

  toJSON(): Entry[] {
    return [...this.entries];
  }

  static fromJSON(data: unknown, max = DEFAULT_MAX_ENTRIES): History {
    const history = new History(max);
    if (!Array.isArray(data)) return history;
    for (const item of data) {
      if (
        typeof item === "object" &&
        item !== null &&
        typeof (item as Entry).expr === "string" &&
        typeof (item as Entry).result === "string" &&
        typeof (item as Entry).at === "number"
      ) {
        history.add(item as Entry);
      }
    }
    return history;
  }
}

/** Loads saved history; a missing or corrupt file yields an empty history. */
export function loadHistory(path: string, max = DEFAULT_MAX_ENTRIES): History {
  try {
    return History.fromJSON(JSON.parse(readFileSync(path, "utf8")), max);
  } catch {
    return new History(max);
  }
}

export function saveHistory(path: string, history: History): void {
  writeFileSync(path, JSON.stringify(history.toJSON(), null, 2) + "\n");
}
