/*
 *  ========================================================================
 *  DBCE chess bot, plain board representation
 *  ========================================================================
 *
 *  This file is part of DBCE.
 *
 *  DBCE is free software: you can redistribute it and/or
 *  modify it under the terms of the GNU General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or (at
 *  your option) any later version.
 *
 *  DBCE is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *  General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License along
 *  with DBCE.  If not, see <http://www.gnu.org/licenses/>.
 *
 *  (C) Copyright 2022-3, Gabor Kecskemeti
 */

use crate::baserules::board_rep::BoardPos;
use crate::baserules::piece_color::PieceColor::{Black, White};
use crate::baserules::piece_kind::PieceKind::{Bishop, King, Knight, Pawn, Queen, Rook};
use crate::baserules::piece_state::PieceState;
use lazy_static::lazy_static;
use std::ops;

pub static MATE: f32 = 1000.0;

pub fn is_mate(score: f32) -> bool {
    (score.abs() - MATE).abs() < 50.0
}

#[derive(Clone, Copy)]
pub struct RawBoard([u32; 8]);

impl RawBoard {
    #[inline]
    pub fn empty() -> Self {
        RawBoard([0; 8])
    }
    #[inline]
    pub fn set_loc(&mut self, BoardPos(row, col): BoardPos, piece: &Option<PieceState>) {
        let shift_amount = col << 2;
        let piece_mask = !(0b1111 << shift_amount);
        let us_row = row as usize;
        let a_row = self.0[us_row] & piece_mask;
        let a_bit_piece = (PieceState::bits(piece) as u32) << shift_amount;
        self.0[us_row] = a_row | a_bit_piece;
    }

    /// A simple scoring mechanism which just counts up the pieces and pawns based on their usual values
    ///
    /// # Example use
    /// Each `PSBoard` has its score automatically calculated with this method during creation, so this is an indirect demonstration.
    /// ```
    /// use dbce::baserules::board::PSBoard;
    /// use dbce::baserules::rawboard::{MATE};
    /// let scholars_mate = PSBoard::from_fen("1rbqQb1r/pppp2pp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b QKqk - 9 5");
    /// assert_eq!(MATE, scholars_mate.board.score());
    /// ```
    pub fn score(&self) -> f32 {
        let (loc_score, white_king_found, black_king_found) = self
            .into_iter()
            .filter_map(|c_p| *c_p)
            .map(|curr_piece| match (curr_piece.kind, curr_piece.color) {
                (Pawn, White) => (1f32, false, false),
                (Pawn, Black) => (-1f32, false, false),
                (Knight, White) => (3f32, false, false),
                (Knight, Black) => (-3f32, false, false),
                (Bishop, White) => (3.1f32, false, false),
                (Bishop, Black) => (-3.1f32, false, false),
                (Rook, White) => (5f32, false, false),
                (Rook, Black) => (-5f32, false, false),
                (Queen, White) => (9f32, false, false),
                (Queen, Black) => (-9f32, false, false),
                (King, White) => (0f32, true, false),
                (King, Black) => (0f32, false, true),
            })
            .fold(
                (0f32, false, false),
                |(curr_score, curr_white_king, curr_black_king),
                 (score_adjust, white_king, black_king)| {
                    (
                        curr_score + score_adjust,
                        curr_white_king | white_king,
                        curr_black_king | black_king,
                    )
                },
            );
        if white_king_found {
            if black_king_found {
                loc_score
            } else {
                MATE
            }
        } else {
            -MATE
        }
    }
}

impl ops::Index<BoardPos> for RawBoard {
    type Output = Option<PieceState>;

    #[inline]
    fn index(&self, BoardPos(row, col): BoardPos) -> &Self::Output {
        let a_row = self.0[row as usize];
        let a_bit_piece = a_row >> (col << 2) & 0b1111;
        PieceState::from_u8(a_bit_piece)
    }
}
impl<'a> IntoIterator for &'a RawBoard {
    type Item = &'a Option<PieceState>;
    type IntoIter = RawBoardIterator<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        RawBoardIterator {
            raw_board: self,
            curr_idx: 0,
        }
    }
}
lazy_static! {
    static ref ALL_POSSIBLE_MASKS: [u32; (u32::BITS / 4) as usize] = generate_masks();
}

fn generate_masks() -> [u32; (u32::BITS / 4) as usize] {
    let mut mask_permutations = [0; (u32::BITS / 4) as usize];

    for (index, shift) in (0..u32::BITS).step_by(4).enumerate() {
        let n: u32 = 0xF << shift;
        mask_permutations[index] = n;
    }

    mask_permutations
}

pub struct RawBoardIterator<'a> {
    raw_board: &'a RawBoard,
    curr_idx: usize,
}

impl<'a> Iterator for RawBoardIterator<'a> {
    type Item = &'a Option<PieceState>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_idx == 64 {
            None
        } else {
            unsafe {
                let ret = PieceState::from_u8(
                    ALL_POSSIBLE_MASKS.get_unchecked(self.curr_idx & 0b111)
                        & (*self.raw_board.0.get_unchecked(self.curr_idx >> 3)),
                );

                self.curr_idx += 1;
                Some(ret)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remainder = 64 - self.curr_idx;
        (remainder, Some(remainder))
    }
}

impl<'a> ExactSizeIterator for RawBoardIterator<'a> {}

#[cfg(test)]
mod test {
    use crate::baserules::board::PSBoard;
    use crate::baserules::board_rep::BoardPos;
    use std::str::FromStr;

    use super::generate_masks;

    #[test]
    fn iterator_test() {
        let psboard = PSBoard::default();
        let mut iter = psboard.board.into_iter();
        assert_eq!(
            psboard.get_loc(BoardPos::from_str("a1").unwrap()),
            iter.next().unwrap()
        );
        let mut iter = psboard.board.into_iter();
        assert_eq!(
            psboard.get_loc(BoardPos::from_str("d8").unwrap()),
            iter.nth(8 * 7 + 3).unwrap()
        );
    }

    #[test]
    pub fn test_mask_generator() {
        // [0b1111, 0b11110000, 0b111100000000, 0b1111000000000000, ...];
        let masks = generate_masks();
        assert_eq!(masks[0], 0b1111);
        assert_eq!(masks[1], 0b11110000);
        assert_eq!(masks[2], 0b111100000000);
        assert_eq!(masks[3], 0b1111000000000000);
        assert_eq!(masks[7], 0b11110000000000000000000000000000);
    }
}
