# ternary-wasm

**Ternary agents in the browser. A `no_std`-compatible engine for ternary evolution, spreadsheet cells, and agent simulation — compiled to WebAssembly with zero dependencies.**

## Why This Exists

Most ML inference happens on servers. But for interactive applications — spreadsheets with embedded intelligence, simulations users can tweak in real-time, educational tools that show how ternary networks evolve — you need computation in the browser. The problem: browser WASM runtimes have no threads, no `std::time`, no `rand`, and tight memory budgets.

This crate solves those constraints by design. It's a complete ternary agent system built on:
- **XorShift64 PRNG** — deterministic, seed-based randomness (no `rand` crate needed)
- **`no_std` + `alloc`** — works in WASM without `std`
- **C ABI exports** — `#[no_mangle] extern "C"` functions that JavaScript can call directly, no wasm-bindgen required
- **Bounded memory** — configurable limits for stack size, agent count, and grid dimensions

The result: a ternary engine that compiles to ~50KB WASM, initializes in milliseconds, and runs deterministic simulations that reproduce identically across every browser.

## The Key Insight

Determinism in distributed systems isn't a nice-to-have — it's a correctness requirement. When a ternary agent system evolves in a browser and the same system evolves on a server, they must produce identical results. Float-based systems can't guarantee this (different hardware, different rounding). Ternary systems can, because {-1, 0, +1} arithmetic has no rounding. The only source of non-determinism is the PRNG seed, which you control.

## Quick Start

### Rust (native development/testing)

```rust
use ternary_wasm::{WasmConfig, BrowserEngine, CellEngine};
use ternary_wasm::wasm_bridge::{Trit, Tryte};

// Create engine with deterministic seed
let config = WasmConfig::with_seed(42);
let mut engine = BrowserEngine::new(config);

// Spawn ternary agents
let a = engine.spawn_agent().unwrap();
let b = engine.spawn_agent().unwrap();

// Connect them (affinity link)
engine.connect(a, b);

// Evolve the system 100 ticks
engine.tick_n(100);

// Check agent state
let agents = engine.agents();
println!("Agent {}: energy = {}", agents[0].id, agents[0].energy);
```

### Spreadsheet Cells

```rust
use ternary_wasm::{CellEngine, WasmConfig};
use ternary_wasm::cell_engine::CellExpr;
use ternary_wasm::wasm_bridge::Tryte;

// Create a 10×10 grid of ternary spreadsheet cells
let config = WasmConfig::default();
let mut cells = CellEngine::new_grid(10, 10, &config);

// Set cell (0,0) to literal value 42 (encoded as a ternary tryte)
cells.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(42)));

// Reference another cell
cells.set_cell(0, 1, CellExpr::Reference(0, 0));

// Evaluate the entire grid
cells.evaluate_all();
```

### Building for WASM

```bash
# Install the target
rustup target add wasm32-unknown-unknown

# Build as cdylib (no default features → no_std)
cargo build --target wasm32-unknown-unknown --release --no-default-features

# The output .wasm file has C ABI exports that JS can call directly
```

## Architecture

### Module Layout

```
ternary-wasm
├── wasm_bridge        # Trit/Tryte types, serialize/deserialize for JS↔Rust
├── browser_engine     # Agent evolution engine (spawn, connect, tick)
├── cell_engine        # Spreadsheet cell evaluation (formulas, references)
├── export_bindings    # #[no_mangle] extern "C" functions for JS
└── wasm_config        # Memory limits, stack size, PRNG seed
```

### Data Types

| Type | Size | Range | Description |
|------|------|-------|-------------|
| `Trit` | 1 byte | {-1, 0, +1} | Single ternary digit |
| `Tryte` | 1 byte | [-364, +364] | 6 trits packed, ternary "byte" |

A **tryte** is the ternary analog of a byte: 6 ternary digits. Where a byte is 8 bits (256 values), a tryte is 6 trits (3⁶ = 729 values, balanced around zero). It's the natural unit for ternary computation.

### BrowserEngine

The evolution engine manages a population of ternary agents:

```
┌──────────────────────────────────┐
│         BrowserEngine            │
│                                  │
│  Agents: Vec<Agent>              │
│    each has: state, energy,      │
│    connections, position         │
│                                  │
│  PRNG: XorShift64 (seeded)       │
│  Config: limits, tick budget     │
│                                  │
│  tick():   advance one step      │
│  tick_n(): advance N steps       │
└──────────────────────────────────┘
```

Agents have energy (which rises and falls during evolution) and connections to other agents (affinity links that influence evolution). The `tick()` method advances the entire system one step: agents interact with their neighbors, energy flows along connections, and states update.

### CellEngine

The spreadsheet engine treats each cell as a tiny ternary agent with an expression:

```rust
pub enum CellExpr {
    Literal(Tryte),           // Constant value
    Reference(usize, usize),  // Reference to another cell
    Add(Box<CellExpr>, Box<CellExpr>),  // Z₃ addition
    Mul(Box<CellExpr>, Box<CellExpr>),  // Z₃ multiplication
    // ... more operations
}
```

This is essentially a spreadsheet formula system where all arithmetic is ternary. Dependencies are tracked so `evaluate_all()` processes cells in topological order.

### WasmConfig

```rust
pub struct WasmConfig {
    pub seed: u64,            // PRNG seed (determinism)
    pub max_agents: usize,    // Memory budget
    pub max_stack: usize,     // Call stack depth
    pub memory_limit: usize,  // Total allocation ceiling
}
```

These limits matter in WASM: there's no swap, no virtual memory. Hitting the limit is an OOM crash. The config lets you set ceilings that match your deployment environment.

## Design Principles

**No `unsafe`** — The entire crate is safe Rust. WASM's sandbox already provides memory safety; adding `unsafe` would be double-bookkeeping with no benefit.

**No `std`** — The `std` feature is optional (default on for native testing). In WASM builds, only `core` and `alloc` are used. No filesystem, no threads, no system time.

**Deterministic** — Same seed → same simulation, always. XorShift64 is a well-understood PRNG with a 64-bit state. It's not cryptographically secure (don't use it for key generation), but it's perfect for reproducible simulations.

**C ABI exports** — Instead of depending on wasm-bindgen (which adds build complexity and JS glue code), the crate exports plain `extern "C"` functions. JavaScript calls these via `WebAssembly.instantiate()` and reads memory directly from the WASM linear memory. Simpler, smaller, faster to load.

## API Reference

### BrowserEngine

| Method | Description |
|--------|-------------|
| `new(config: WasmConfig)` | Create engine |
| `spawn_agent() → Option<usize>` | Add agent, returns ID |
| `connect(a, b)` | Link two agents |
| `tick()` | Advance one step |
| `tick_n(n)` | Advance N steps |
| `agents() → &[Agent]` | Inspect all agents |

### CellEngine

| Method | Description |
|--------|-------------|
| `new_grid(rows, cols, config)` | Create spreadsheet grid |
| `set_cell(row, col, expr)` | Set cell formula |
| `get_cell(row, col) → Tryte` | Read computed value |
| `evaluate_all()` | Recompute entire grid |

### WasmConfig

| Method | Description |
|--------|-------------|
| `default()` | Reasonable defaults |
| `with_seed(seed: u64)` | Custom PRNG seed |

### wasm_bridge

| Type | Description |
|------|-------------|
| `Trit` | Single ternary digit (i8) |
| `Tryte` | 6-trit ternary number |

## Ecosystem Connections

- **`ternary-hardware`** — Hardware simulation for ternary circuits
- **`ternary-circuit`** — Logical circuit design with ternary gates
- **`ternary-compiler`** — Expression compiler (can target this engine)
- **`ternary-transform`** — Data transformation pipeline
- **`ternary-protocol`** — Network protocol for distributed ternary agents

## Open Questions

- **Streaming updates**: Currently `evaluate_all()` recomputes the entire grid. For large spreadsheets, a dirty-flag system that only recomputes changed cells would be much faster.
- **SharedArrayBuffer**: Could use `SharedArrayBuffer` for parallel agent evolution in the browser, but browser support is still inconsistent.
- **Serialization format**: The `wasm_bridge` module handles JS↔Rust conversion, but there's no standard binary format for persisting ternary state. A protobuf or flatbuffers schema would help.
- **GPU compute**: WebGPU is maturing. A WebGPU compute shader for agent evolution would blow the CPU implementation out of the water, but would require a completely different code path.

## License

MIT
