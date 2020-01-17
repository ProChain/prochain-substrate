#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	debug::native,
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement},
	weights::SimpleDispatchInfo,
	Parameter, StorageMap, StorageValue,
};
use frame_system::{
	self as system, ensure_none, ensure_root, ensure_signed, offchain::SubmitUnsignedTransaction,
};
use hex::FromHex;
use simple_json::{self, json::JsonValue};
use sp_core::{offchain::Duration, offchain::HttpRequestId, offchain::HttpRequestStatus};
use sp_runtime::app_crypto::{KeyTypeId, RuntimeAppPublic};
use sp_runtime::{
	offchain::http,
	traits::{Hash, Member},
	transaction_validity::{
		TransactionLongevity, TransactionPriority, TransactionValidity, UnknownTransaction,
		ValidTransaction,
	},
	DispatchResult as dispatch_result,
};
use sp_std::{
	convert::{Into, TryInto},
	prelude::*,
	result::Result,
	vec::Vec,
};

extern crate num_bigint_dig as num_bigint;
//extern crate num_traits;
use num_bigint::{BigUint, ToBigUint};
use num_traits::{One, Zero};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"orin");

pub mod sr25519 {
	mod app_sr25519 {
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, super::super::KEY_TYPE);

		impl From<Signature> for sp_runtime::AnySignature {
			fn from(sig: Signature) -> Self {
				sr25519::Signature::from(sig).into()
			}
		}
	}

	pub type AuthorityId = app_sr25519::Public;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct EventLogSource {
	event_type: Vec<u8>,
	event_url: Vec<u8>,
	event_data: Vec<u8>,
}

// Config event json parse fields
const KEY_STATUS: &'static str = "status";
const KEY_MESSAGE: &'static str = "message";
const KEY_RESULT: &'static str = "result";
const KEY_ADDRESS: &'static str = "address";
const KEY_TOPICS: &'static str = "topics";
const KEY_DATA: &'static str = "data";
const KEY_BLOCK_NUMBER: &'static str = "blockNumber";
const KEY_TIME_STAMP: &'static str = "timeStamp";
const KEY_TX_HASH: &'static str = "transactionHash";
const KEY_TX_INDEX: &'static str = "transactionIndex";
const KEY_REMOVED: &'static str = "removed";

const STATUS_OK: &'static str = "1";
const MESSAGE_OK: &'static str = "OK";
const MESSAGE_NOT_FOUND: &'static str = "No records found";
const STR_PREFIX: &'static str = "0x";

//event source types
const EVENT_SRC_ETHERSCAN: &'static [u8; 9] = b"etherscan";
const EVENT_SRC_INFURA: &'static [u8; 6] = b"infura";

// TODO: auto generate EventSignature by contract abi
const EVENT_SIG_HTLC: &'static str =
	"0x5a0cc384a12a55445d4625db5d24f6a72177fd330644e2d4b3ea0ebd6f78c54d";
const EVENT_SIG_CLAIM: &'static str =
	"0x07a9dd1ef03da239626dc5c5bac1995991043d2b6e0e23ca789bbc0a16eb911f";
const EVENT_SIG_REFUND: &'static str =
	"0x215e15eef6d0300f9e89d940198e4f7fc22e44b7c80118c03571cd96da6c6c98";

const B_ALPHA: &'static [u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct EventHTLC<BlockNumber, Balance, Hash>
where
	BlockNumber: PartialEq + Eq + Decode + Encode,
{
	eth_contract_addr: Vec<u8>,
	htlc_block_number: BlockNumber,
	event_block_number: BlockNumber,
	expire_height: u32,
	random_number_hash: Vec<u8>, //When event_type is Claimed，value is random_number instead of hash
	swap_id: Hash,
	// event_timestamp: u64,
	// htlc_timestamp: u64,
	sender_addr: Vec<u8>,
	sender_chain_type: HTLCChain,
	receiver_addr: Hash,
	receiver_chain_type: HTLCChain,
	recipient_addr: Vec<u8>,
	out_amount: Balance,
	event_type: HTLCType,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub enum HTLCStates {
	INVALID,
	OPEN,
	COMPLETED,
	EXPIRED,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub enum HTLCChain {
	/// Ethereum Mainnet
	ETHMain,
	/// Prochain
	PRA,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub enum HTLCType {
	HTLC,
	Claimed,
	Refunded,
}

//  automates offchain fetching every certain blocks
pub const BLOCK_DURATION: u64 = 5;

pub trait Trait: pallet_balances::Trait + pallet_timestamp::Trait + did::Trait {
	/// The identifier type for an authority.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	/// A dispatchable call type.
	type Call: From<Call<Self>>;
	/// A transaction submitter.
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// invlid event type
		InvalidEventType,

		/// invlid did
		InvalidDidType,

		/// invlid event source type
		InvalidEventSrcType,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as OracleSwap {
		/// Stores the locked pra tokens
		pub PraTokenAddr get(pra_token_addr): Option<T::AccountId>;

		/// The current set of keys that may call update
		pub Authorities get(authorities) config(): Option<T::AccountId>;

		/// Stores offchain request jobs
		pub OcRequests get(oc_requests): Option<EventLogSource>;

		/// Key is swap_id, value is EventHTLC, should be removed after completed
		pub SwapData get(swap_data): map T::Hash => Option<EventHTLC<T::BlockNumber, T::Balance, T::Hash>>;

		/// Key is swap_id, Value is HTLCStates, Note: should never be removed
		pub SwapStates get(swap_states): map T::Hash => Option<HTLCStates>;

		/// Total count in SwapStates, Note: should always be larger
		pub SwapStatesCount get(swap_states_count): u64;
	}
}

decl_event!(
	pub enum Event<T>
	where
		<T as system::Trait>::BlockNumber,
		<T as system::Trait>::AccountId,
		<T as system::Trait>::Hash,
		<T as pallet_balances::Trait>::Balance,
	{
		///Setup and kickoff event_type, event_url, event_data
		Kickoff(Vec<u8>, Vec<u8>, Vec<u8>),

		///kill scanned event_name and event_url, make sure run only once
		//Kill(Vec<u8>, Vec<u8>),

		///receiver_addr, eth_contract_addr, htlc_block_number, expire_height, random_number_hash, swap_id, sender_addr, out_amount
		HTLC(Hash, Vec<u8>, BlockNumber, u32, Vec<u8>, Hash, Vec<u8>, Balance),

		///receiver_addr, eth_contract_addr, swap_id, sender_addr, random_number
		Claim(Hash, Vec<u8>, Hash, Vec<u8>,Vec<u8>),

		///receiver_addr, eth_contract_addr, sender_addr, random_number_hash
		Refund(Hash, Vec<u8>, Hash, Vec<u8>, Vec<u8>),

		///sender_account, receiver_account, receiver_did, out_amount
		TransferToDid(AccountId, AccountId, Hash, Balance),

		//did hex str, did account
		TryParseDid(Vec<u8>, AccountId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		fn on_initialize(_now: T::BlockNumber) {
			<Self as Store>::OcRequests::take();
		}

		// Initializing event fetch jobs
		#[weight = SimpleDispatchInfo::FixedNormal(500_000)]
		fn kickoff(origin, event_src_type: Vec<u8>, event_url: Vec<u8>, event_data: Vec<u8>) -> dispatch_result {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_authority(&sender), "error not authority sender");

			if event_src_type != EVENT_SRC_ETHERSCAN && event_src_type != EVENT_SRC_INFURA {
				return Err(Error::<T>::InvalidEventSrcType)?;
			}

			native::info!(target: "swap", "kickoff event fetch jobs");

			let event_src = EventLogSource {
				event_type: event_src_type.clone(),
				event_url: event_url.clone(),
				event_data: event_data.clone(),
			};
			<Self as Store>::OcRequests::put(event_src);
			Self::deposit_event(RawEvent::Kickoff(event_src_type, event_url, event_data));
			Ok(())
		}

		// Kill all event fetch jobs
		fn killall(origin) -> dispatch_result {
			let sender = ensure_signed(origin)?;

			if Self::is_authority(&sender) {
				<Self as Store>::OcRequests::take();
			}

			Ok(())
		}

		// Try parse did in hex str format
		fn try_parse_did(origin, did_hex: Vec<u8>) -> dispatch_result {
			let _ = ensure_signed(origin)?;

			if let Ok(account) = Self::parse_did(&did_hex) {
				Self::deposit_event(RawEvent::TryParseDid(did_hex, account));
				return Ok(());
			}
			Err(Error::<T>::InvalidDidType)?
		}

		// Add a new authority to the set of keys that are allowed to update.
		fn init(origin, auth: T::AccountId, pra_token_addr: T::AccountId) -> dispatch_result {
			ensure_root(origin)?;

			// TODO: add auth control
			// let sender = ensure_signed(origin)?;
			// ensure!(Self::is_authority(&sender), "sender is not authority account");
			// ensure!(!Self::is_authority(&who), "user is already authority account");

			<Authorities<T>>::put(auth);
			<PraTokenAddr<T>>::put(pra_token_addr.clone());
			<SwapStatesCount>::put(0);
			Ok(())
		}

		// Runs after every block.
		fn offchain_worker(now: T::BlockNumber) {
			frame_support::debug::RuntimeLogger::init();
			//if BLOCK_DURATION > 0 && (TryInto::<u64>::try_into(now).ok().unwrap()) % BLOCK_DURATION == 0 {
			Self::offchain_events(now);
			//}
		}

		// Stores valid swap data and states
		fn update_enevt_htlc(origin, htlcs: Vec<EventHTLC<T::BlockNumber, T::Balance, T::Hash>>) -> dispatch_result {
			// TODO: add auth control
			ensure_none(origin)?;

			ensure!(Self::pra_token_addr().is_some(), "error not valid pra_token_addr");
			let pra_token_addr = Self::pra_token_addr().unwrap();

			for htlc in htlcs {
				match htlc.event_type {
					HTLCType::HTLC => {
						if !<SwapData<T>>::exists(htlc.swap_id) && !<SwapStates<T>>::exists(htlc.swap_id) {
							<SwapData<T>>::insert(htlc.swap_id, &htlc);
							<SwapStates<T>>::insert(htlc.swap_id, HTLCStates::OPEN);

							let swap_states_count = Self::swap_states_count();
							let new_count = swap_states_count.checked_add(1).ok_or("Overflow adding swap_states_count")?;
							<SwapStatesCount>::put(new_count);

							Self::deposit_event(RawEvent::HTLC(htlc.receiver_addr, htlc.eth_contract_addr, htlc.htlc_block_number, htlc.expire_height,
								htlc.random_number_hash, htlc.swap_id, htlc.sender_addr, htlc.out_amount));
						} else {
							native::error!(target: "swap", "HTLC init swap_id already exists");
						}
					},
					HTLCType::Claimed => {
						if <SwapData<T>>::exists(htlc.swap_id) && <SwapStates<T>>::exists(htlc.swap_id) {
							let swap_id = htlc.swap_id;
							let htlc = <SwapData<T>>::get(&swap_id).unwrap();

							//transfer
							Self::transfer_to_did_hash(pra_token_addr.clone(), htlc.receiver_addr.clone(), htlc.out_amount)?;

							<SwapData<T>>::remove(&swap_id);
							<SwapStates<T>>::insert(htlc.swap_id, HTLCStates::COMPLETED);
							Self::deposit_event(RawEvent::Claim(htlc.receiver_addr, htlc.eth_contract_addr, swap_id, htlc.sender_addr, htlc.random_number_hash));
						} else {
							native::error!(target: "swap", "HTLC claimed swap_id not exists");
						}
					},
					HTLCType::Refunded => {
						if <SwapData<T>>::exists(htlc.swap_id) && <SwapStates<T>>::exists(htlc.swap_id) {
							let swap_id = htlc.swap_id;
							<SwapData<T>>::remove(&swap_id);
							<SwapStates<T>>::insert(htlc.swap_id, HTLCStates::EXPIRED);

							Self::deposit_event(RawEvent::Refund(htlc.receiver_addr, htlc.eth_contract_addr, swap_id, htlc.sender_addr, htlc.random_number_hash));
						} else {
							native::error!(target: "swap", "HTLC refund swap_id not exists");
						}
					},
					_ =>  Err(Error::<T>::InvalidEventType)?
				}
			}
			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
	fn offchain_events(now: T::BlockNumber) {
		let fetch_info = <Self as Store>::OcRequests::get().clone();
		if fetch_info.is_some() {
			let fetch_info = fetch_info.unwrap();

			<Self as Store>::OcRequests::take();
			Self::fetch_events(&fetch_info);
		}
	}

	fn to_balance(val: u128) -> Result<T::Balance, &'static str> {
		val.try_into()
			.map_err(|_| "Convert to Balance type overflow")
	}

	//for etherscan
	fn fetch_events(event: &EventLogSource) -> Result<(), &'static str> {
		let pra_token_addr = Self::pra_token_addr();
		ensure!(pra_token_addr.is_some(), "pra_token_addr can not be empty");

		let url_result = core::str::from_utf8(&event.event_url);
		if url_result.is_err() {
			return Err("error event_url is not valid utf8");
		}

		let url = url_result.unwrap();
		native::info!(target: "swap", "kickoff fetch events {:?}", url);
		if event.event_type == EVENT_SRC_ETHERSCAN {
			let res = Self::http_request_get(&url);
			if let Ok(buf) = res {
				let htlcs = Self::parse_data(buf);

				let call = Call::update_enevt_htlc(htlcs);
				let result = T::SubmitTransaction::submit_unsigned(call);
				match result {
					Ok(_) => {
						native::info!(target: "swap", "execute off-chain worker success EVENT_SRC_ETHERSCAN");
					}
					Err(_) => {
						native::error!(target: "swap", "execute off-chain worker failed EVENT_SRC_ETHERSCAN");
						return Err("error happens when submit unsigned transaction");
					}
				}
			} else {
			}
		} else if event.event_type == EVENT_SRC_INFURA {
			let res = Self::http_request_post(&url, &event.event_data);
			if let Ok(buf) = res {
				let htlcs = Self::parse_infura_data(buf);

				let call = Call::update_enevt_htlc(htlcs);
				let result = T::SubmitTransaction::submit_unsigned(call);
				match result {
					Ok(_) => {
						native::info!(target: "swap", "execute off-chain worker success EVENT_SRC_INFURA");
					}
					Err(_) => {
						native::error!(target: "swap", "execute off-chain worker failed EVENT_SRC_INFURA");
						return Err("error happens when submit unsigned transaction");
					}
				}
			}
		}

		Ok(())
	}

	//for etherscan
	fn parse_data(res: Vec<u8>) -> Vec<EventHTLC<T::BlockNumber, T::Balance, T::Hash>> {
		native::debug!(target: "swap", "parse etherscan data {:?}", res);

		let mut vec_results: Vec<EventHTLC<T::BlockNumber, T::Balance, T::Hash>> = Vec::new();

		let json_str = core::str::from_utf8(&res);
		if json_str.is_err() {
			return vec_results;
		}

		if let Ok(json_val) = simple_json::parse_json(json_str.unwrap()) {
			let mut message = Vec::new();
			let mut status = Vec::new();
			let mut results = Vec::new();

			json_val
				.get_object()
				.iter()
				.filter(|(k, _)| {
					let key: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					KEY_MESSAGE.as_bytes().to_vec() == key
						|| KEY_STATUS.as_bytes().to_vec() == key
						|| KEY_RESULT.as_bytes().to_vec() == key
				})
				.for_each(|(k, v)| {
					let vec_of_u8s: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					let key = core::str::from_utf8(&vec_of_u8s).unwrap();

					if key == KEY_MESSAGE {
						if let JsonValue::String(obj) = v {
							message = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
						}
					} else if key == KEY_STATUS {
						if let JsonValue::String(obj) = v {
							status = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
						}
					} else if key == KEY_RESULT {
						if let JsonValue::Array(array) = v {
							results = array.to_vec();
						}
					}
				});

			if status != b"1" || message != b"OK" {
				return vec_results;
			}

			for result in results.iter() {
				let mut contract_addr = Vec::new();
				let mut topics = Vec::new();
				let mut data = Vec::new();
				let mut event_block_number = Vec::new();
				let mut event_time_stamp = Vec::new();
				let mut tx_hash = Vec::new();
				let mut tx_index = Vec::new();

				result
					.get_object()
					.iter()
					.filter(|(k, _)| {
						let key: Vec<u8> = k.iter().map(|c| *c as u8).collect();
						KEY_ADDRESS.as_bytes().to_vec() == key
							|| KEY_TOPICS.as_bytes().to_vec() == key
							|| KEY_DATA.as_bytes().to_vec() == key
							|| KEY_BLOCK_NUMBER.as_bytes().to_vec() == key
							|| KEY_TIME_STAMP.as_bytes().to_vec() == key
							|| KEY_TX_HASH.as_bytes().to_vec() == key
							|| KEY_TX_INDEX.as_bytes().to_vec() == key
					})
					.for_each(|(k, v)| {
						let vec_of_u8s: Vec<u8> = k.iter().map(|c| *c as u8).collect();
						let key = core::str::from_utf8(&vec_of_u8s).unwrap();

						if key == KEY_ADDRESS {
							if let JsonValue::String(obj) = v {
								contract_addr = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TOPICS {
							if let JsonValue::Array(array) = v {
								for i in array.iter() {
									if let JsonValue::String(obj) = i {
										topics.push(
											obj.iter().map(|c| *c as u8).collect::<Vec<u8>>(),
										);
									}
								}
							}
						} else if key == KEY_DATA {
							if let JsonValue::String(obj) = v {
								data = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_BLOCK_NUMBER {
							if let JsonValue::String(obj) = v {
								event_block_number =
									obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TIME_STAMP {
							if let JsonValue::String(obj) = v {
								event_time_stamp =
									obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TX_HASH {
							if let JsonValue::String(obj) = v {
								tx_hash = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TX_INDEX {
							if let JsonValue::String(obj) = v {
								tx_index = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						}
					});

				if topics.len() == 0 {
					continue;
				}

				match core::str::from_utf8(&topics[0]).unwrap() {
					EVENT_SIG_HTLC => {
						match Self::parse_htlc_event(
							contract_addr,
							topics,
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_htlc_event err {:?}", e);
							}
						}
					}
					EVENT_SIG_REFUND => {
						match Self::parse_refund_event(
							contract_addr,
							topics,
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_refund_event err {:?}", e);
							}
						}
					}
					EVENT_SIG_CLAIM => {
						match Self::parse_claim_event(
							contract_addr,
							topics,
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_claim_event err {:?}", e);
							}
						}
					}
					_ => {
						native::error!(target: "swap", "not valid event signature {:?}", &topics[0]);
					}
				}
			}
		}

		return vec_results;
	}

	//for infura
	fn parse_infura_data(res: Vec<u8>) -> Vec<EventHTLC<T::BlockNumber, T::Balance, T::Hash>> {
		native::debug!(target: "swap", "parse infura data {:?}", res);

		let mut vec_results: Vec<EventHTLC<T::BlockNumber, T::Balance, T::Hash>> = Vec::new();

		let json_str = core::str::from_utf8(&res);
		if json_str.is_err() {
			return vec_results;
		}

		if let Ok(json_val) = simple_json::parse_json(json_str.unwrap()) {
			let mut results = Vec::new();

			json_val
				.get_object()
				.iter()
				.filter(|(k, _)| {
					let key: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					KEY_RESULT.as_bytes().to_vec() == key
				})
				.for_each(|(k, v)| {
					let vec_of_u8s: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					let key = core::str::from_utf8(&vec_of_u8s).unwrap();

					if key == KEY_RESULT {
						if let JsonValue::Array(array) = v {
							results = array.to_vec();
						}
					}
				});

			for result in results.iter() {
				let mut contract_addr = Vec::new();
				let mut topics = Vec::new();
				let mut data = Vec::new();
				let mut event_block_number = Vec::new();
				let removed: bool = false;
				let mut event_time_stamp = Vec::new();
				let mut tx_hash = Vec::new();
				let mut tx_index = Vec::new();

				result
					.get_object()
					.iter()
					.filter(|(k, _)| {
						let key: Vec<u8> = k.iter().map(|c| *c as u8).collect();
						KEY_ADDRESS.as_bytes().to_vec() == key
							|| KEY_TOPICS.as_bytes().to_vec() == key
							|| KEY_DATA.as_bytes().to_vec() == key
							|| KEY_BLOCK_NUMBER.as_bytes().to_vec() == key
							|| KEY_TX_HASH.as_bytes().to_vec() == key
							|| KEY_TX_INDEX.as_bytes().to_vec() == key
							|| KEY_REMOVED.as_bytes().to_vec() == key
					})
					.for_each(|(k, v)| {
						let vec_of_u8s: Vec<u8> = k.iter().map(|c| *c as u8).collect();
						let key = core::str::from_utf8(&vec_of_u8s).unwrap();

						if key == KEY_ADDRESS {
							if let JsonValue::String(obj) = v {
								contract_addr = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TOPICS {
							if let JsonValue::Array(array) = v {
								for i in array.iter() {
									if let JsonValue::String(obj) = i {
										topics.push(
											obj.iter().map(|c| *c as u8).collect::<Vec<u8>>(),
										);
									}
								}
							}
						} else if key == KEY_DATA {
							if let JsonValue::String(obj) = v {
								data = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_BLOCK_NUMBER {
							if let JsonValue::String(obj) = v {
								event_block_number =
									obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_REMOVED {
							if let JsonValue::Boolean(obj) = v {
								//TODO: parse removed
								//removed =
								//	obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TX_HASH {
							if let JsonValue::String(obj) = v {
								tx_hash = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						} else if key == KEY_TX_INDEX {
							if let JsonValue::String(obj) = v {
								tx_index = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
							}
						}
					});

				if topics.len() == 0 || removed {
					continue;
				}

				match core::str::from_utf8(&topics[0]).unwrap() {
					EVENT_SIG_HTLC => {
						match Self::parse_htlc_event(
							contract_addr,
							topics.clone(),
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_htlc_event err {:?}", e);
							}
						}
					}
					EVENT_SIG_REFUND => {
						match Self::parse_refund_event(
							contract_addr,
							topics.clone(),
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_refund_event err {:?}", e);
							}
						}
					}
					EVENT_SIG_CLAIM => {
						match Self::parse_claim_event(
							contract_addr,
							topics.clone(),
							data,
							event_block_number,
							event_time_stamp,
							tx_hash,
							tx_index,
						) {
							Ok(htlc) => vec_results.push(htlc),
							Err(e) => {
								native::error!(target: "swap", "parse_claim_event err {:?}", e);
							}
						}
					}
					_ => {
						native::error!(target: "swap", "not valid event signature {:?}", &topics[0]);
					}
				}

				if topics.len() == 0 {
					continue;
				}
			}
		}

		return vec_results;
	}

	//for etherscan
	fn parse_htlc_event(
		contract_addr: Vec<u8>,
		topics: Vec<Vec<u8>>,
		data: Vec<u8>,
		event_block_number: Vec<u8>,
		event_time_stamp: Vec<u8>,
		tx_hash: Vec<u8>,
		tx_index: Vec<u8>,
	) -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash>, &'static str> {
		let msg_sender = &topics[1][STR_PREFIX.len()..].to_vec();
		let recipient_addr = &topics[2][STR_PREFIX.len()..].to_vec();
		let swap_id = &topics[3][STR_PREFIX.len()..].to_vec();

		let random_num_hash = &data[STR_PREFIX.len()..66].to_vec();
		let htlc_time_stamp = &data[STR_PREFIX.len() + 64..66 + 64].to_vec();
		let expire_height = &data[STR_PREFIX.len() + 64 + 64..66 + 64 + 64].to_vec();
		let out_amount = &data[STR_PREFIX.len() + 64 + 64 + 64..66 + 64 + 64 + 64].to_vec();
		let pra_amount =
			&data[STR_PREFIX.len() + 64 + 64 + 64 + 64..66 + 64 + 64 + 64 + 64].to_vec();
		let receiver_addr_len = &data
			[STR_PREFIX.len() + 64 + 64 + 64 + 64 + 64 + 64..64 + 64 + 64 + 64 + 64 + 64 + 66]
			.to_vec();
		let receiver_addr = &data[STR_PREFIX.len() + 64 + 64 + 64 + 64 + 64 + 64 + 64..].to_vec();

		let d = core::str::from_utf8(&receiver_addr_len[..]).unwrap();
		let mut length =
			usize::from_str_radix(d, 16).map_err(|_| "error parse length from utf8")?;

		// let event_ts = u64::from_str_radix(
		// 	core::str::from_utf8(&event_time_stamp[STR_PREFIX.len()..]).unwrap(),
		// 	16,
		// )
		// .map_err(|_| "error parse event_time_stamp from utf8")?;
		// let htlc_ts = u64::from_str_radix(
		// 	core::str::from_utf8(&htlc_time_stamp[STR_PREFIX.len()..]).unwrap(),
		// 	16,
		// )
		// .map_err(|_| "error parse htlc_time_stamp from utf8")?;
		let event_block_num = u32::from_str_radix(
			core::str::from_utf8(&event_block_number[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse event_block_num from utf8")?;
		let expire_block_num = u32::from_str_radix(
			core::str::from_utf8(&expire_height[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse event_block_num from utf8")?;

		let event_out_amount = u128::from_str_radix(
			core::str::from_utf8(&out_amount[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse out_amount from utf8")?;
		let event_pra_amount = u128::from_str_radix(
			core::str::from_utf8(&pra_amount[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse pra_amount from utf8")?;
		ensure!(
			event_out_amount > 0 && event_out_amount == event_pra_amount,
			"not valid out_amount or pra_amount"
		);

		//Important: precision from eth contract is 18, substrate precision is 15
		let out_balance = Self::to_balance(event_out_amount / 1000u128)
			.map_err(|_| "error parse event_out_amount to balance")?;

		length = length * 2usize;
		let did_hex = Vec::from_hex(&receiver_addr[..length])
			.map_err(|_| "error parse receiver_addr from utf8")?;
		let data_str =
			core::str::from_utf8(&did_hex[..]).map_err(|_| "error not valid utf8 did")?;

		let vecs: Vec<&str> = data_str.split(":").collect();
		ensure!(
			vecs.len() == 3 && vecs[2].len() > 0,
			"error htlc not found valid did"
		);

		let did_ele_hex = Self::from_base58(vecs[2].clone()).map_err(|_| "error Bad Base58")?;
		let receiver_did_hash = T::Hashing::hash(&did_ele_hex);

		let htlc = EventHTLC {
			eth_contract_addr: contract_addr,
			event_block_number: T::BlockNumber::from(event_block_num),
			htlc_block_number: <system::Module<T>>::block_number(),
			out_amount: out_balance,
			expire_height: expire_block_num - event_block_num,
			random_number_hash: random_num_hash.clone(),
			swap_id: T::Hashing::hash(&swap_id[..]),
			// event_timestamp: event_ts,
			// htlc_timestamp: htlc_ts,
			sender_addr: msg_sender.clone(),
			sender_chain_type: HTLCChain::ETHMain,
			receiver_addr: receiver_did_hash,
			receiver_chain_type: HTLCChain::PRA,
			recipient_addr: recipient_addr.clone(),
			event_type: HTLCType::HTLC,
		};
		Ok(htlc)
	}

	//for etherscan
	fn parse_claim_event(
		contract_addr: Vec<u8>,
		topics: Vec<Vec<u8>>,
		data: Vec<u8>,
		event_block_number: Vec<u8>,
		event_time_stamp: Vec<u8>,
		tx_hash: Vec<u8>,
		tx_index: Vec<u8>,
	) -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash>, &'static str> {
		let msg_sender = &topics[1][STR_PREFIX.len()..].to_vec();
		let recipient_addr = &topics[2][STR_PREFIX.len()..].to_vec();
		let swap_id = T::Hashing::hash(&topics[3][STR_PREFIX.len()..]);

		let random_num = &data[STR_PREFIX.len()..66].to_vec();
		let receiver_addr_len = &data[STR_PREFIX.len() + 64 + 64..64 + 64 + 66].to_vec();
		let receiver_addr = &data[STR_PREFIX.len() + 64 + 64 + 64..].to_vec();

		let d = core::str::from_utf8(&receiver_addr_len[..]).unwrap();
		let mut length =
			usize::from_str_radix(d, 16).map_err(|_| "error parse length from utf8")?;
		length = length * 2usize;

		let did_hex = Vec::from_hex(&receiver_addr[..length])
			.map_err(|_| "error parse receiver_addr from utf8")?;
		let data_str =
			core::str::from_utf8(&did_hex[..]).map_err(|_| "error not valid utf8 did")?;

		let vecs: Vec<&str> = data_str.split(":").collect();
		ensure!(
			vecs.len() == 3 && vecs[2].len() > 0,
			"error not found valid did"
		);

		let did_ele_hex = Self::from_base58(vecs[2].clone()).map_err(|_| "error Bad Base58")?;
		let receiver_did_hash = T::Hashing::hash(&did_ele_hex);

		let event_block_num = u32::from_str_radix(
			core::str::from_utf8(&event_block_number[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse event_block_num from utf8")?;

		let htlc = EventHTLC {
			eth_contract_addr: contract_addr,
			event_block_number: T::BlockNumber::from(event_block_num),
			htlc_block_number: <system::Module<T>>::block_number(),
			out_amount: T::Balance::from(0u32),
			expire_height: 0u32,
			random_number_hash: random_num.clone(),
			swap_id: swap_id.clone(),
			sender_addr: msg_sender.clone(),
			sender_chain_type: HTLCChain::ETHMain,
			receiver_addr: receiver_did_hash,
			recipient_addr: recipient_addr.clone(),
			receiver_chain_type: HTLCChain::PRA,
			event_type: HTLCType::Claimed,
		};
		Ok(htlc)
	}

	//for etherscan
	fn parse_refund_event(
		contract_addr: Vec<u8>,
		topics: Vec<Vec<u8>>,
		data: Vec<u8>,
		event_block_number: Vec<u8>,
		event_time_stamp: Vec<u8>,
		tx_hash: Vec<u8>,
		tx_index: Vec<u8>,
	) -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash>, &'static str> {
		let msg_sender = &topics[1][STR_PREFIX.len()..].to_vec();
		let recipient_addr = &topics[2][STR_PREFIX.len()..].to_vec();
		let swap_id = T::Hashing::hash(&topics[3][STR_PREFIX.len()..]);

		let random_num_hash = &data[STR_PREFIX.len()..66].to_vec();
		let receiver_addr_len = &data[STR_PREFIX.len() + 64 + 64 + 64..64 + 64 + 64 + 66].to_vec();
		let receiver_addr = &data[STR_PREFIX.len() + 64 + 64 + 64..].to_vec();

		let d = core::str::from_utf8(&receiver_addr_len[..]).unwrap();
		let mut length =
			usize::from_str_radix(d, 16).map_err(|_| "error parse length from utf8")?;
		length = length * 2usize;

		let did_hex = Vec::from_hex(&receiver_addr[..length])
			.map_err(|_| "error parse receiver_addr from utf8")?;
		let data_str =
			core::str::from_utf8(&did_hex[..]).map_err(|_| "error not valid utf8 did")?;

		let vecs: Vec<&str> = data_str.split(":").collect();
		ensure!(
			vecs.len() == 3 && vecs[2].len() > 0,
			"error not found valid did"
		);

		let did_ele_hex = Self::from_base58(vecs[2].clone()).map_err(|_| "error Bad Base58")?;
		let receiver_did_hash = T::Hashing::hash(&did_ele_hex);

		let event_block_num = u32::from_str_radix(
			core::str::from_utf8(&event_block_number[STR_PREFIX.len()..]).unwrap(),
			16,
		)
		.map_err(|_| "error parse event_block_num from utf8")?;

		let htlc = EventHTLC {
			eth_contract_addr: contract_addr,
			event_block_number: T::BlockNumber::from(event_block_num),
			htlc_block_number: <system::Module<T>>::block_number(),
			out_amount: T::Balance::from(0u32),
			expire_height: 0u32,
			random_number_hash: random_num_hash.clone(),
			swap_id: swap_id.clone(),
			sender_addr: msg_sender.clone(),
			sender_chain_type: HTLCChain::ETHMain,
			receiver_addr: receiver_did_hash,
			recipient_addr: recipient_addr.clone(),
			receiver_chain_type: HTLCChain::PRA,
			event_type: HTLCType::Refunded,
		};
		Ok(htlc)
	}

	fn http_request_get(uri: &str) -> Result<Vec<u8>, &'static str> {
		Self::http_simple_get(uri)
	}

	fn http_request_post(url: &str, data: &Vec<u8>) -> Result<Vec<u8>, &'static str> {
		Self::http_request_with_header(url, "POST", None, &data[..])
		//Self::http_simple_post(url, data)
	}

	fn http_simple_get(url: &str) -> Result<Vec<u8>, &'static str> {
		native::info!(target: "swap", "[url: {:?}]", url);

		let pending = http::Request::get(url)
			.send()
			.map_err(|_| "Request http GET failed")?;

		let response = pending
			.wait()
			.map_err(|_| "Request waiting http GET response failed")?;

		if response.code != 200 {
			return Err("Request Non-200 status GET returned");
		}

		let result: Vec<u8> = response.body().collect::<Vec<u8>>();
		return Ok(result);
	}

	// fn http_simple_post(url: &str, data: &Vec<u8>) -> Result<Vec<u8>, &'static str> {
	// 	let pending = http::Request::post(url, &data[..])
	// 		.send()
	// 		.map_err(|_| "Request http POST failed")?;

	// 	let response = pending
	// 		.wait()
	// 		.map_err(|_| "Request waiting http POST response failed")?;

	// 	if response.code != 200 {
	// 		return Err("Request Non-200 status POST returned");
	// 	}

	// 	let result: Vec<u8> = response.body().collect::<Vec<u8>>();
	// 	return Ok(result);
	// }

	fn http_request_with_header(
		uri: &str,
		method: &str,
		header: Option<(&str, &str)>,
		data: &[u8],
	) -> Result<Vec<u8>, &'static str> {
		let id: HttpRequestId = sp_io::offchain::http_request_start(method, uri, &[]).unwrap();
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(100_000));

		if let Some((name, value)) = header {
			match sp_io::offchain::http_request_add_header(id, name, value) {
				Ok(_) => (),
				Err(_) => return Err("Add request header failed"),
			};
		}

		match sp_io::offchain::http_request_write_body(id, data, Some(deadline)) {
			Ok(_) => (),
			Err(_) => return Err("Add request write body failed"),
		};

		match sp_io::offchain::http_response_wait(&[id], Some(deadline))[0] {
			HttpRequestStatus::Finished(200) => (),
			_ => return Err("Request failed"),
		}

		let mut result: Vec<u8> = vec![];
		loop {
			let mut buffer = vec![0; 1024];
			let _read = sp_io::offchain::http_response_read_body(id, &mut buffer, Some(deadline))
				.map_err(|_e| ());
			result = [&result[..], &buffer[..]].concat();
			if _read == Ok(0) {
				break;
			}
		}
		if result.len() > 0 {
			return Ok(result);
		} else {
			return Err("Parse body failed");
		}
	}

	//Helper that confirms whether the given `AccountId` has auth
	fn is_authority(who: &T::AccountId) -> bool {
		let auth = Self::authorities();
		auth.is_some() && auth.unwrap() == who.clone()
	}

	//if HTLC exists
	fn is_swap_exist(swap_id: &T::Hash) -> bool {
		let state = Self::swap_states(swap_id);
		state.is_some() && state.unwrap() != HTLCStates::INVALID
	}

	//if HTLC claimable
	fn is_claimable(swap_id: &T::Hash) -> bool {
		let state = Self::swap_states(swap_id);
		if state.is_some() && state.unwrap() == HTLCStates::OPEN {
			let swap = Self::swap_data(swap_id);
			if swap.is_some() {
				let swap = swap.unwrap();
				if <system::Module<T>>::block_number()
					< swap.htlc_block_number + T::BlockNumber::from(swap.expire_height)
				{
					return true;
				}
			}
		}
		false
	}

	//transfer to receiver by did
	fn transfer_to_did(
		sender: T::AccountId,
		receiver_did: Vec<u8>,
		amount: T::Balance,
		memo: Vec<u8>,
	) -> dispatch_result {
		ensure!(receiver_did.len() > 0, "error receiver_did is empty");

		let receiver_hash = <T::Hash as Decode>::decode(&mut receiver_did.as_slice())
			.map_err(|_| "error parse receiver_did from utf8")?;
		let receiver = <did::Module<T>>::identity_of(receiver_hash.clone());
		ensure!(receiver.is_some(), "error not valid receiver did");

		let receiver = receiver.unwrap();
		<pallet_balances::Module<T> as Currency<_>>::transfer(
			&sender,
			&receiver,
			amount,
			ExistenceRequirement::KeepAlive,
		)?;
		Self::deposit_event(RawEvent::TransferToDid(
			sender,
			receiver,
			receiver_hash,
			amount,
		));
		Ok(())
	}

	fn transfer_to_did_hash(
		sender: T::AccountId,
		receiver_did: T::Hash,
		amount: T::Balance,
	) -> dispatch_result {
		let receiver = <did::Module<T>>::identity_of(receiver_did.clone());
		ensure!(receiver.is_some(), "error not valid receiver did");

		let receiver = receiver.unwrap();
		<pallet_balances::Module<T> as Currency<_>>::transfer(
			&sender,
			&receiver,
			amount,
			ExistenceRequirement::KeepAlive,
		)?;
		Self::deposit_event(RawEvent::TransferToDid(
			sender,
			receiver,
			receiver_did,
			amount,
		));
		Ok(())
	}

	//did hex_str
	fn parse_did(did: &Vec<u8>) -> Result<T::AccountId, &'static str> {
		let data = core::str::from_utf8(&did).map_err(|_| "error not valid utf8 did")?;

		let vecs: Vec<&str> = data.split(":").collect();
		ensure!(
			vecs.len() == 3 && vecs[2].len() > 0,
			"error not found valid did"
		);

		let did_ele_hex = Self::from_base58(vecs[2].clone()).map_err(|_| "error Bad Base58")?;

		let receiver_did_hash = T::Hashing::hash(&did_ele_hex);
		let receiver = <did::Module<T>>::identity_of(receiver_did_hash);
		if receiver.is_some() {
			return Ok(receiver.unwrap());
		}

		Err("error parse did failed")
	}

	// Convert the base58 str to Vec<u8>
	fn from_base58(data: &str) -> Result<Vec<u8>, &'static str> {
		let radix = 58u32.to_biguint().unwrap();
		let mut x: BigUint = Zero::zero();
		let mut rad_mult: BigUint = One::one();

		for (idx, &byte) in data.as_bytes().iter().enumerate().rev() {
			let first_idx = B_ALPHA
				.iter()
				.enumerate()
				.find(|x| *x.1 == byte)
				.map(|x| x.0);
			match first_idx {
				Some(i) => {
					x = x + i.to_biguint().unwrap() * &rad_mult;
				}
				None => return Err("InvalidBase58Byte"),
			}

			rad_mult = &rad_mult * &radix;
		}

		let mut r = Vec::new();
		for _ in data.as_bytes().iter().take_while(|&x| *x == B_ALPHA[0]) {
			r.push(0);
		}
		if x > Zero::zero() {
			// TODO: use append when it becomes stable
			r.extend(x.to_bytes_be());
		}
		Ok(r)
	}
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		match call {
			Call::update_enevt_htlc(_) => Ok(ValidTransaction {
				priority: TransactionPriority::max_value(),
				requires: vec![],
				provides: vec![0.encode()],
				longevity: TransactionLongevity::max_value(),
				propagate: true,
			}),
			_ => UnknownTransaction::NoUnsignedValidator.into(),
		}
	}
}
