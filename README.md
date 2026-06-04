# ternary-wasm

Ternary agent system compiled to WebAssembly for browser-based intelligence. This crate provides the engine behind the **SuperInstance Spreadsheet** running in the browser.

## Architecture

| Module | Description |
|--------|-------------|
| `wasm_bridge` | Serialize/deserialize ternary data for JS↔Rust communication |
| `browser_engine` | Lightweight ternary evolution engine (seed-based PRNG, no `std::time`/`rand`) |
| `cell_engine` | Spreadsheet cell evaluation — each cell is a tiny ternary agent |
| `export_bindings` | `#[no_mangle] extern "C"` exports (wasm-bindgen style, no dependency) |
| `wasm_config` | WASM-specific configuration (memory limits, stack size, PRNG seed) |

## Ternary Basics

- **Trit**: balanced ternary digit — Negative (-1), Zero (0), or Positive (+1)
- **Tryte**: 6 trits, representing integers in [-364, 364]
- **Agent**: a ternary entity with state, energy, and connections to other agents

## Building

### Native (for development/testing)

```bash
cargo test
cargo build
```

### WASM target

```bash
# Install the WASM target
rustup target add wasm32-unknown-unknown

# Build as cdylib
cargo build --target wasm32-unknown-unknown --release --no-default-features

# Optional: generate JS bindings with wasm-bindgen
# wasm-bindgen target/wasm32-unknown-unknown/release/ternary_wasm.wasm --out-dir pkg --target web
```

## Usage

```rust
use ternary_wasm::{WasmConfig, BrowserEngine, CellEngine, wasm_bridge::{Trit, Tryte}};

// Create engine
let config = WasmConfig::with_seed(42);
let mut engine = BrowserEngine::new(config);

// Spawn agents
let a = engine.spawn_agent().unwrap();
let b = engine.spawn_agent().unwrap();
engine.connect(a, b);

// Evolve
engine.tick_n(100);

// Spreadsheet cells
let mut cells = CellEngine::new_grid(10, 10, &WasmConfig::default());
cells.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(42)));
cells.evaluate_all();
```

## Design Principles

- **Pure Rust** — no unsafe code, no external dependencies
- **`no_std` compatible** — uses `core::` and `alloc::` (via feature gate)
- **Deterministic** — XorShift64 PRNG for reproducible simulations
- **WASM-first** — C ABI exports, no `std::time`/`std::thread`, minimal allocation

## License

MIT
