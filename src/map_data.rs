use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum SetPieceFeature {
    Rng,
    Tooth,
    Chest,
}

// (rows, cols)
pub fn get_piece_sizes(set_size: i32) -> (i32, i32) {
    include!(concat!(env!("OUT_DIR"), "/set_piece_sizes.rs"))
}

// index is (size, row, col)
//pub fn get_piece_events() -> HashMap<(i32, i32, i32), &'static [SetPieceFeature]> {
include!(concat!(env!("OUT_DIR"), "/set_piece_events.rs"));
//}

pub static LAYOUTS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/layouts.bin"));
