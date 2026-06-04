//! Lightweight ternary evolution engine optimized for WASM.
//!
//! Uses a seed-based XorShift64 PRNG (no std::time, no rand).

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use crate::wasm_bridge::{Trit, Tryte};
use crate::wasm_config::WasmConfig;

/// Simple XorShift64 PRNG — deterministic, no_std friendly.
#[derive(Debug, Clone)]
pub struct Prng {
    state: u64,
}

impl Prng {
    pub fn new(seed: u64) -> Self {
        // Ensure non-zero state
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Generate a random trit.
    pub fn next_trit(&mut self) -> Trit {
        match self.next_u32() % 3 {
            0 => Trit::Neg,
            1 => Trit::Zero,
            _ => Trit::Pos,
        }
    }

    /// Generate a random tryte.
    pub fn next_tryte(&mut self) -> Tryte {
        let mut trits = [Trit::Zero; 6];
        for t in &mut trits {
            *t = self.next_trit();
        }
        Tryte(trits)
    }
}

/// A ternary agent — the fundamental unit of the browser engine.
#[derive(Debug, Clone)]
pub struct TernaryAgent {
    /// Unique agent ID.
    pub id: u32,
    /// Current state as a tryte.
    pub state: Tryte,
    /// Energy level (0..=255).
    pub energy: u8,
    /// Generation counter.
    pub generation: u32,
    /// Connections to other agent IDs.
    pub connections: Vec<u32>,
}

impl TernaryAgent {
    pub fn new(id: u32, state: Tryte) -> Self {
        Self {
            id,
            state,
            energy: 128,
            generation: 0,
            connections: Vec::new(),
        }
    }

    /// Evaluate this agent against a neighbor's state.
    pub fn evaluate(&mut self, neighbor_state: Tryte) {
        let interaction = Trit::add(self.state.0[0], neighbor_state.0[0]).1;
        self.state.0[0] = interaction;
        self.generation += 1;

        // Energy adjustment based on interaction
        match interaction {
            Trit::Pos => {
                self.energy = self.energy.saturating_add(1);
            }
            Trit::Neg => {
                self.energy = self.energy.saturating_sub(1);
            }
            Trit::Zero => {}
        }
    }

    /// Check if agent is still alive.
    pub fn is_alive(&self) -> bool {
        self.energy > 0
    }
}

/// The browser-based ternary evolution engine.
pub struct BrowserEngine {
    agents: Vec<TernaryAgent>,
    prng: Prng,
    config: WasmConfig,
    tick: u64,
}

impl BrowserEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: WasmConfig) -> Self {
        let prng = Prng::new(config.prng_seed);
        Self {
            agents: Vec::with_capacity(config.max_agents),
            prng,
            config,
            tick: 0,
        }
    }

    /// Spawn a new agent with random state.
    pub fn spawn_agent(&mut self) -> Option<u32> {
        if self.agents.len() >= self.config.max_agents {
            return None;
        }
        let id = self.agents.len() as u32;
        let state = self.prng.next_tryte();
        let mut agent = TernaryAgent::new(id, state);
        agent.energy = 128;
        self.agents.push(agent);
        Some(id)
    }

    /// Spawn an agent with a specific state.
    pub fn spawn_agent_with_state(&mut self, state: Tryte) -> Option<u32> {
        if self.agents.len() >= self.config.max_agents {
            return None;
        }
        let id = self.agents.len() as u32;
        self.agents.push(TernaryAgent::new(id, state));
        Some(id)
    }

    /// Get an agent by ID.
    pub fn get_agent(&self, id: u32) -> Option<&TernaryAgent> {
        self.agents.get(id as usize)
    }

    /// Get a mutable agent by ID.
    pub fn get_agent_mut(&mut self, id: u32) -> Option<&mut TernaryAgent> {
        self.agents.get_mut(id as usize)
    }

    /// Connect two agents.
    pub fn connect(&mut self, a: u32, b: u32) -> bool {
        if a as usize >= self.agents.len() || b as usize >= self.agents.len() {
            return false;
        }
        if !self.agents[a as usize].connections.contains(&b) {
            self.agents[a as usize].connections.push(b);
        }
        if !self.agents[b as usize].connections.contains(&a) {
            self.agents[b as usize].connections.push(a);
        }
        true
    }

    /// Run one evolution tick: each agent interacts with its connections.
    pub fn tick(&mut self) {
        // Collect interaction pairs to avoid borrow issues
        let interactions: Vec<(u32, Tryte)> = {
            let mut pairs = Vec::new();
            for agent in &self.agents {
                if let Some(&neighbor_id) = agent.connections.first() {
                    if let Some(neighbor) = self.agents.get(neighbor_id as usize) {
                        pairs.push((agent.id, neighbor.state));
                    }
                }
            }
            pairs
        };

        for (agent_id, neighbor_state) in interactions {
            if let Some(agent) = self.agents.get_mut(agent_id as usize) {
                agent.evaluate(neighbor_state);
            }
        }
        self.tick += 1;
    }

    /// Run N ticks.
    pub fn tick_n(&mut self, n: u32) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Current tick count.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Number of alive agents.
    pub fn alive_count(&self) -> usize {
        self.agents.iter().filter(|a| a.is_alive()).count()
    }

    /// Total agent count (including dead).
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Collect all agent states as trytes.
    pub fn collect_states(&self) -> Vec<Tryte> {
        self.agents.iter().map(|a| a.state).collect()
    }

    /// Compute a simple hash of the engine state (for verification).
    pub fn state_hash(&self) -> u64 {
        let mut hash: u64 = self.tick;
        for agent in &self.agents {
            hash ^= (agent.id as u64).wrapping_mul(0x9E3779B97F4A7C15);
            hash ^= (agent.state.to_i32() as u64).wrapping_shl(16);
            hash ^= agent.energy as u64;
        }
        hash
    }
}
