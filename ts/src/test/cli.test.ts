import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const binary = process.env.CALC_BIN ?? join(here, "..", "..", "..", "target", "debug", "calc");
const cli = join(here, "..", "cli.js");
const skip = existsSync(binary) ? false : "build calc with cargo build before CLI integration tests";

test("piped REPL input drains every line after EOF without trying to prompt a closed readline", { skip }, () => {
  const cwd = mkdtempSync(join(tmpdir(), "calc-cli-pipe-"));
  try {
    const result = spawnSync(process.execPath, [cli, "--no-history"], {
      cwd,
      env: { ...process.env, CALC_BIN: binary },
      input: "1+1\n:places 0\n10/4\n:history 0\n:quit\n",
      encoding: "utf8",
      timeout: 5000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout, "2\nplaces = 0\n3\nusage: :history [count]\n");
    assert.equal(result.stderr, "");
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
});
