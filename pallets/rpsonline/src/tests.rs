use super::*;
use crate::{Error, mock::*};
use frame_support::{assert_ok, assert_noop};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(RPSOnline::do_something(Origin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(RPSOnline::something(), Some(42));
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(
			RPSOnline::cause_error(Origin::signed(1)),
			Error::<Test>::NoneValue
		);
	});
}

#[test]
fn test_game_creation() {
	new_test_ext().execute_with(|| {

		let player_1:u64 = 1;
		let player_2:u64 = 2;
		let player_3:u64 = 3;

		let mut current_block:u64 = 100;

		// start from block 100
		run_to_block(current_block);

		// Test player can not play against himself
		assert_noop!(
			RPSOnline::new_game(Origin::signed(player_1), player_1),
			Error::<Test>::NoFakePlay
		);

		// Test game creation between to different players
		assert_ok!(RPSOnline::new_game(Origin::signed(player_1), player_2));
		run_to_block(1);

		let game_id_1 = RPSOnline::player_game(player_1);
		let game_id_2 = RPSOnline::player_game(player_2);

		assert_eq!(game_id_1, game_id_2);

		assert_noop!(
			RPSOnline::new_game(Origin::signed(player_1), player_3),
			Error::<Test>::PlayerHasGame
		);

		assert_noop!(
			RPSOnline::new_game(Origin::signed(player_3), player_2),
			Error::<Test>::PlayerHasGame
		);

		let game = RPSOnline::games(game_id_1);

		assert_eq!(game.last_action, 100);

	});
}

#[test]
fn try_simple_rps_game() {
	new_test_ext().execute_with(|| {

		let player_1:u64 = 1;
		let setup_1:[u8; 14] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13];
		let salt_1: [u8; 32] = [1u8;32];

		let player_2:u64 = 2;
		let setup_2:[u8; 14] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13];
		let salt_2: [u8; 32] = [2u8;32];

		let mut current_block:u64 = 100;

		// start from block 100
		run_to_block(current_block);

		// Create game
		assert_ok!(RPSOnline::new_game(Origin::signed(player_1), player_2));
		let game_id = RPSOnline::player_game(player_1);
		let game = RPSOnline::games(game_id);
		matches!(game.game_state, GameState::Initiate(_));
		assert_eq!(game.last_action, current_block);

		run_next_block();
		current_block = current_block + 1;

		// Initiate phase
		assert_ok!(RPSOnline::initiate(Origin::signed(player_1)));
		let game = RPSOnline::games(game_id);
		matches!(game.game_state, GameState::Initiate(_));
		assert_eq!(game.last_action, current_block);

		run_next_block();
		current_block = current_block + 1;

		assert_ok!(RPSOnline::initiate(Origin::signed(player_2)));
		let game = RPSOnline::games(game_id);
		matches!(game.game_state, GameState::Prepare(_));
		assert_eq!(game.last_action, current_block);

		run_next_block();
		current_block = current_block + 1;

		assert_ok!(RPSOnline::prepare(Origin::signed(player_2), setup_2, salt_2));
		let game = RPSOnline::games(game_id);
		matches!(game.game_state, GameState::Prepare(_));
		assert_eq!(game.last_action, current_block);

		run_next_block();
		current_block = current_block + 1;

		assert_ok!(RPSOnline::prepare(Origin::signed(player_1), setup_1, salt_1));
		let game = RPSOnline::games(game_id);
		matches!(game.game_state, GameState::Running(_));
		assert_eq!(game.last_action, current_block);

	});
}
