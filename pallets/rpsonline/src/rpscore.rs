// use rand::Rng;
use codec::{Encode, Decode};

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum Direction {
    None,
    Right,
    Forward,
    Left,
}

pub struct Logic {
}

impl Logic {

    pub fn initialize() -> [[u8; 6]; 7] {
        let mut board = [[u8::MAX; 6]; 7];

        for y in 0..board[0].len() {
            for x in 0..board.len() {
                let pos: u8 = (y * 7 + x) as u8;
                if pos < 14 {
                    board[x][y] = pos;
                } else if pos > 27 {
                    board[x][y] = 15 + (42 - pos);
                }
            }
        }

        board
    }

    pub fn destination(player: u8, position: &mut [u8; 2], direction: Direction) -> bool {

        if ((player == 0 && direction ==  Direction::Left) || (player == 1 && direction == Direction::Right)) && position[0] > 0 {
            position[0] = position[0] - 1;
            return true;
        } else if ((player == 0 && direction == Direction::Right) || (player == 1 && direction == Direction::Left)) && position[0] < 6 {
            position[0] = position[0] + 1;
            return true;
        } else if player == 0 && direction == Direction::Forward && position[1] < 5 {
            position[1] = position[1] + 1;
            return true;
        } else if player == 1 && direction == Direction::Forward && position[1] > 0 {
            position[1] = position[1] - 1;
            return true;
        } else {
            return false;
        }
    }
}