#![no_main]

use chrs_lib::data::{BoardConfig, BoardPiece, Color, Move, Square};
use libfuzzer_sys::fuzz_target;
use std::str::FromStr;

fuzz_target!(|data: Vec<&str>| {
    let mut config = BoardConfig::default();
    for m in data {
        if m.len() >= 4 {
            let chars = m.chars();
            match Square::from_str(&chars.clone().take(2).collect::<String>()) {
                Ok(from) => {
                    match Square::from_str(&chars.clone().skip(2).take(2).collect::<String>()) {
                        Ok(to) => {
                            let mut m = Move::infer(from, to, &config);
                        },
                        Err(_) => ()
                    }
                },
                Err(_) => ()
            }
            
        }
    }
});
