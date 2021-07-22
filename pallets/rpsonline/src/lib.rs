#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>

use codec::{Encode, Decode};
use frame_support::{
//	log,
	traits::{Randomness, LockIdentifier, schedule::{Named, DispatchTime}},
};
use sp_runtime::{
	traits::{Hash, Dispatchable, TrailingZeroInput}
};
use sp_io::hashing::blake2_256;
use sp_std::vec::{
	Vec
};
use pallet_matchmaker::MatchFunc;

//use log::info;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

const RPSONLINE_ID: LockIdentifier = *b"rps-fire";

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod rpscore;
use rpscore::{Logic};

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum PhaseState<AccountId> {
	None,
	Move,
	Choose(Vec<AccountId>),
	Reveal(Vec<AccountId>),
}

impl<AccountId> Default for PhaseState<AccountId> { fn default() -> Self { Self::None } }

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum GameState<AccountId> {
	None,
	Initiate(Vec<AccountId>),
	Prepare(Vec<AccountId>),
	Running(AccountId),
	Finished(AccountId),
}
impl<AccountId> Default for GameState<AccountId> { fn default() -> Self { Self::None } }

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum Weapon {
	None,
	Rock,
	Paper,
	Scissor,
	Trap,
	King,
}
impl Default for Weapon { fn default() -> Self { Self::None } }

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum NinjaState<Hash> {
	None,
	Stealth(Hash),
	Reveal(Weapon),
	Dead,
}
impl<Hash> Default for NinjaState<Hash> { fn default() -> Self { Self::None } }

/// RPS Online board structure containing two players and the board
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Game<Hash, AccountId, BlockNumber> {
	id: Hash,
	players: Vec<AccountId>,
	ninjas: [Vec<NinjaState<Hash>>; 2],
	board: [[u8; 6]; 7],
	last_move: (u8, u8),
	last_action: BlockNumber,
	phase_state: PhaseState<AccountId>,
	game_state: GameState<AccountId>,
}

const PLAYER_1: u8 = 1;
const PLAYER_2: u8 = 2;
const MAX_GAMES_PER_BLOCK: u8 = 10;
const MAX_BLOCKS_PER_TURN: u8 = 10;
const CLEANUP_BOARDS_AFTER: u8 = 20;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	// important to use outside structs and consts
	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The generator used to supply randomness to contracts through `seal_random`.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		type Proposal: Parameter + Dispatchable<Origin=Self::Origin> + From<Call<Self>>;

		type Scheduler: Named<Self::BlockNumber, Self::Proposal, Self::PalletsOrigin>;

		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

		type MatchMaker: MatchFunc<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn founder_key)]
	pub type FounderKey<T: Config> = StorageValue<_, T::AccountId>;

	// Default value for Nonce
	#[pallet::type_value]
	pub fn NonceDefault<T: Config>() -> u64 { 0 }
	// Nonce used for generating a different seed each time.
	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64, ValueQuery, NonceDefault<T>>;

	#[pallet::storage]
	#[pallet::getter(fn games)]
	/// Store all games that are currently being played.
	pub type Games<T: Config> = StorageMap<_, Identity, T::Hash, Game<T::Hash, T::AccountId, T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn player_game)]
	/// Store players active games, currently only one game per player allowed.
	pub type PlayerGame<T: Config> = StorageMap<_, Identity, T::AccountId, T::Hash, ValueQuery>;

	// The genesis config type.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub founder_key: T::AccountId,
	}

	// The default value for the genesis config type.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				founder_key: Default::default(),
			}
		}
	}

	// The build of genesis for the pallet.
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<FounderKey<T>>::put(&self.founder_key);
		}
	}

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		/// A new game got created.
		NewGame(T::Hash),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Only founder is allowed to do this.
		OnlyFounderAllowed,
		/// Player can't play against them self.
		NoFakePlay,
		/// Player has already a game.
		PlayerHasGame,
		/// Player has no active game or there is no such game.
		GameDoesntExist,
		/// Player choice already exist.
		PlayerChoiceExist,
		/// Player choice doesn't exist.
		PlayerChoiceDoesntExist,
		/// Bad behaviour, trying to cheat?
		BadBehaviour,
		/// Bad setup, trying to cheat?
		BadSetup,
		/// Player is already queued.
		AlreadyQueued,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// `on_initialize` is executed at the beginning of the block before any extrinsic are
		// dispatched.
		//
		// This function must return the weight consumed by `on_initialize` and `on_finalize`.
		fn on_initialize(_: T::BlockNumber) -> Weight {
			// Anything that needs to be done at the start of the block.
			// We don't do anything here.
			
			// initial weights
			let mut tot_weights = 10_000;
			for _i in 0..MAX_GAMES_PER_BLOCK {
				// try to create a match till we reached max games or no more matches available
				let result = T::MatchMaker::try_match();
				// if result is not empty we have a valid match
				if !result.is_empty() {
					// Create new game
					let _game_id = Self::create_game(result);
					// weights need to be adjusted
					tot_weights = tot_weights + T::DbWeight::get().reads_writes(1,1);
					continue;
				}
				break;
			}

			// return standard weigth for trying to fiond a match
			return tot_weights
		}

		// `on_finalize` is executed at the end of block after all extrinsic are dispatched.
		fn on_finalize(_n: BlockNumberFor<T>) {
			// Perform necessary data/state clean up here.
		}

		// A runtime code run after every block and have access to extended set of APIs.
		//
		// For instance you can generate extrinsics for the upcoming produced block.
		fn offchain_worker(_n: T::BlockNumber) {
			// We don't do anything here.
			// but we could dispatch extrinsic (transaction/unsigned/inherent) using
			// sp_io::submit_extrinsic.
			// To see example on offchain worker, please refer to example-offchain-worker pallet
		 	// accompanied in this repository.
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T:Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}

		/// Create game for two players
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn new_game(origin: OriginFor<T>, opponent: T::AccountId) -> DispatchResult {
			
			let sender = ensure_signed(origin)?;

			// Don't allow playing against yourself.
			ensure!(sender != opponent, Error::<T>::NoFakePlay);

			// Don't allow queued player to create a game.
			ensure!(!T::MatchMaker::is_queued(sender.clone()), Error::<T>::AlreadyQueued);
			ensure!(!T::MatchMaker::is_queued(opponent.clone()), Error::<T>::AlreadyQueued);

			// Make sure players have no board open.
			ensure!(!PlayerGame::<T>::contains_key(&sender), Error::<T>::PlayerHasGame);
			ensure!(!PlayerGame::<T>::contains_key(&opponent), Error::<T>::PlayerHasGame);
			
			let mut players = Vec::new();
			players.push(sender.clone());
			players.push(opponent.clone());

			// Create new game
			let _game_id = Self::create_game(players);

			Ok(())
		}

		/// Queue sender up for a game, ranking brackets
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn queue(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Make sure player has no board open.
			ensure!(!PlayerGame::<T>::contains_key(&sender), Error::<T>::PlayerHasGame);

			let bracket: u8 = 0;
			// Add player to queue, duplicate check is done in matchmaker.
			if !T::MatchMaker::add_queue(sender, bracket) {
				return Err(Error::<T>::AlreadyQueued)?
			} 

			Ok(())
		}

		/// Empty all brackets, this is a founder only extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn empty_queue(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Make sure sender is founder.
			ensure!(sender == Self::founder_key().unwrap(), Error::<T>::OnlyFounderAllowed);

			// Empty queues
			T::MatchMaker::all_empty_queue();

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn initiate(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Make sure player has a running game.
			ensure!(PlayerGame::<T>::contains_key(&sender), Error::<T>::GameDoesntExist);
			let game_id = Self::player_game(&sender);

			// Make sure game exists.
			ensure!(Games::<T>::contains_key(&game_id), Error::<T>::GameDoesntExist);

			// get players game
			let game = Self::games(&game_id);

			// check if we have correct state
			if let GameState::Initiate(_) = game.game_state {
				// check we have the correct state
			} else {
				Err(Error::<T>::BadBehaviour)?
			}

			// game state change
			if !Self::game_state_change(sender, game) {
				Err(Error::<T>::BadBehaviour)?
			}
				
			Ok(())		
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn prepare(origin: OriginFor<T>, setup: [u8;14], salt: [u8; 32]) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Make sure player has a running game.
			ensure!(PlayerGame::<T>::contains_key(&sender), Error::<T>::GameDoesntExist);
			let game_id = Self::player_game(&sender);

			// Make sure game exists.
			ensure!(Games::<T>::contains_key(&game_id), Error::<T>::GameDoesntExist);

			// get players game
			let mut game = Self::games(&game_id);

			// get index of current player
			let index = game.players.iter().position(|p| *p == sender).unwrap();
			
			// setup ninjas for fight
			let mut ninjas: Vec<NinjaState<T::Hash>> = Vec::new();
			let mut check: [u8;14] = [0u8;14];
			for i in 0..14 {
				match setup[i] {
					0 => { ninjas.push(NinjaState::Stealth(Self::hash_choice(salt, i, Weapon::King))) },
					1 => { ninjas.push(NinjaState::Stealth(Self::hash_choice(salt, i, Weapon::Trap))) },
					2|3|4|5 => { ninjas.push(NinjaState::Stealth(Self::hash_choice(salt, i, Weapon::Rock))) },
					6|7|8|9 => { ninjas.push(NinjaState::Stealth(Self::hash_choice(salt, i, Weapon::Paper))) },
					10|11|12|13 => { ninjas.push(NinjaState::Stealth(Self::hash_choice(salt, i, Weapon::Scissor))) },
					// out of range u8 leads to error
					_ => Err(Error::<T>::BadSetup)?,
				}
				// check occurance maximum one, otherwise leads to error
				if check[setup[i] as usize] == 0 {
					check[setup[i] as usize] = 1;
				} else {
					Err(Error::<T>::BadSetup)?
				}
			}

			// set player ninjas in game.
			game.ninjas[index] = ninjas;

			// check if we have correct state
			if let GameState::Prepare(_) = game.game_state {
				// check we have the correct state
			} else {
				Err(Error::<T>::BadBehaviour)?
			}

			// game state change
			if !Self::game_state_change(sender, game) {
				Err(Error::<T>::BadBehaviour)?
			}
				
			Ok(())		
		}
	}
}

impl<T: Config> Pallet<T> {

	/// Update nonce once used. 
	fn encode_and_update_nonce(
	) -> Vec<u8> {
		let nonce = <Nonce<T>>::get();
		<Nonce<T>>::put(nonce.wrapping_add(1));
		nonce.encode()
	}

	/// Generates a random hash out of a seed.
	fn generate_random_hash(
		phrase: &[u8], 
		sender: T::AccountId
	) -> T::Hash {
		let (seed, _) = T::Randomness::random(phrase);
		let seed = <[u8; 32]>::decode(&mut TrailingZeroInput::new(seed.as_ref()))
			.expect("input is padded with zeroes; qed");
		return (seed, &sender, Self::encode_and_update_nonce()).using_encoded(T::Hashing::hash);
	}

	fn hash_choice(
		salt: [u8; 32],
		position: usize,
		choice: Weapon
	) -> T::Hash {
		let mut choice_value = salt;
		choice_value[30] = position as u8;
		choice_value[31] = choice as u8;
		let choice_hashed = blake2_256(&choice_value);
		// return hashed choice
		choice_hashed.using_encoded(T::Hashing::hash)
	}

	fn create_game(
		players: Vec<T::AccountId>
	) -> T::Hash {

		// get a random hash as board id
		let game_id = Self::generate_random_hash(b"create", players[0].clone());

		// get current blocknumber
		let block_number = <frame_system::Pallet<T>>::block_number();

		// create a new empty game
		let game: Game<T::Hash, T::AccountId, T::BlockNumber> = Game {
			id: game_id,
			players: players.clone(),
			ninjas: [Vec::new(),Vec::new()],
			board: [[u8::MAX; 6]; 7],
			last_move: (u8::MAX, u8::MAX),
			last_action: block_number,
			phase_state: PhaseState::None,
			game_state: GameState::Initiate(players.clone()),
		};

		// insert the new game into the storage
		<Games<T>>::insert(game_id, game);

		// insert conenction for each player with the game
		for player in &players {
			<PlayerGame<T>>::insert(player, game_id);
		}
		
		// emit event for a new game creation
		Self::deposit_event(Event::NewGame(game_id));

		game_id
	}

	fn try_remove(
		player: T::AccountId,
		players: &mut Vec<T::AccountId>
	) -> bool {
		if let Some(p) = players.iter().position(|x| *x == player) {
			// remove player from vec
			players.swap_remove(p);
			return true;
		} 
		
		false
	}

	fn game_state_change(
		player: T::AccountId,
		mut game: Game<T::Hash, T::AccountId, T::BlockNumber>
	) -> bool {

		match game.game_state.clone() {

			GameState::Initiate(mut players) => {
				if !Self::try_remove(player, &mut players) {
					return false;
				}
				// check if all players have initiated
				if players.is_empty() {
					game.game_state = GameState::Prepare(game.players.clone());
				} else {
					game.game_state = GameState::Initiate(players);
				}				
			},

			GameState::Prepare(mut players) => {
				if !Self::try_remove(player, &mut players) {
					return false;
				}
				// check if all players have choosen
				if players.is_empty() {
					game.game_state = GameState::Running(game.players[0].clone());
				} else {
					game.game_state = GameState::Prepare(players);
				}
			},

			GameState::Running(mut player) => {

			},

			GameState::Finished(mut player) => {

			},
			_ => return false,
		}
		
		// get current blocknumber
		let block_number = <frame_system::Pallet<T>>::block_number();
		game.last_action = block_number;
		<Games<T>>::insert(game.id, game);
		
		true
	}
}
