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
use support::{decl_event, decl_module, decl_storage, ensure, Parameter, StorageMap, StorageValue, dispatch::Result as dispatch_result};
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

pub const FETCHE_EVENT_LOGS: [(&'static [u8], &'static [u8]); 1] = [
    (b"HTLC", b"https://api-ropsten.etherscan.io/api?module=logs&action=getLogs&fromBlock=379224&toBlock=latest&address=0x16D5195Fe8c6Ba98b2f61A9a787BC0Bde19e3f6F"),
];

pub const ABI_DATA: &'static str = r#"{"status":"1","message":"OK","result":[{"address":"0x16d5195fe8c6ba98b2f61a9a787bc0bde19e3f6f","topics":["0x924028c31cbef81354a146f585e1c91ea6a9caa2a9880e0e2f195cb8894823aa","0x000000000000000000000000f7fea1722f9b27b0666919a5664bab486a4b18d3","0xc731f90c0df8fd2a27268bb7942ea7a53e0861ddd57227869645e5157f685913","0x952dc77591ca272bcb010e6acce188a078be41ca4598987ef122e28c2ae9d707"],"data":"0x000000000000000000000000cf5becb7245e2e6ee2e092f0bd63f6bd79ef19fe6c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005dca9f440000000000000000000000000000000000000000000000000000000000674f9800000000000000000000000000000000000000000000000000000000009896800000000000000000000000000000000000000000000000000000000000989680","blockNumber":"0x672888","timeStamp":"0x5dcaa1cb","gasPrice":"0x3b9aca00","gasUsed":"0x43bac","logIndex":"0x7","transactionHash":"0x196ee30fa9076bcb4b1e04a37df215ef754c27db7cdca926395116a2971ab1cf","transactionIndex":"0x39"},{"address":"0x16d5195fe8c6ba98b2f61a9a787bc0bde19e3f6f","topics":["0x924028c31cbef81354a146f585e1c91ea6a9caa2a9880e0e2f195cb8894823aa","0x000000000000000000000000603a2abcbb0414a5c13a8bb22c20daf2f9388ad8","0xef85676f7752cb4d76942df4fff5c46a4e57dec88aa96766ddafe084cbe59421","0xbf19265f61734f9e5483b03aa5b97693dee83c88858a2cda0de6fd55b01624fc"],"data":"0x000000000000000000000000cf5becb7245e2e6ee2e092f0bd63f6bd79ef19fe6c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005dd7c7610000000000000000000000000000000000000000000000000000000000684a1e00000000000000000000000000000000000000000000000000000000009896800000000000000000000000000000000000000000000000000000000000989680","blockNumber":"0x68230e","timeStamp":"0x5dd7c789","gasPrice":"0x1a13b8600","gasUsed":"0x40114","logIndex":"0x16","transactionHash":"0x42fb1b4b113a0fb9d0b2c8ce6cb888ff37bb70db4b789e300b5ed424413ad589","transactionIndex":"0x1c"}]}"#;

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

			Self::parse_abi_data();

			Ok(())
		}

		pub fn kill_pricefetch(origin) -> dispatch_result {
			let _ = ensure_signed(origin)?;

			runtime_io::misc::print_utf8(b"======== kill pricefetch");
			<Self as Store>::OcRequests::kill();

			Ok(())
		}

        // Runs after every block.
        fn offchain_worker(now: T::BlockNumber) {
			//TODO: check block_number to fetch only once per block
			Self::offchain(now);
        }

        pub fn submit_value(origin, value: BTCValue<T::BlockNumber>
            // signature: <T::AuthorityId as RuntimeAppPublic>::Signature
        ) {
            runtime_io::misc::print_utf8(b"submit value call happens ========");
            runtime_io::misc::print_num(value.price as u64);
            ensure_none(origin)?;

            // update value in storage
            <Values<T>>::insert(value.block_number, &value);
            <PriceValue>::put(32);

            Self::deposit_event(RawEvent::UpdateValue(value.price, value.block_number));
        }
    }
}

impl<T: Trait> Module<T> {
    fn offchain(now: T::BlockNumber) {
        <BlockNumber<T>>::put(now);

        let cmc_value = Self::request_gec_value();
        let cds_value = Self::request_cds_value();
        let nom_value = Self::request_nomics_value();

        let values: [u32; 3] = [cmc_value, cds_value, nom_value];
        runtime_io::misc::print_utf8(b"=====result values:");
        runtime_io::misc::print_num(cmc_value.clone() as u64) ;
        if let Some(average_value) = Self::average_values(values) {
            Self::update_value(average_value);
		}
    }

    fn parse_result(res: [u8; BUFFER_LEN], start: &str) -> Value {
        if let Ok(data) = core::str::from_utf8(&res) {
            let start_bytes = data.find(start).unwrap_or(0) + start.len();
            let end_bytes = start_bytes + 10;
            let price = &data[start_bytes..end_bytes];

            let mid_bytes = price.find(".").unwrap_or(0);
            let rs = &price[0..mid_bytes];
            return rs.replace(",", "").parse::<Value>().unwrap_or(0);
        } else {
            return 0;
        }
	}

	fn parse_abi_data() -> Option<Value> {
		runtime_io::misc::print_utf8(b"======== start parse_json");

		// let result = simple_json::parse_json(ABI_DATA).unwrap();
		// match result {
		// 	JsonValue::String(JsonObject) => {
		// 		let vec_of_u8s: Vec<u8> = JsonObject.iter().map(|c| *c as u8).collect();
		// 		let c: &[u8] = &vec_of_u8s;
		// 		runtime_io::misc::print_utf8(c);
		// 	},
		// 	_ => return None,
		// }

		runtime_io::misc::print_utf8(b"json should parsed ========");

		Some(1001u32)
	}

    // request limited
    fn _request_cmc_value() -> Value {
        // TODO: uri and api key should write into sotrage like authorisedKey
        let uri = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?id=1";
        let api_key_value = "20a084fd-afdd-4c81-8e95-08868a45fcaf";
        let api_key = "X-CMC_PRO_API_KEY";

        let header = Some((api_key, api_key_value));
        let res = Self::http_request_get(uri, header);
        match res {
            Ok(buf) => return Self::parse_result(buf, "price\":"),
            Err(_) => return 0,
        }
    }

    fn request_gec_value() -> Value {
        runtime_io::misc::print_utf8(b"request gec value========");
        let uri = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
        let res = Self::http_request_get(uri, None);
        match res {
            Ok(buf) => return Self::parse_result(buf, "usd\":"),
            Err(_) => return 0,
        }
    }

    fn request_cds_value() -> Value {
        runtime_io::misc::print_utf8(b"request cds value========");
        let uri = "https://api.coindesk.com/v1/bpi/currentprice/USD.json";
        let res = Self::http_request_get(uri, None);
        match res {
            Ok(buf) => return Self::parse_result(buf, "rate\":\""),
            Err(_) => return 0,
        }
    }

    fn request_nomics_value() -> Value {
        runtime_io::misc::print_utf8(b"request nomic value========");
        let uri = "https://api.nomics.com/v1/currencies/ticker?key=3d93bdca7ee51ad25fcf650f2883b92d&ids=BTC";
        let res = Self::http_request_get(uri, None);
        match res {
            Ok(buf) => return Self::parse_result(buf, "price\":\""),
            Err(_) => return 0,
        }
    }

    fn http_request_get(
        uri: &str,
        header: Option<(&str, &str)>,
    ) -> Result<[u8; BUFFER_LEN], &'static str> {
        // TODO: extract id, maybe use for other place
        runtime_io::misc::print_utf8(b"request http request ========");
        let id: HttpRequestId = runtime_io::offchain::http_request_start("GET", uri, &[0]).unwrap();
        let deadline = runtime_io::offchain::timestamp().add(Duration::from_millis(10_000));

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

        // set a fix len for result
        let mut buf = Vec::with_capacity(BUFFER_LEN as usize);
        buf.resize(BUFFER_LEN as usize, 0);

        let res = runtime_io::offchain::http_response_read_body(id, &mut buf, Some(deadline));
        match res {
            Ok(_len) => {
                let result = &buf[..BUFFER_LEN];
                // runtime_io::misc::print_utf8(result);

                let mut res: [u8; BUFFER_LEN] = [0; BUFFER_LEN];
                res.copy_from_slice(result);
                return Ok(res);
            }
            Err(_) => return Err("Parse body failed"),
        }
    }

    fn average_values(values: [u32; 3]) -> Option<Value> {
        // 1. filter value == 0; if filter_values_count < 2, give up this round
        let values = values.iter().filter(|v| *v > &0).collect::<Vec<_>>();
        let count = values.len() as u32;
        if count < 2 {
            return None;
        }

        // 2. calculate variance, variance_threshold = 10_000;
        // The threshold could be put in the storage and set by authoritor
        let mean = values.iter().map(|v| **v).sum::<u32>() / count;
        let variance = values
            .iter()
            .map(|v| {
                let diff = mean as i32 - (**v as i32);
                diff * diff
            })
            .sum::<i32>()
            / count as i32;

        if variance > 10_000 {
            return None;
        }

        Some(mean)
    }

    fn update_value(value: Value) -> Result<(), &'static str> {
        let block_number = Self::block_number();
        runtime_io::misc::print_utf8(b"in update ========");
        ensure!(block_number.is_some(), "block number can not be empty");
        let block_number = block_number.unwrap();

        runtime_io::misc::print_utf8(b"update btc value: ========");
        runtime_io::misc::print_num(value as u64);
        let btc_value = BTCValue {
            block_number,
            price: value,
        };

        let call = Call::submit_value(btc_value);

        // submit unsigned transaction
        let result = T::SubmitTransaction::submit_unsigned(call);
        match result {
            Ok(_) => runtime_io::misc::print_utf8(b"execute off-chain worker success"),
            Err(_) => {
                runtime_io::misc::print_utf8(b"execute off-chain worker failed!");
                return Err("error happens when submit unsigned transaction of btc value")
            },
        }

        runtime_io::misc::print_utf8(b"=========end of update btc value: ========");
        Ok(())
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
