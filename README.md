# test-calculator

A small exact-arithmetic calculator split across three languages. It exists as a realistic,
medium-sized target for testing [LYRA](https://github.com/TechWizard9999/LYRA): bugs are planted
on top of this clean base and LYRA is asked to reproduce, root-cause and fix them.

```
ts/ (TypeScript)        REPL and CLI: history, formatting, completion
   │  spawns `calc serve`, JSON lines on stdin/stdout
   ▼
crates/ (Rust)          calc-lexer → calc-parser → calc-eval → calc-cli (`calc`)
   │  spawns `python3 -m calcstats`, JSON lines, numbers as exact strings ("7", "1/3")
   ▼
python/ (Python)        calcstats: statistics and finance functions
```

- **Rust** owns parsing and exact rational arithmetic (`i128` fractions, no floats).
- **Python** provides `mean median mode variance stddev pvariance pstddev percentile pmt fv pv npv irr compound`.
- **TypeScript** is the user-facing shell: `calc-ui "2^10"` or an interactive session with `:places`, `:mode`, `:history`.

## Use it

```sh
cargo build
cargo run -p calc-cli -- eval "1/3"                  # 0.3333333333
cargo run -p calc-cli -- eval "mean(1,2,4)" --places 4
cd ts && npm ci && npm run build && CALC_BIN=../target/debug/calc node dist/cli.js
```

Expression language: `+ - * / %` (floored modulo), `^` (right associative, `-2^2` is `-4`), postfix `!`,
variables (`x = 5`, names are case-sensitive), `ans`, constants `pi` and `e` (15 significant digits), and the
functions listed by `:help`. Error positions are byte offsets into the UTF-8 input.

## Test it

```sh
./scripts/test-all.sh          # everything below
cargo test --workspace         # Rust
(cd python && python3 -m unittest discover tests)
(cd ts && npm ci && npm test)
```

Requires Rust (stable), Python 3.9+ and Node 20+. Python tests use only the standard library.
The Rust end-to-end tests and the TypeScript end-to-end tests skip themselves when `python3` or the
built `calc` binary is missing.

## Layout

| Path | What |
|---|---|
| `crates/calc-lexer` | tokenizer |
| `crates/calc-parser` | Pratt parser, AST, pretty printer |
| `crates/calc-eval` | rationals, evaluator, builtins, Python bridge, output formatting |
| `crates/calc-cli` | `calc eval` and `calc serve` (JSON lines) |
| `python/calcstats` | protocol, `stats.py`, `finance.py` |
| `ts/src` | client, REPL, history, formatter, completion, argument parsing |
