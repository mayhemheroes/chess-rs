pub mod bitboard;
mod fen;
mod moves;
pub mod piece;
mod square;

use fen::Fen;
use moves::CastleType;
use std::str::FromStr;
use strum::IntoEnumIterator;

pub use bitboard::BitBoard;
pub use moves::{Move, MoveCommit, MoveHistory, MoveList, MoveType};
pub use piece::{BoardPiece, Color};
pub use square::Square;

pub type BoardMap = [BitBoard; 12];

#[derive(Debug)]
pub struct BoardConfig {
    active_color: Color,
    en_passant_target: Option<Square>,
    castle_flags: CastleFlags,
    halfmove_clock: u32,
    fullmove_number: u32,
    pub bitboards: BoardMap,
    move_history: MoveHistory,
}

impl Default for BoardConfig {
    fn default() -> Self {
        Fen::make_config_from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }
}

impl BoardConfig {
    pub fn apply_move(&mut self, m: Move) {
        log::info!("{:?}", m);
        let p = self.get_at_sq(m.from).unwrap();
        let pcolor = p.get_color();

        // prevent from moving when its not their turn
        if pcolor != self.active_color {
            return;
        }

        let prev_ep_target = self.en_passant_target;
        let prev_castle_flags = self.castle_flags;

        use MoveType::*;
        let cap = match m.move_type {
            Normal => self.apply_normal(m.from, m.to),
            DoublePush => self.apply_double_push(m.from, m.to, p),
            EnPassant => self.apply_en_passant(m.from, m.to, p),
            Castle(castle_type) => self.apply_castle(p, castle_type),
            Promotion(prom) => {
                if let Some(prom) = prom {
                    self.apply_promotion(m.from, m.to, prom)
                } else {
                    log::error!("Promotion Move has no promotion piece assigned to it");
                    panic!();
                }
            }
        };

        // en passant state update
        if m.move_type != DoublePush {
            self.en_passant_target = None;
        }
        // castling state update
        if p == BoardPiece::WhiteRook {
            if m.from == Square::A1 {
                self.castle_flags.unset_white_ooo();
            } else if m.from == Square::H1 {
                self.castle_flags.unset_white_oo();
            }
        } else if p == BoardPiece::BlackRook {
            if m.from == Square::A8 {
                self.castle_flags.unset_black_ooo();
            } else if m.from == Square::H8 {
                self.castle_flags.unset_black_oo();
            }
        } else if p == BoardPiece::WhiteKing {
            self.castle_flags.unset_white_oo();
            self.castle_flags.unset_white_ooo();
        } else if p == BoardPiece::BlackKing {
            self.castle_flags.unset_black_oo();
            self.castle_flags.unset_black_ooo();
        }

        if pcolor == Color::Black {
            self.fullmove_number += 1;
        }
        let castledelta = self.castle_flags.0 ^ prev_castle_flags.0;
        self.halfmove_clock += 1;
        self.toggle_active_color();
        // self.fen_str = Fen::make_fen_from_config(self);
        self.move_history.push(MoveCommit::new(
            m,
            cap,
            prev_ep_target,
            CastleFlags(castledelta),
        ));
    }

    fn apply_normal(&mut self, from: Square, to: Square) -> Option<BoardPiece> {
        let cap = self.remove_piece(to);
        self.move_piece(from, to);
        cap
    }

    fn apply_double_push(&mut self, from: Square, to: Square, p: BoardPiece) -> Option<BoardPiece> {
        let pcolor = p.get_color();
        self.remove_piece(to);
        self.move_piece(from, to);
        if pcolor == Color::White {
            self.en_passant_target = Some(Square::try_from(to as usize - 8).unwrap());
        } else {
            self.en_passant_target = Some(Square::try_from(to as usize + 8).unwrap());
        }
        None
    }

    fn apply_en_passant(&mut self, from: Square, to: Square, p: BoardPiece) -> Option<BoardPiece> {
        let pcolor = p.get_color();
        // self.remove_piece(to);
        self.move_piece(from, to);
        if pcolor == Color::White {
            self.remove_piece(Square::try_from(to as usize - 8).unwrap())
        } else {
            self.remove_piece(Square::try_from(to as usize + 8).unwrap())
        }
    }

    fn apply_castle(&mut self, p: BoardPiece, castle_type: CastleType) -> Option<BoardPiece> {
        let pcolor = p.get_color();
        match castle_type {
            CastleType::KingSide => {
                if pcolor == Color::White {
                    self.move_piece(Square::E1, Square::G1);
                    self.move_piece(Square::H1, Square::F1);
                }
                if pcolor == Color::Black {
                    self.move_piece(Square::E8, Square::G8);
                    self.move_piece(Square::H8, Square::F8);
                }
            }
            CastleType::QueenSide => {
                if pcolor == Color::White {
                    self.move_piece(Square::E1, Square::C1);
                    self.move_piece(Square::A1, Square::D1);
                }
                if pcolor == Color::Black {
                    self.move_piece(Square::E8, Square::C8);
                    self.move_piece(Square::A8, Square::D8);
                }
            }
        }

        match pcolor {
            Color::White => {
                self.castle_flags.unset_white_oo();
                self.castle_flags.unset_white_ooo();
            }
            Color::Black => {
                self.castle_flags.unset_black_oo();
                self.castle_flags.unset_black_ooo();
            }
        }

        None
    }

    fn apply_promotion(
        &mut self,
        from: Square,
        to: Square,
        prom: BoardPiece,
    ) -> Option<BoardPiece> {
        self.remove_piece(from);
        let cap = self.get_at_sq(to);
        self.add_piece(prom, to);
        cap
    }

    pub fn undo(&mut self) {
        if let Some(commit) = self.move_history.pop() {
            let m = commit.m;
            let cap = commit.captured;
            let p = self.get_at_sq(m.to).unwrap();
            let pcolor = p.get_color();

            use MoveType::*;
            match m.move_type {
                Normal => self.undo_normal(m.from, m.to, cap),
                DoublePush => self.undo_double_push(m.from, m.to),
                EnPassant => self.undo_en_passant(m.from, m.to, p, cap),
                Castle(castle_type) => self.undo_castle(p, castle_type),
                Promotion(prom) => {
                    if let Some(_) = prom {
                        self.undo_promotion(m.from, m.to, p, cap);
                    } else {
                        log::error!("Promotion Move has no promotion piece assigned to it");
                        panic!();
                    }
                }
            }

            if pcolor == Color::Black {
                self.fullmove_number -= 1;
            }
            self.en_passant_target = commit.ep_target;
            self.castle_flags.0 ^= commit.castledelta.0;
            self.halfmove_clock -= 1;
            self.toggle_active_color();
            // self.fen_str = Fen::make_fen_from_config(self);
        }
    }

    fn undo_normal(&mut self, from: Square, to: Square, cap: Option<BoardPiece>) {
        self.move_piece(to, from);
        if let Some(cap) = cap {
            self.add_piece(cap, to);
        }
    }

    fn undo_double_push(&mut self, from: Square, to: Square) {
        self.move_piece(to, from);
    }

    fn undo_castle(&mut self, p: BoardPiece, castle_type: CastleType) {
        match p.get_color() {
            Color::White => match castle_type {
                CastleType::KingSide => {
                    self.move_piece(Square::G1, Square::E1);
                    self.move_piece(Square::F1, Square::H1);
                }
                CastleType::QueenSide => {
                    self.move_piece(Square::C1, Square::E1);
                    self.move_piece(Square::D1, Square::A1);
                }
            },
            Color::Black => match castle_type {
                CastleType::KingSide => {
                    self.move_piece(Square::G8, Square::E8);
                    self.move_piece(Square::F8, Square::H8);
                }
                CastleType::QueenSide => {
                    self.move_piece(Square::C8, Square::E8);
                    self.move_piece(Square::D8, Square::A8);
                }
            },
        }
    }

    fn undo_promotion(&mut self, from: Square, to: Square, p: BoardPiece, cap: Option<BoardPiece>) {
        self.remove_piece(to);
        match p.get_color() {
            Color::White => self.add_piece(BoardPiece::WhitePawn, from),
            Color::Black => self.add_piece(BoardPiece::BlackPawn, from),
        }
        if let Some(cap) = cap {
            self.add_piece(cap, to);
        }
    }

    fn undo_en_passant(
        &mut self,
        from: Square,
        to: Square,
        p: BoardPiece,
        cap: Option<BoardPiece>,
    ) {
        self.move_piece(to, from);
        if let Some(cap) = cap {
            let cap_sq = if p.get_color() == Color::White {
                Square::try_from(to as usize - 8).unwrap()
            } else {
                Square::try_from(to as usize + 8).unwrap()
            };
            self.add_piece(cap, cap_sq);
        }
    }

    pub fn reset(&mut self) {
        *self = BoardConfig::default();
    }

    pub fn from_fen_str(s: &str) -> Self {
        Fen::make_config_from_str(s)
    }

    pub fn load_fen(&mut self, s: &str) {
        *self = Fen::make_config_from_str(s);
    }

    pub fn get_fen(&self) -> String {
        Fen::make_fen_from_config(self)
    }

    pub fn get_at_sq(&self, sq: Square) -> Option<BoardPiece> {
        for piece in BoardPiece::iter() {
            if self.bitboards[piece as usize].is_set(sq) {
                return Some(piece);
            }
        }
        None
    }

    pub fn get_active_color(&self) -> Color {
        self.active_color
    }

    pub fn get_can_white_castle_queenside(&self) -> bool {
        self.castle_flags.can_white_ooo()
    }

    pub fn get_can_white_castle_kingside(&self) -> bool {
        self.castle_flags.can_white_oo()
    }

    pub fn get_can_black_castle_queenside(&self) -> bool {
        self.castle_flags.can_black_ooo()
    }

    pub fn get_can_black_castle_kingside(&self) -> bool {
        self.castle_flags.can_black_oo()
    }

    pub fn get_en_passant_target(&self) -> Option<Square> {
        self.en_passant_target
    }

    pub fn get_halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    pub fn get_fullmove_number(&self) -> u32 {
        self.fullmove_number
    }

    pub fn get_bit_board(&self, c: char) -> Option<BitBoard> {
        if let Ok(p) = BoardPiece::from_str(&c.to_string()) {
            return Some(self.bitboards[p as usize]);
        }
        None
    }

    pub fn get_piece_occupancy(&self, p: BoardPiece) -> BitBoard {
        self.bitboards[p as usize]
    }

    pub fn all_occupancy(&self) -> BitBoard {
        let mut ret = BitBoard::from(0);
        for bb in self.bitboards.iter() {
            ret |= *bb;
        }
        ret
    }

    pub fn white_occupancy(&self) -> BitBoard {
        let mut ret = BitBoard::from(0);
        use BoardPiece::*;
        ret |= self.bitboards[WhiteRook as usize]
            | self.bitboards[WhiteBishop as usize]
            | self.bitboards[WhiteKnight as usize]
            | self.bitboards[WhiteKing as usize]
            | self.bitboards[WhiteQueen as usize]
            | self.bitboards[WhitePawn as usize];
        ret
    }

    pub fn black_occupancy(&self) -> BitBoard {
        let mut ret = BitBoard::from(0);
        use BoardPiece::*;
        ret |= self.bitboards[BlackRook as usize]
            | self.bitboards[BlackBishop as usize]
            | self.bitboards[BlackKnight as usize]
            | self.bitboards[BlackKing as usize]
            | self.bitboards[BlackQueen as usize]
            | self.bitboards[BlackPawn as usize];
        ret
    }

    fn move_piece(&mut self, from: Square, to: Square) {
        let p = self.get_at_sq(from).unwrap();
        self.remove_piece(from);
        self.add_piece(p, to);
    }

    fn remove_piece(&mut self, from: Square) -> Option<BoardPiece> {
        if let Some(p) = self.get_at_sq(from) {
            self.remove_from_bitboard(p, from);
            Some(p)
        } else {
            None
        }
    }

    fn add_piece(&mut self, p: BoardPiece, to: Square) {
        self.add_to_bitboard(p, to)
    }

    fn toggle_active_color(&mut self) {
        self.active_color = match self.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    fn update_bitboard(&mut self, p: BoardPiece, prev: Square, new: Square) {
        self.bitboards[p as usize].make_move(prev, new);
    }

    fn remove_from_bitboard(&mut self, p: BoardPiece, pos: Square) {
        self.bitboards[p as usize].unset(pos);
    }

    fn add_to_bitboard(&mut self, p: BoardPiece, pos: Square) {
        self.bitboards[p as usize].set(pos);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CastleFlags(u8);

impl CastleFlags {
    pub fn can_white_oo(&self) -> bool {
        self.0 & 1 > 0
    }

    pub fn set_white_oo(&mut self) {
        self.0 |= 1;
    }

    pub fn unset_white_oo(&mut self) {
        self.0 &= !(1);
    }

    pub fn can_white_ooo(&self) -> bool {
        self.0 & (1 << 1) > 0
    }

    pub fn set_white_ooo(&mut self) {
        self.0 |= 1 << 1;
    }

    pub fn unset_white_ooo(&mut self) {
        self.0 &= !(1 << 1);
    }

    pub fn can_black_oo(&self) -> bool {
        self.0 & (1 << 2) > 0
    }

    pub fn set_black_oo(&mut self) {
        self.0 |= 1 << 2;
    }

    pub fn unset_black_oo(&mut self) {
        self.0 &= !(1 << 2);
    }

    pub fn can_black_ooo(&self) -> bool {
        self.0 & (1 << 3) > 0
    }

    pub fn set_black_ooo(&mut self) {
        self.0 |= 1 << 3;
    }

    pub fn unset_black_ooo(&mut self) {
        self.0 &= !(1 << 3);
    }
}
