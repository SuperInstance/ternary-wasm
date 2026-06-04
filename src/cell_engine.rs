//! Spreadsheet cell evaluation engine.
//!
//! Each cell is a tiny ternary agent that can reference other cells,
//! evaluate expressions, and propagate updates.

use crate::wasm_bridge::Tryte;
use crate::wasm_config::WasmConfig;

/// A cell reference (row, col).
pub type CellRef = (u32, u32);

/// Types of expressions a cell can contain.
#[derive(Debug, Clone)]
pub enum CellExpr {
    /// Literal tryte value.
    Literal(Tryte),
    /// Reference to another cell.
    Ref(CellRef),
    /// Sum of two cell references.
    Sum(CellRef, CellRef),
    /// Product of two cell references.
    Product(CellRef, CellRef),
    /// Threshold gate: if first ref > second ref, produce Pos; if < Neg; else Zero.
    Threshold(CellRef, CellRef),
}

/// A spreadsheet cell — a tiny ternary agent.
#[derive(Debug, Clone)]
pub struct Cell {
    pub row: u32,
    pub col: u32,
    pub expr: CellExpr,
    pub cached_value: Tryte,
    pub dirty: bool,
    pub generation: u32,
}

impl Cell {
    pub fn new(row: u32, col: u32, expr: CellExpr) -> Self {
        Self {
            row,
            col,
            expr,
            cached_value: Tryte::ZERO,
            dirty: true,
            generation: 0,
        }
    }

    pub fn ref_key(&self) -> CellRef {
        (self.row, self.col)
    }
}

/// The spreadsheet cell evaluation engine.
pub struct CellEngine {
    cells: Vec<Cell>,
    max_eval_depth: u32,
    rows: u32,
    cols: u32,
}

impl CellEngine {
    pub fn new(config: &WasmConfig) -> Self {
        Self {
            cells: Vec::new(),
            max_eval_depth: config.max_eval_depth,
            rows: 0,
            cols: 0,
        }
    }

    /// Create a grid of the given dimensions, initialized to zero.
    pub fn new_grid(rows: u32, cols: u32, config: &WasmConfig) -> Self {
        let mut engine = Self::new(config);
        engine.rows = rows;
        engine.cols = cols;
        for r in 0..rows {
            for c in 0..cols {
                engine.cells.push(Cell::new(r, c, CellExpr::Literal(Tryte::ZERO)));
            }
        }
        engine
    }

    /// Find a cell by reference.
    fn find_cell(&self, key: CellRef) -> Option<usize> {
        self.cells.iter().position(|c| c.ref_key() == key)
    }

    /// Set a cell's expression.
    pub fn set_cell(&mut self, row: u32, col: u32, expr: CellExpr) {
        if let Some(idx) = self.find_cell((row, col)) {
            self.cells[idx].expr = expr;
            self.cells[idx].dirty = true;
            self.cells[idx].generation += 1;
        } else {
            self.cells.push(Cell::new(row, col, expr));
        }
    }

    /// Get a cell's cached value.
    pub fn get_value(&self, row: u32, col: u32) -> Tryte {
        self.find_cell((row, col))
            .map(|i| self.cells[i].cached_value)
            .unwrap_or(Tryte::ZERO)
    }

    /// Evaluate a single expression, returning its tryte value.
    fn eval_expr(&self, expr: &CellExpr, depth: u32) -> Tryte {
        if depth > self.max_eval_depth {
            return Tryte::ZERO; // guard against infinite recursion
        }

        match expr {
            CellExpr::Literal(t) => *t,
            CellExpr::Ref(key) => {
                self.find_cell(*key)
                    .map(|i| self.eval_expr(&self.cells[i].expr, depth + 1))
                    .unwrap_or(Tryte::ZERO)
            }
            CellExpr::Sum(a, b) => {
                let va = self.eval_expr(&CellExpr::Ref(*a), depth + 1).to_i32();
                let vb = self.eval_expr(&CellExpr::Ref(*b), depth + 1).to_i32();
                Tryte::from_i32(va + vb)
            }
            CellExpr::Product(a, b) => {
                let va = self.eval_expr(&CellExpr::Ref(*a), depth + 1).to_i32();
                let vb = self.eval_expr(&CellExpr::Ref(*b), depth + 1).to_i32();
                Tryte::from_i32(va * vb)
            }
            CellExpr::Threshold(a, b) => {
                let va = self.eval_expr(&CellExpr::Ref(*a), depth + 1).to_i32();
                let vb = self.eval_expr(&CellExpr::Ref(*b), depth + 1).to_i32();
                if va > vb {
                    Tryte::from_i32(1)
                } else if va < vb {
                    Tryte::from_i32(-1)
                } else {
                    Tryte::ZERO
                }
            }
        }
    }

    /// Recompute all dirty cells.
    pub fn evaluate_all(&mut self) {
        // First pass: evaluate all cells
        let new_values: Vec<(usize, Tryte)> = self.cells
            .iter()
            .enumerate()
            .filter(|(_, c)| c.dirty)
            .map(|(i, c)| (i, self.eval_expr(&c.expr, 0)))
            .collect();

        // Second pass: update caches
        for (idx, val) in new_values {
            self.cells[idx].cached_value = val;
            self.cells[idx].dirty = false;
        }
    }

    /// Mark all cells as dirty.
    pub fn mark_all_dirty(&mut self) {
        for cell in &mut self.cells {
            cell.dirty = true;
        }
    }

    /// Number of cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Export all cell values as a flat vector of trytes.
    pub fn export_values(&self) -> Vec<Tryte> {
        self.cells.iter().map(|c| c.cached_value).collect()
    }

    /// Import values from a flat vector, filling cells in row-major order.
    pub fn import_values(&mut self, values: &[Tryte]) {
        for (i, &val) in values.iter().enumerate() {
            if i < self.cells.len() {
                self.cells[i].expr = CellExpr::Literal(val);
                self.cells[i].dirty = true;
            }
        }
    }
}
