#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::app_crypto::{KeyTypeId, RuntimeAppPublic};
use codec::{Decode, Encode};
use primitives::{offchain::Duration, offchain::HttpRequestId, offchain::HttpRequestStatus};
use rstd::{prelude::*, result::Result, vec::Vec};
use sp_runtime::{
    traits::Member, traits::Hash,
    transaction_validity::{
        TransactionValidity, TransactionPriority, ValidTransaction, UnknownTransaction, TransactionLongevity}
};
use support::{decl_event, decl_module, decl_storage, ensure, Parameter, StorageMap, StorageValue,
    dispatch::Result as dispatch_result, weights::SimpleDispatchInfo};
use system::{offchain::SubmitUnsignedTransaction, ensure_none, ensure_signed, ensure_root};
use simple_json::{self, json::JsonValue};
use hex::FromHex;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"orin");
pub const BUFFER_LEN: usize = 2048;

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

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct EventLogSource {
    event_name: Vec<u8>,
    event_url: Vec<u8>,
}

// Config event type and source urls
pub const FETCHE_EVENT_LOGS: [(&'static [u8], &'static [u8]); 1] = [
    (b"HTLC", b"https://api-ropsten.etherscan.io/api?module=logs&action=getLogs&fromBlock=379224&toBlock=latest&address=0x16D5195Fe8c6Ba98b2f61A9a787BC0Bde19e3f6F"),
];

// Config event log json fields
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

const STATUS_OK: &'static str = "1";
const MESSAGE_OK: &'static str = "OK";
const STR_PREFIX: &'static str = "0x";

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct EventHTLC<BlockNumber, Balance, Hash, AccountId>
where
    BlockNumber: PartialEq + Eq + Decode + Encode,
{
	eth_contract_addr: Vec<u8>,
	htlc_block_number: BlockNumber,
    event_block_number: BlockNumber,
    expire_height: u32,
    random_number_hash: Vec<u8>,
    swap_id: Hash,
	event_timestamp: u64,
	htlc_timestamp: u64,
    sender_addr: Vec<u8>,
    sender_chain_type: HTLCChain,
    receiver_addr: AccountId,
    receiver_chain_type: HTLCChain,
	recipient_addr: Vec<u8>,
	out_amount: Balance,
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

pub trait Trait: balances::Trait + timestamp::Trait {
    /// The identifier type for an authority.
    type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// A dispatchable call type.
    type Call: From<Call<Self>>;
    /// A transaction submitter.
    type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Oracle {
        /// Stores the locked pra tokens
		pub PraTokenAddr get(pra_token_addr): Option<T::AccountId>;

		/// The current set of keys that may call update
		pub Authorities get(authorities) config(): Vec<T::AccountId>;

		/// Stores offchain request jobs
		pub OcRequests get(oc_requests): Vec<EventLogSource>;

		/// Key is swap_id, value is EventHTLC
		pub SwapData get(swap_data): map T::Hash => Option<EventHTLC<T::BlockNumber, T::Balance, T::Hash, T::AccountId>>;

		/// Key is swap_id, Value is HTLCStates
		pub SwapStates get(swap_states): map T::Hash => Option<HTLCStates>;
    }
}

decl_event!(
    pub enum Event<T>
    where
		<T as system::Trait>::BlockNumber,
		<T as system::Trait>::AccountId,
		<T as system::Trait>::Hash,
		<T as balances::Trait>::Balance,
    {
		///Setup pra_token_addr
		INIT(AccountId),

		//receiver_addr, eth_contract_addr, htlc_block_number, expire_height, random_number_hash, swap_id, sender_addr, out_amount, htlc_timestamp
		UpdateHTLC(AccountId, Vec<u8>, BlockNumber, u32, Vec<u8>, Hash, Vec<u8>, Balance, u64),

		//ClaimHTLC

		//RefundHTLC
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        // Initializing event fetch jobs
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        pub fn kickoff_event_fetch(origin, pra_token_addr: T::AccountId) -> dispatch_result {
			ensure_root(origin)?;

			runtime_io::misc::print_utf8(b"======== kickoff event fetch jobs");
            <Self as Store>::OcRequests::kill();

            for event_log_info in FETCHE_EVENT_LOGS.iter() {
                let event_log = EventLogSource {
                    event_name: event_log_info.0.to_vec(),
                    event_url: event_log_info.1.to_vec(),
                };

                <Self as Store>::OcRequests::mutate(|v|
                    v.push(event_log)
                );
			}

			<PraTokenAddr<T>>::put(pra_token_addr.clone());
			Self::deposit_event(RawEvent::INIT(pra_token_addr));

            Ok(())
        }

        // Kill all event fetch jobs
        #[weight = SimpleDispatchInfo::FixedNormal(500_000)]
        pub fn kill_event_fetch(origin) -> dispatch_result {
            ensure_root(origin)?;

            runtime_io::misc::print_utf8(b"======== kill event fetch jobs");
            <Self as Store>::OcRequests::kill();

            Ok(())
		}

		// Add a new authority to the set of keys that are allowed to update.
		pub fn add_authority(origin, who: T::AccountId) -> dispatch_result {
			ensure_root(origin)?;

			if !Self::is_authority(&who) {
				<Authorities<T>>::mutate(|l| l.push(who));
			}
			Ok(())
		}

        // Runs after every block.
        fn offchain_worker(now: T::BlockNumber) {
            Self::offchain_events(now);
        }

		// Update htlc and status, TODO: add auth control
		fn update_enevt_htlc(origin, htlc: EventHTLC<T::BlockNumber, T::Balance, T::Hash, T::AccountId>) {
			ensure_none(origin)?;

			ensure!(!<SwapData<T>>::exists(htlc.swap_id), "htlc already exists");
			ensure!(!<SwapStates<T>>::exists(htlc.swap_id), "htlc already exists");

			<SwapData<T>>::insert(htlc.swap_id, &htlc);
			<SwapStates<T>>::insert(htlc.swap_id, HTLCStates::OPEN);

			Self::deposit_event(RawEvent::UpdateHTLC(htlc.receiver_addr, htlc.eth_contract_addr, htlc.htlc_block_number, htlc.expire_height,
				htlc.random_number_hash, htlc.swap_id, htlc.sender_addr, htlc.out_amount, htlc.htlc_timestamp));
		}
    }
}

impl<T: Trait> Module<T> {
    fn offchain_events(now: T::BlockNumber) {
        for fetch_info in Self::oc_requests() {
            let res = Self::fetch_events(fetch_info.event_name, fetch_info.event_url);

            if let Err(err_msg) = res {
				runtime_io::misc::print_utf8(b"======== fetch events err msg");
                runtime_io::misc::print_utf8(err_msg.as_bytes());
            }
        }
    }

    fn fetch_events(src: Vec<u8>, remote_url: Vec<u8>) -> Result<(), &'static str> {
        runtime_io::misc::print_utf8(&remote_url);

		let pra_token_addr = Self::pra_token_addr();
        ensure!(pra_token_addr.is_some(), "pra_token_addr can not be empty");

        let remote_url_str: &str = core::str::from_utf8(&remote_url).unwrap();
        let res = Self::http_request_get(remote_url_str, None);

        if let Ok(buf) = res {
            let htlc = Self::parse_data(buf)?;

			if !<SwapData<T>>::exists(htlc.swap_id) {
				let call = Call::update_enevt_htlc(htlc);

				let result = T::SubmitTransaction::submit_unsigned(call);
				match result {
					Ok(_) => runtime_io::misc::print_utf8(b"execute off-chain worker success"),
					Err(_) => {
						runtime_io::misc::print_utf8(b"execute off-chain worker failed!");
						return Err("error happens when submit unsigned transaction")
					},
				}
			}
        }
        Ok(())
    }

    fn parse_data(res: [u8; BUFFER_LEN]) -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash, T::AccountId>, &'static str> {
        runtime_io::misc::print_utf8(b"======== start parse_json");
        runtime_io::misc::print_utf8(&res);

		let json_str = core::str::from_utf8(&res).map_err(|_| "err parse json from utf8")?;

        if let Ok(json_val) = simple_json::parse_json(json_str) {
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

            ensure!(status == b"1", "not valid status");
            ensure!(message == b"OK", "not valid message");

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
                                    topics.push(obj.iter().map(|c| *c as u8).collect::<Vec<u8>>());
                                }
                            }
                        }
                    } else if key == KEY_DATA {
                        if let JsonValue::String(obj) = v {
                            data = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
                        }
                    } else if key == KEY_BLOCK_NUMBER {
                        if let JsonValue::String(obj) = v {
                            event_block_number = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
                        }
                    } else if key == KEY_TIME_STAMP {
                        if let JsonValue::String(obj) = v {
                            event_time_stamp = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
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

                ensure!(topics.len() == 4, "not valid htlc topics length");
                ensure!(data.len() == 386, "not valid htlc data length");
				return Self::parse_htlc_event(contract_addr, topics, data, event_block_number, event_time_stamp, tx_hash, tx_index);
            }
        }

        Err("parse data fail")
    }

    fn parse_htlc_event(contract_addr: Vec<u8>, topics: Vec<Vec<u8>>, data: Vec<u8>,
						event_block_number: Vec<u8>, event_time_stamp: Vec<u8>, tx_hash: Vec<u8>, tx_index: Vec<u8>)
                        -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash, T::AccountId>, &'static str> {

		//indexed topics: _msgSender(Address); _receiverAddr(FixedBytes(32));_swapID(FixedBytes(32))
        let msg_sender = &topics[1][STR_PREFIX.len()..].to_vec();
        let receiver_addr = &topics[2][STR_PREFIX.len()..].to_vec();
		let swap_id = &topics[3][STR_PREFIX.len()..].to_vec();

		//#4310, <T::AccountId as Decode>::decode(&mut &hex::from_hex())
		let receiver_t = Vec::from_hex(&receiver_addr[STR_PREFIX.len()..]).map_err(|_| "err parse receiver_addr from utf8")?;
		let receiver_accnt = <T::AccountId as Decode>::decode(&mut receiver_t.as_slice()).map_err(|_| "err parse receiver_t from utf8")?;
		ensure!(receiver_accnt != Self::pra_token_addr().unwrap(), "Needs different accounts");

        //unindexed:_recipientAddr(Address);_randomNumberHash(FixedBytes(32));_timestamp(Uint(64));_expireHeight(Uint(256));_outAmount(Uint(256));_praAmount(Uint(256));
        let recipient_addr = &data[STR_PREFIX.len()..66].to_vec();
		let random_num_hash = &data[STR_PREFIX.len()+64..66+64].to_vec();
		let htlc_time_stamp = &data[STR_PREFIX.len()+64+64..66+64+64].to_vec();
		let expire_height = &data[STR_PREFIX.len()+64+64+64..66+64+64+64].to_vec();
		let out_amount = &data[STR_PREFIX.len()+64+64+64+64..66+64+64+64+64].to_vec();
		let pra_amount = &data[STR_PREFIX.len()+64+64+64+64+64..].to_vec();

		let event_ts = u64::from_str_radix(core::str::from_utf8(&event_time_stamp[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse event_time_stamp from utf8")?;
		let htlc_ts = u64::from_str_radix(core::str::from_utf8(&htlc_time_stamp[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse htlc_time_stamp from utf8")?;
		let event_block_num = u32::from_str_radix(core::str::from_utf8(&event_block_number[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse event_block_num from utf8")?;
		let expire_block_num = u32::from_str_radix(core::str::from_utf8(&expire_height[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse event_block_num from utf8")?;
		let event_out_amount = u32::from_str_radix(core::str::from_utf8(&out_amount[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse out_amount from utf8")?;
		let event_pra_amount = u32::from_str_radix(core::str::from_utf8(&pra_amount[STR_PREFIX.len()..]).unwrap(), 16)
				.map_err(|_| "err parse pra_amount from utf8")?;

		ensure!(event_out_amount > 0 && event_out_amount == event_pra_amount, "not valid out_amount or pra_amount");

		let htlc = EventHTLC {
			eth_contract_addr: contract_addr,
			event_block_number: T::BlockNumber::from(event_block_num),
			htlc_block_number: <system::Module<T>>::block_number(),
			out_amount: T::Balance::from(event_out_amount),
			expire_height: expire_block_num - event_block_num,
			random_number_hash: random_num_hash.clone(),
			swap_id: T::Hashing::hash(&swap_id[..]),
			event_timestamp: event_ts,
			htlc_timestamp: htlc_ts,
			sender_addr: msg_sender.clone(),
			sender_chain_type: HTLCChain::ETHMain,
			receiver_addr: receiver_accnt,
			receiver_chain_type: HTLCChain::PRA,
			recipient_addr: recipient_addr.clone(),
        };

        Ok(htlc)
	}

	//TODO: parse claim event
	fn parse_claim_event() -> Result<EventHTLC<T::BlockNumber, T::Balance, T::Hash, T::AccountId>, &'static str> {
		Err("")
	}

    fn http_request_get(uri: &str, header: Option<(&str, &str)>) -> Result<[u8; BUFFER_LEN], &'static str> {
        runtime_io::misc::print_utf8(b"request http request ========");
        let id: HttpRequestId = runtime_io::offchain::http_request_start("GET", uri, &[0]).unwrap();
        let deadline = runtime_io::offchain::timestamp().add(Duration::from_millis(5_000));

        if let Some((name, value)) = header {
            match runtime_io::offchain::http_request_add_header(id, name, value) {
                Ok(_) => (),
                Err(_) => return Err("Add request header failed"),
            };
        }

        match runtime_io::offchain::http_response_wait(&[id], Some(deadline))[0] {
            HttpRequestStatus::Finished(200) => (),
            _ => return Err("Request failed"),
        }

        let mut buf = Vec::with_capacity(BUFFER_LEN as usize);
        buf.resize(BUFFER_LEN as usize, 0);

        let res = runtime_io::offchain::http_response_read_body(id, &mut buf, Some(deadline));
        match res {
            Ok(_len) => {
                let result = &buf[..BUFFER_LEN];
                let mut res: [u8; BUFFER_LEN] = [0; BUFFER_LEN];
                res.copy_from_slice(result);
                return Ok(res);
            }
            Err(_) => return Err("Parse body failed"),
        }
	}

	//Helper that confirms whether the given `AccountId` has auth
	fn is_authority(who: &T::AccountId) -> bool {
		Self::authorities().into_iter().find(|i| i == who).is_some()
	}

	//if HTLC exists
	fn is_swap_exist(swap_id: T::Hash) -> bool {
		let state = Self::swap_states(swap_id);

		state.is_some() && state.unwrap() != HTLCStates::INVALID
	}

	//if HTLC claimable
	fn is_claimable(swap_id: T::Hash) -> bool {
		let state = Self::swap_states(swap_id);

		if state.is_some() && state.unwrap() == HTLCStates::OPEN {
			let swap = Self::swap_data(swap_id);
			if swap.is_some() {
				let swap = swap.unwrap();
				if <system::Module<T>>::block_number() < swap.htlc_block_number + T::BlockNumber::from(swap.expire_height) {
					return true;
				}
			}
		}
		false
	}
}

impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
        match call {
            Call::update_enevt_htlc(_) => Ok(
			ValidTransaction {
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
