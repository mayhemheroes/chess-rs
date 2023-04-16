#![no_main]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{self, Arbitrary};
use chrs_lib::data::{BoardConfig, BoardPiece, Color, Move, Square};

#[derive(Arbitrary, Copy, Clone, Debug)]
enum StartingText<'a> {
    Custom(&'a str),
}

fuzz_target!(|data: StartingText| {
    let fen = match data {
        StartingText::Custom(s) => s,
    };
    
    let mut config = BoardConfig::from_fen_str(&fen);
});
