//! WASM-specific configuration (memory limits, stack size, etc.)

/// Configuration for the WASM ternary engine.
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Maximum number of ternary agents (cells) the engine can hold.
    pub max_agents: usize,
    /// Maximum memory in bytes the engine may allocate.
    pub memory_limit: usize,
    /// Stack size hint for the WASM module (used by the JS host).
    pub stack_size: usize,
    /// Seed for the deterministic PRNG.
    pub prng_seed: u64,
    /// Maximum evaluation depth for cell recursion.
    pub max_eval_depth: u32,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_agents: 10_000,
            memory_limit: 16 * 1024 * 1024, // 16 MB
            stack_size: 64 * 1024,          // 64 KB
            prng_seed: 0xDEADBEEF_CAFEBABE,
            max_eval_depth: 64,
        }
    }
}

impl WasmConfig {
    /// Create a new config with a specific PRNG seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            prng_seed: seed,
            ..Self::default()
        }
    }

    /// Validate configuration; returns true if all values are within sane bounds.
    pub fn is_valid(&self) -> bool {
        self.max_agents > 0
            && self.max_agents <= 1_000_000
            && self.memory_limit >= 1024
            && self.stack_size >= 1024
            && self.max_eval_depth > 0
            && self.max_eval_depth <= 1024
    }

    /// Estimate memory usage per agent in bytes (rough).
    pub fn bytes_per_agent(&self) -> usize {
        self.memory_limit / self.max_agents.max(1)
    }
}
