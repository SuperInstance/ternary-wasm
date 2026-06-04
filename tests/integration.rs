//! Tests

#[cfg(test)]
mod tests {
    use ternary_wasm::wasm_bridge::{Trit, Tryte, WasmBridge};
    use ternary_wasm::browser_engine::{BrowserEngine, Prng};
    use ternary_wasm::cell_engine::{CellEngine, CellExpr};
    use ternary_wasm::wasm_config::WasmConfig;

    // --- WasmConfig tests ---

    #[test]
    fn test_default_config_is_valid() {
        assert!(WasmConfig::default().is_valid());
    }

    #[test]
    fn test_config_with_seed() {
        let cfg = WasmConfig::with_seed(42);
        assert_eq!(cfg.prng_seed, 42);
        assert!(cfg.is_valid());
    }

    #[test]
    fn test_config_bytes_per_agent() {
        let cfg = WasmConfig::default();
        let bpa = cfg.bytes_per_agent();
        assert!(bpa > 0);
        assert_eq!(bpa, cfg.memory_limit / cfg.max_agents);
    }

    // --- Trit tests ---

    #[test]
    fn test_trit_from_i8() {
        assert_eq!(Trit::from_i8(-5), Trit::Neg);
        assert_eq!(Trit::from_i8(-1), Trit::Neg);
        assert_eq!(Trit::from_i8(0), Trit::Zero);
        assert_eq!(Trit::from_i8(1), Trit::Pos);
        assert_eq!(Trit::from_i8(100), Trit::Pos);
    }

    #[test]
    fn test_trit_mul() {
        assert_eq!(Trit::Pos.mul(Trit::Pos), Trit::Pos);
        assert_eq!(Trit::Neg.mul(Trit::Neg), Trit::Pos);
        assert_eq!(Trit::Pos.mul(Trit::Neg), Trit::Neg);
        assert_eq!(Trit::Zero.mul(Trit::Pos), Trit::Zero);
    }

    #[test]
    fn test_trit_add() {
        let (carry, sum) = Trit::Pos.add(Trit::Pos);
        assert_eq!(sum, Trit::Neg);
        assert_eq!(carry, Trit::Pos);

        let (carry, sum) = Trit::Pos.add(Trit::Neg);
        assert_eq!(sum, Trit::Zero);
        assert_eq!(carry, Trit::Zero);
    }

    // --- Tryte tests ---

    #[test]
    fn test_tryte_roundtrip() {
        for v in &[-364i32, -300, -100, -1, 0, 1, 42, 100, 300, 364] {
            let tryte = Tryte::from_i32(*v);
            assert_eq!(tryte.to_i32(), *v, "roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_tryte_zero() {
        assert_eq!(Tryte::ZERO.to_i32(), 0);
    }

    #[test]
    fn test_tryte_clamp() {
        // Out of range values should clamp to +/-364
        let t = Tryte::from_i32(500);
        assert!(t.to_i32() <= 364);
        let t = Tryte::from_i32(-500);
        assert!(t.to_i32() >= -364);
    }

    // --- WasmBridge tests ---

    #[test]
    fn test_serialize_deserialize_trits() {
        let trits = vec![Trit::Neg, Trit::Zero, Trit::Pos, Trit::Neg, Trit::Zero];
        let serialized = WasmBridge::serialize_trits(&trits);
        let deserialized = WasmBridge::deserialize_trits(&serialized, trits.len());
        assert_eq!(deserialized, trits);
    }

    #[test]
    fn test_string_to_trytes_roundtrip() {
        let original = "Hello";
        let trytes = WasmBridge::string_to_trytes(original);
        let recovered = WasmBridge::trytes_to_string(&trytes);
        assert_eq!(recovered, original);
    }

    // --- PRNG tests ---

    #[test]
    fn test_prng_deterministic() {
        let mut a = Prng::new(42);
        let mut b = Prng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_prng_nonzero_seed() {
        let mut p = Prng::new(0);
        assert_ne!(p.next_u64(), 0); // seed 0 → state 1, should produce output
    }

    // --- BrowserEngine tests ---

    #[test]
    fn test_engine_spawn_and_tick() {
        let config = WasmConfig::with_seed(12345);
        let mut engine = BrowserEngine::new(config);

        let a = engine.spawn_agent().unwrap();
        let b = engine.spawn_agent().unwrap();
        engine.connect(a, b);

        engine.tick();
        assert_eq!(engine.current_tick(), 1);
        assert_eq!(engine.agent_count(), 2);
    }

    #[test]
    fn test_engine_tick_n() {
        let mut engine = BrowserEngine::new(WasmConfig::with_seed(99));
        engine.spawn_agent().unwrap();
        engine.tick_n(10);
        assert_eq!(engine.current_tick(), 10);
    }

    #[test]
    fn test_engine_agent_alive() {
        let mut engine = BrowserEngine::new(WasmConfig::with_seed(1));
        engine.spawn_agent().unwrap();
        assert_eq!(engine.alive_count(), 1);
    }

    #[test]
    fn test_engine_max_agents() {
        let config = WasmConfig { max_agents: 3, ..WasmConfig::default() };
        let mut engine = BrowserEngine::new(config);
        assert!(engine.spawn_agent().is_some());
        assert!(engine.spawn_agent().is_some());
        assert!(engine.spawn_agent().is_some());
        assert!(engine.spawn_agent().is_none()); // 4th should fail
    }

    // --- CellEngine tests ---

    #[test]
    fn test_cell_literal() {
        let mut engine = CellEngine::new_grid(3, 3, &WasmConfig::default());
        engine.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(42)));
        engine.evaluate_all();
        assert_eq!(engine.get_value(0, 0).to_i32(), 42);
    }

    #[test]
    fn test_cell_sum() {
        let mut engine = CellEngine::new_grid(3, 3, &WasmConfig::default());
        engine.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(10)));
        engine.set_cell(0, 1, CellExpr::Literal(Tryte::from_i32(20)));
        engine.set_cell(0, 2, CellExpr::Sum((0, 0), (0, 1)));
        engine.evaluate_all();
        assert_eq!(engine.get_value(0, 2).to_i32(), 30);
    }

    #[test]
    fn test_cell_product() {
        let mut engine = CellEngine::new_grid(3, 3, &WasmConfig::default());
        engine.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(6)));
        engine.set_cell(0, 1, CellExpr::Literal(Tryte::from_i32(7)));
        engine.set_cell(0, 2, CellExpr::Product((0, 0), (0, 1)));
        engine.evaluate_all();
        assert_eq!(engine.get_value(0, 2).to_i32(), 42);
    }

    #[test]
    fn test_cell_threshold() {
        let mut engine = CellEngine::new_grid(3, 3, &WasmConfig::default());
        engine.set_cell(0, 0, CellExpr::Literal(Tryte::from_i32(10)));
        engine.set_cell(0, 1, CellExpr::Literal(Tryte::from_i32(5)));
        engine.set_cell(0, 2, CellExpr::Threshold((0, 0), (0, 1)));
        engine.evaluate_all();
        assert_eq!(engine.get_value(0, 2).to_i32(), 1); // 10 > 5 → Pos
    }

    // --- Export bindings test ---

    #[test]
    fn test_export_init_and_spawn() {
        ternary_wasm::export_bindings::ternary_init(42);
        let id = ternary_wasm::export_bindings::ternary_spawn_agent();
        assert!(id >= 0);
        assert_eq!(ternary_wasm::export_bindings::ternary_agent_count(), 1);
        ternary_wasm::export_bindings::ternary_tick(5);
        assert_eq!(ternary_wasm::export_bindings::ternary_current_tick(), 5);
    }
}
