#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::app_crypto::{KeyTypeId, RuntimeAppPublic};
use codec::{Decode, Encode};
use primitives::offchain::{Duration, HttpRequestId, HttpRequestStatus};
use rstd::result::Result;
use rstd::vec::Vec;
use sp_runtime::{
    traits::Member,
    transaction_validity::{
        TransactionValidity, InvalidTransaction, ValidTransaction, UnknownTransaction, TransactionLongevity,
        TransactionPriority
    }
};
use support::{decl_event, decl_module, decl_storage, ensure, Parameter, StorageMap, StorageValue,
	dispatch::Result as dispatch_result, weights::SimpleDispatchInfo};
use system::offchain::SubmitUnsignedTransaction;
use system::{ensure_none, ensure_signed};
use rstd::prelude::*;

use simple_json::{self, json::JsonValue, parser::Parser};

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

// TODO: add Value to Trait, config outside
pub type Value = u32;

// TODO: BTCValue is just an example, feel free to replace it with another name
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct BTCValue<BlockNumber>
where
    BlockNumber: PartialEq + Eq + Decode + Encode,
{
    block_number: BlockNumber,
    price: Value,
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
const KEY_TRX_HASH: &'static str = "transactionHash";
const KEY_TRX_INDEX: &'static str = "transactionIndex";

pub trait Trait: timestamp::Trait {
    /// The identifier type for an authority.
    type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// A dispatchable call type.
    type Call: From<Call<Self>>;
    /// A transaction submitter.
    type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Oracle {
        pub BlockNumber get(block_number): Option<T::BlockNumber>;

        /// Provide price value for external api consuming
        pub PriceValue get(price_value): Option<u32>;

        /// Values for specific block_number
        pub Values get(values): map T::BlockNumber => Option<BTCValue<T::BlockNumber>>;

		pub OcRequests get(oc_requests): Vec<EventLogSource>;
    }
}

decl_event!(
    pub enum Event<T>
    where
		<T as system::Trait>::BlockNumber,
    {
        UpdateValue(Value, BlockNumber),
    }
);

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;

		// Initializing event fetch tasks
		#[weight = SimpleDispatchInfo::FixedNormal(500_000)]
		pub fn kickoff_pricefetch(origin) -> dispatch_result {
			let who = ensure_signed(origin)?;

			runtime_io::misc::print_utf8(b"======== kickoff pricefetch");

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

			Ok(())
		}

		// Kill all event fetch tasks
		#[weight = SimpleDispatchInfo::FixedNormal(500_000)]
		pub fn kill_pricefetch(origin) -> dispatch_result {
			let _ = ensure_signed(origin)?;

			runtime_io::misc::print_utf8(b"======== kill pricefetch");
			<Self as Store>::OcRequests::kill();

			Ok(())
		}

        // Runs after every block.
        fn offchain_worker(now: T::BlockNumber) {
			//TODO: check block_number to fetch only once per block
			Self::offchain_events(now);
        }

		// Submit values
        pub fn submit_value(origin, value: BTCValue<T::BlockNumber>) {
            ensure_none(origin)?;

            // update value in storage
            <Values<T>>::insert(value.block_number, &value);
            <PriceValue>::put(32);

            Self::deposit_event(RawEvent::UpdateValue(value.price, value.block_number));
        }
    }
}

impl<T: Trait> Module<T> {
	fn offchain_events(now: T::BlockNumber) {
		<BlockNumber<T>>::put(now);

		for fetch_info in Self::oc_requests() {
			let res = Self::fetch_events(fetch_info.event_name, fetch_info.event_url);

			if let Err(err_msg) = res {
				runtime_io::misc::print_utf8(err_msg.as_bytes());
			}
		}
	}

	fn fetch_events(src: Vec<u8>, remote_url: Vec<u8>) -> Result<(), &'static str> {
		runtime_io::misc::print_utf8(&src);
		runtime_io::misc::print_utf8(&remote_url);
		runtime_io::misc::print_utf8(b"--- fetch_events");

		let remote_url_str: &str = core::str::from_utf8(&remote_url).unwrap();
		let res = Self::http_request_get(remote_url_str, None);

		let block_number = Self::block_number();
		ensure!(block_number.is_some(), "block number can not be empty");
		let block_number = block_number.unwrap();

		if let Ok(buf) = res {
			let value = Self::parse_data(buf).unwrap();

			let btc_value = BTCValue {
				block_number,
				price: value,
			};

			let call = Call::submit_value(btc_value);
			let result = T::SubmitTransaction::submit_unsigned(call);
			match result {
            	Ok(_) => runtime_io::misc::print_utf8(b"execute off-chain worker success"),
            	Err(_) => {
                	runtime_io::misc::print_utf8(b"execute off-chain worker failed!");
                	return Err("error happens when submit unsigned transaction")
            	},
        	}
		}
		Ok(())
	}

	fn parse_data(res: [u8; BUFFER_LEN]) -> Option<Value> {
		runtime_io::misc::print_utf8(b"======== start parse_json");
		runtime_io::misc::print_utf8(&res);

		let json_str: &str = core::str::from_utf8(&res).unwrap();
		let json_val: JsonValue = simple_json::parse_json(json_str).unwrap();

		let mut message = Vec::new();;
		let mut status = Vec::new();;
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

		runtime_io::misc::print_utf8(b"======== got parsed event data");
		runtime_io::misc::print_utf8(message.as_slice());
		runtime_io::misc::print_utf8(status.as_slice());

		Some(1u32)
	}

    fn http_request_get(
        uri: &str,
        header: Option<(&str, &str)>,
    ) -> Result<[u8; BUFFER_LEN], &'static str> {
        // TODO: extract id, maybe use for other place
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
}

impl<T: Trait> support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
        match call {
            Call::submit_value(_) => Ok(ValidTransaction {
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
