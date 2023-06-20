use crate::baserules::board::PSBoard;
use crate::baserules::board_rep::PossibleMove;
use rand::{thread_rng, Rng};

use generational_arena::Arena;
use itertools::Itertools;
use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct BoardContinuation {
    pub board: Arc<PSBoard>,
    /// The overall expected score of this board after considering the continuations
    pub adjusted_score: f32,
    /// If we have calculated a few positions ahead from this board, we store these positions here
    continuation: Arena<(PossibleMove, BoardContinuation)>,
}

impl Default for BoardContinuation {
    /// Produces a board continuation with the starting board
    ///
    /// # Example use:
    /// ```
    /// use dbce::baserules::board::PSBoard;
    /// use dbce::baserules::rawboard::RawBoard;
    /// use dbce::engine::continuation::BoardContinuation;
    /// let starting_position = BoardContinuation::default();
    /// assert_eq!(starting_position.raw, PSBoard::default().raw);
    /// assert_eq!(0, starting_position.total_continuation_boards());
    /// ```
    fn default() -> Self {
        BoardContinuation::new(PSBoard::default())
    }
}

/// Allows direct access to the PSBoard embedded inside BoardContinuations.
///
impl Deref for BoardContinuation {
    type Target = PSBoard;
    fn deref(&self) -> &Self::Target {
        &self.board
    }
}

impl BoardContinuation {
    pub fn new(board: PSBoard) -> BoardContinuation {
        BoardContinuation {
            board: Arc::new(board),
            adjusted_score: f32::NAN,
            continuation: Arena::new(),
        }
    }

    #[inline]
    pub fn make_cached_move(mut self, the_move: &PossibleMove) -> BoardContinuation {
        if let Some(cont) = self.find_continuation_remove(the_move) {
            cont
        } else {
            BoardContinuation::new(self.make_move_noncached(the_move))
        }
    }

    pub fn continuation_exists(&self, the_move: &PossibleMove) -> bool {
        self.keys().any(|possible_move| possible_move == the_move)
    }

    pub fn insert_psboard(&mut self, the_move: &PossibleMove, board: PSBoard) {
        self.continuation
            .insert((*the_move, BoardContinuation::new(board)));
    }

    pub fn find_continuation_remove(
        &mut self,
        the_move: &PossibleMove,
    ) -> Option<BoardContinuation> {
        let index_opt =
            self.continuation.iter().find_map(
                |(index, (amove, _))| {
                    if amove == the_move {
                        Some(index)
                    } else {
                        None
                    }
                },
            );
        index_opt.map(|index| self.continuation.remove(index).unwrap().1)
    }

    pub fn find_continuation(&self, the_move: &PossibleMove) -> Option<&BoardContinuation> {
        self.iter().find_map(|(possible_move, continuation)| {
            if possible_move == the_move {
                Some(continuation)
            } else {
                None
            }
        })
    }

    pub fn find_continuation_mut(
        &mut self,
        the_move: &PossibleMove,
    ) -> Option<&mut BoardContinuation> {
        self.continuation
            .iter_mut()
            .find_map(|(_, (possible_move, continuation))| {
                if possible_move == the_move {
                    Some(continuation)
                } else {
                    None
                }
            })
    }

    pub fn values(&self) -> impl Iterator<Item = &BoardContinuation> {
        self.continuation
            .iter()
            .map(|(_, (_, continutation))| continutation)
    }

    pub fn keys(&self) -> impl Iterator<Item = &PossibleMove> {
        self.continuation
            .iter()
            .map(|(_, (posssible_move, _))| posssible_move)
    }

    pub fn merge(&mut self, mut to_merge: BoardContinuation) {
        to_merge
            .continuation
            .drain()
            .for_each(|(_, (amove, sub_continuation))| {
                if let Some(found_in_self) = self.find_continuation_mut(&amove) {
                    found_in_self.merge(sub_continuation);
                } else {
                    self.continuation.insert((amove, sub_continuation));
                }
            });
    }

    pub fn iter(&self) -> impl Iterator<Item = &(PossibleMove, BoardContinuation)> {
        self.continuation.iter().map(|(_, tuple)| tuple)
    }

    pub fn similar_quality_moves<'a, F>(
        &'a self,
        best_board: &'a BoardContinuation,
        score_query: F,
    ) -> impl Iterator<Item = &'a BoardContinuation>
    where
        F: Fn(&BoardContinuation) -> f32,
    {
        let bb_score = score_query(best_board);

        self.values().filter(move |other| {
            let other_score = score_query(other);
            (other_score.max(bb_score) - other_score.min(bb_score)) < 0.05
        })
    }

    pub fn select_similar_board<'a, F>(
        &'a self,
        best_board: &'a BoardContinuation,
        score_query: F,
    ) -> &'a BoardContinuation
    where
        F: Fn(&BoardContinuation) -> f32,
    {
        let choices = self.similar_quality_moves(best_board, &score_query).count();
        self.similar_quality_moves(best_board, &score_query)
            .nth(thread_rng().gen_range(0..choices))
            .unwrap()
    }

    pub fn total_continuation_boards(&self) -> u32 {
        self.continuation.len() as u32
            + self
                .values()
                .map(|next_board| next_board.total_continuation_boards())
                .sum::<u32>()
    }

    #[allow(dead_code)]
    pub fn visualise_explored_moves(&self) -> String {
        self.prefixed_visualise_explored_moves("")
    }

    #[allow(dead_code)]
    pub fn prefixed_visualise_explored_moves(&self, prefix: &str) -> String {
        self.internal_visualise(prefix, 0)
    }

    fn internal_visualise(&self, prefix: &str, depth: usize) -> String {
        let next_depth = depth + 1;
        self.iter()
            .map(|(a_move, its_board)| {
                format!(
                    "{prefix}{:depth$}{a_move}\n{}",
                    "",
                    its_board.internal_visualise(prefix, next_depth)
                )
            })
            .join("")
    }
}

#[cfg(test)]
mod test {
    use crate::baserules::board::PSBoard;
    use crate::baserules::board_rep::PossibleMove;
    use crate::engine::continuation::BoardContinuation;
    use generational_arena::Arena;
    use std::sync::Arc;

    fn create_simple_cont() -> BoardContinuation {
        let mut first = BoardContinuation {
            board: Arc::new(PSBoard::default()),
            adjusted_score: f32::NAN,
            continuation: Arena::new(),
        };
        let e2e4 = PossibleMove::simple_from_uci("e2e4").unwrap();
        first.insert_psboard(&e2e4, PSBoard::default().make_move_noncached(&e2e4));
        first
    }

    #[test]
    fn merge_two_simple() {
        let mut acont = create_simple_cont();
        let mut bcont = create_simple_cont();
        let first_move = *bcont.keys().next().unwrap();
        let e7e5 = PossibleMove::simple_from_uci("e7e5").unwrap();
        let new_board = bcont.make_move_noncached(&e7e5);
        let inner_cont = bcont.find_continuation_mut(&first_move).unwrap();
        inner_cont.insert_psboard(&e7e5, new_board);
        let btotal = bcont.total_continuation_boards();
        acont.merge(bcont);
        assert_eq!(acont.total_continuation_boards(), btotal);
    }
}
