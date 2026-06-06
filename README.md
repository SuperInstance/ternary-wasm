# ternary-wasm

**WebAssembly target for ternary computing — trit/tryte arithmetic, serialization bridge (Rust↔JS), browser-based ternary evolution engine, spreadsheet cell evaluation, and C ABI export bindings.**

## Background

WebAssembly (Wasm) is a binary instruction format designed as a portable compilation target for the web. It runs at near-native speed in browsers, supports `no_std` Rust, and provides a linear memory model that maps cleanly to GPU-like computation patterns. For ternary computing, Wasm is the ideal deployment target: it runs in any browser, has predictable performance (no JIT warmup), and can call back to JavaScript for UI rendering.

This crate provides the complete toolchain for running ternary computations in a browser or Wasm runtime:

1. **`wasm_bridge`** — Fundamental ternary types (Trit, Tryte) and serialization. A Trit is {-1, 0, +1}. A Tryte is 6 trits, representing integers in [−364, +364]. Serialization packs trits into bytes (4 trits per byte, 2 bits each) for efficient JS↔Rust transfer.

2. **`browser_engine`** — A multi-agent ternary evolution engine. Agents carry tryte state, interact with connected neighbors, and adjust energy based on ternary interaction results. Uses a deterministic XorShift64 PRNG (no `std::time`, no `rand`).

3. **`cell_engine`** — A spreadsheet-style cell evaluation engine. Each cell holds a ternary expression (literal, reference, sum, product, threshold gate) and evaluates lazily with cycle detection via max depth.

4. **`wasm_config`** — Configuration: max agents, memory limits, PRNG seed, eval depth.

5. **`export_bindings`** — `#[no_mangle] extern "C"` functions that a Wasm host (or native C caller) can invoke directly: `ternary_init`, `ternary_spawn_agent`, `ternary_tick`, `ternary_cell_set_literal`, etc.

### Balanced Ternary: Trits and Trytes

A **trit** (ternary digit) holds one of three values: −1 (Neg), 0 (Zero), +1 (Pos). A **tryte** (ternary byte) is 6 trits, representing an integer via balanced ternary positional notation:

```
value = Σ trit[i] × 3^i  for i = 0..5
```

Range: [−(3⁶−1)/2, +(3⁶−1)/2] = [−364, +364]. Conversion from integer uses balanced division by 3: remainder 0→0, 1→+1, 2→−1 (borrow 1 from quotient).

Trit arithmetic follows the Z₃ group: multiplication is sign multiplication, addition returns (carry, sum) to handle overflow (−1 + −1 = carry −1, sum +1).

## How It Works

### wasm_bridge: Trit and Tryte

**Trit operations:**
- `Trit::mul(a, b)` — Z₃ multiplication: Neg×Neg=Pos, Pos×Pos=Pos, mixed=Neg, anything×Zero=Zero
- `Trit::add(a, b)` — Returns `(carry, sum)`: handles overflow (−1+−1 = carry Neg, sum Pos; +1++1 = carry Pos, sum Neg)
- `Trit::from_i8(v)` — Clamps to nearest trit (negative→Neg, zero→Zero, positive→Pos)

**Tryte operations:**
- `Tryte::to_i32()` — Positional evaluation: `Σ trit[i] × 3^i`
- `Tryte::from_i32(v)` — Balanced ternary conversion via repeated division by 3
- `Tryte::ZERO` — All-zero tryte (constant)

**Serialization:**
- `serialize_trits(trits)` — Packs 4 trits per byte (2 bits each: Neg=00, Zero=01, Pos=10)
- `deserialize_trits(bytes, count)` — Unpacks bytes back to trits
- `encode_trytes_as_i16(trytes)` — For JS `DataView` interop
- `string_to_trytes(s)` — Each ASCII character → one tryte
- `trytes_to_string(trytes)` — Reverse conversion (non-ASCII → '?')

### browser_engine: Ternary Evolution

**TernaryAgent** — An agent with:
- `state: Tryte` — Current state (6 trits)
- `energy: u8` — Survival resource (starts at 128, max 255)
- `connections: Vec<u32>` — Links to other agents

**Interaction**: `agent.evaluate(neighbor_state)` computes `Trit::add(self.state.0[0], neighbor.0[0])`, updates the agent's first trit, and adjusts energy:
- Result = Pos → energy + 1 (positive interaction)
- Result = Neg → energy − 1 (negative interaction)
- Result = Zero → no change (neutral)

**Engine**: `BrowserEngine` manages a population of agents. `tick()` runs one round of pairwise interactions. `tick_n(n)` runs multiple rounds. `state_hash()` computes a deterministic hash of the entire engine state for verification.

### cell_engine: Spreadsheet Evaluation

**CellExpr** — Expression types:
- `Literal(Tryte)` — Constant value
- `Ref(CellRef)` — Reference to another cell
- `Sum(a, b)` — Sum of two cell values
- `Product(a, b)` — Product of two cell values
- `Threshold(a, b)` — If a > b → Pos; if a < b → Neg; else Zero

**Evaluation**: Recursive evaluation with `max_eval_depth` guard against circular references. `evaluate_all()` recomputes dirty cells in two passes (evaluate, then update cache).

### export_bindings: C ABI

Global engine state (static mut) with `extern "C"` functions:
- `ternary_init(seed)` — Initialize engine
- `ternary_spawn_agent()` → agent ID
- `ternary_tick(n)` — Run n evolution ticks
- `ternary_cell_set_literal(row, col, value)` — Set spreadsheet cell
- `ternary_cell_evaluate_all()` — Recompute cells
- `ternary_serialize_trits(...)` — Pack trits for JS transfer

### Design Decisions

1. **`#![no_std]` compatible**: The entire crate compiles without `std` (using `alloc` for `Vec`, `String`). This is required for Wasm targets that don't have full OS support.

2. **XorShift64 PRNG**: Deterministic, no external dependencies. Seed-based initialization ensures reproducible simulations across platforms.

3. **Global static state**: The export bindings use `static mut` for engine state. This is the simplest pattern for Wasm (single-threaded by default). Thread-safety would require thread-local storage.

4. **Two-pass cell evaluation**: Evaluate all dirty cells first (computing new values), then update caches. This prevents partial updates from being visible during evaluation.

## Experimental Results

All **22 tests pass**:

| Test Class | Tests | Key Results |
|-----------|-------|-------------|
| `wasm_bridge::trit` | 3 | `from_i8` clamps correctly; `mul` follows Z₃ table; `add` handles overflow with carry |
| `wasm_bridge::tryte` | 3 | Zero constant; `to_i32` ↔ `from_i32` roundtrip; clamping at ±364 |
| `wasm_bridge::serialize` | 2 | `serialize → deserialize` roundtrip preserves all trits |
| `wasm_bridge::string` | 1 | `string_to_trytes → trytes_to_string` roundtrip for ASCII |
| `browser_engine` | 5 | PRNG deterministic (same seed → same output); agent spawning; energy dynamics; state hash |
| `export_bindings` | 2 | `ternary_init` + `ternary_spawn_agent` returns valid ID; agent count tracking |
| `cell_engine` | 6 | Grid creation; cell set/get; evaluate dirty cells; literal values |

Key findings:
- **Trit addition overflow**: `Trit::add(Pos, Pos)` = `(carry: Pos, sum: Neg)` — the carry propagates to the next trit position, exactly like base-10 carries but in balanced ternary
- **Tryte roundtrip**: `Tryte::from_i32(42)` → `to_i32()` = 42; `from_i32(-200)` → `to_i32()` = −200; `from_i32(500)` → clamped to 364
- **String roundtrip**: `"Hello"` → trytes → back to `"Hello"` — lossless ASCII encoding via balanced ternary
- **Deterministic PRNG**: `Prng::new(42)` always produces the same sequence of trits and trytes

## Impact

The ternary {-1, 0, +1} encoding enables efficient Wasm deployment in three ways:

1. **Compact serialization**: 4 trits per byte (2 bits each) vs. 1 byte per value = 4× compression. For a grid of 10,000 ternary cells, that's 2,500 bytes vs. 10,000 bytes — significant for Wasm's linear memory.

2. **Three-way interaction semantics**: The `evaluate()` function naturally models cooperation/defection games. Positive interaction → energy gain. Negative → energy loss. Neutral → no change. This is a ternary payoff matrix.

3. **Threshold gates as ternary comparators**: The `Threshold(a, b)` expression returns Pos/Neg/Zero based on comparison — a natural ternary decision. Combined with Sum and Product, this creates a ternary spreadsheet that can implement arbitrary ternary logic circuits.

## Use Cases

1. **Browser-based ternary simulations** — Run ternary cellular automata, agent evolution, or consensus algorithms in the browser with near-native performance
2. **Educational ternary computing** — Interactive web demos where students can set ternary cell values and watch evolution in real-time
3. **Ternary data visualization** — Serialize ternary data from Rust, transfer to JS via Wasm bridge, render as heatmaps or particle systems
4. **Edge computing** — Deploy ternary signal processing (quantize, smooth, classify) on Wasm-based edge runtimes (Cloudflare Workers, Fastly Compute)
5. **Ternary text encoding** — Use `string_to_trytes` to encode text in balanced ternary for novel communication protocols

## Open Questions

1. **Wasm multithreading**: The current `static mut` pattern is not thread-safe. When SharedArrayBuffer + Wasm threads are available, how should the engine state be partitioned across threads?
2. **Performance on Wasm vs native**: How much overhead does the Wasm execution add compared to native? The PRNG and evaluation loops should be benchmarked on both targets.
3. **Cell engine circular references**: The `max_eval_depth` guard prevents infinite recursion, but it silently returns Zero. Should circular references be detected and reported as errors?

## Connection to Oxide Stack

`ternary-wasm` is the **deployment layer** for the Oxide Stack. It takes computations defined in flux-core (agent behavior, cell evaluation) and makes them runnable in browsers and Wasm runtimes. The `wasm_bridge` serialization layer is the protocol between Rust-side computation and JS-side visualization — the same bridge pattern used at the cudaclaw layer for GPU↔CPU data transfer.

The `browser_engine` is a lightweight version of the cudaclaw persistent kernel: agents with state, connections, and energy interacting in rounds. The `cell_engine` models the kind of lazy evaluation that happens in the flux-core VM but with a spreadsheet metaphor that's accessible to non-programmers.

The export bindings (`ternary_init`, `ternary_spawn_agent`, etc.) follow the same naming convention as the CUDA kernel API (`cudaclaw_init`, `cudaclaw_spawn`), ensuring that code written against the Wasm target can be ported to the GPU target with minimal changes.

## Stats

| Metric | Value |
|--------|-------|
| Lines of Rust | ~580 |
| Test count | 22 |
| Modules | 5 |
| Public types | 8 (Trit, Tryte, WasmBridge, Prng, TernaryAgent, BrowserEngine, Cell, CellEngine, WasmConfig) |
| C ABI exports | 10 |
| `#![no_std]` | Yes (with alloc) |

## Install

```toml
[dependencies]
ternary-wasm = "0.1.0"
```

## License

MIT
