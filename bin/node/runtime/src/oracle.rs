use sr_primitives::app_crypto::{KeyTypeId, RuntimeAppPublic};
use codec::{Decode, Encode};
use primitives::offchain::{Duration, HttpRequestId, HttpRequestStatus};
use rstd::result::Result;
use rstd::vec::Vec;
use sr_primitives::{
    traits::Member,
    transaction_validity::{
        TransactionValidity, InvalidTransaction, ValidTransaction, UnknownTransaction, TransactionLongevity,
        TransactionPriority
    }
};
use support::{decl_event, decl_module, decl_storage, ensure, Parameter, StorageMap, StorageValue};
use system::offchain::SubmitUnsignedTransaction;
use system::{ensure_none, ensure_signed};
use rstd::prelude::*;

#[cfg(feature = "std")]
use simple_json::{self, json::JsonValue, parser::Parser};

#[cfg(feature = "std")]
use ethabi::{self, Event, Contract};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"orin");
pub const BUFFER_LEN: usize = 2048;

pub mod sr25519 {
    mod app_sr25519 {
        use sr_primitives::app_crypto::{app_crypto, sr25519};
        app_crypto!(sr25519, super::super::KEY_TYPE);

        impl From<Signature> for sr_primitives::AnySignature {
            fn from(sig: Signature) -> Self {
                sr25519::Signature::from(sig).into()
            }
        }
    }

    /// An oracle signature using sr25519 as its crypto.
    // pub type AuthoritySignature = app_sr25519::Signature;

    /// An oracle identifier using sr25519 as its crypto.
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

pub const HTLC_ABI: &'static [u8] = r#"[{"constant":false,"inputs":[{"name":"_randomNumberHash","type":"bytes32"},{"name":"_timestamp","type":"uint64"},{"name":"_heightSpan","type":"uint256"},{"name":"_recipientAddr","type":"address"},{"name":"_outAmount","type":"uint256"},{"name":"_praAmount","type":"uint256"},{"name":"_receiverAddr","type":"string"}],"name":"htlc","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[{"name":"_swapID","type":"bytes32"}],"name":"isSwapExist","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":false,"inputs":[{"name":"_swapID","type":"bytes32"}],"name":"refund","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":false,"inputs":[{"name":"_swapID","type":"bytes32"},{"name":"_randomNumber","type":"bytes32"}],"name":"claim","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"nonpayable","type":"function"},{"constant":true,"inputs":[{"name":"_swapID","type":"bytes32"}],"name":"claimable","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_swapID","type":"bytes32"}],"name":"refundable","outputs":[{"name":"","type":"bool"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_swapID","type":"bytes32"}],"name":"queryOpenSwap","outputs":[{"name":"_randomNumberHash","type":"bytes32"},{"name":"_timestamp","type":"uint64"},{"name":"_expireHeight","type":"uint256"},{"name":"_outAmount","type":"uint256"},{"name":"_sender","type":"address"},{"name":"_recipient","type":"address"}],"payable":false,"stateMutability":"view","type":"function"},{"constant":true,"inputs":[{"name":"_randomNumberHash","type":"bytes32"},{"name":"_swapSender","type":"address"}],"name":"calSwapID","outputs":[{"name":"","type":"bytes32"}],"payable":false,"stateMutability":"pure","type":"function"},{"constant":true,"inputs":[],"name":"PraContractAddr","outputs":[{"name":"","type":"address"}],"payable":false,"stateMutability":"view","type":"function"},{"inputs":[{"name":"_praContract","type":"address"}],"payable":false,"stateMutability":"nonpayable","type":"constructor"},{"anonymous":false,"inputs":[{"indexed":true,"name":"_msgSender","type":"address"},{"indexed":false,"name":"_recipientAddr","type":"address"},{"indexed":true,"name":"_receiverAddr","type":"string"},{"indexed":true,"name":"_swapID","type":"bytes32"},{"indexed":false,"name":"_randomNumberHash","type":"bytes32"},{"indexed":false,"name":"_timestamp","type":"uint64"},{"indexed":false,"name":"_expireHeight","type":"uint256"},{"indexed":false,"name":"_outAmount","type":"uint256"},{"indexed":false,"name":"_praAmount","type":"uint256"}],"name":"HTLC","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"_msgSender","type":"address"},{"indexed":false,"name":"_recipientAddr","type":"address"},{"indexed":true,"name":"_receiverAddr","type":"string"},{"indexed":true,"name":"_swapID","type":"bytes32"},{"indexed":false,"name":"_randomNumber","type":"bytes32"}],"name":"Claimed","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"_msgSender","type":"address"},{"indexed":false,"name":"_recipientAddr","type":"address"},{"indexed":true,"name":"_receiverAddr","type":"string"},{"indexed":true,"name":"_swapID","type":"bytes32"},{"indexed":false,"name":"_randomNumberHash","type":"bytes32"}],"name":"Refunded","type":"event"}]"#.as_bytes();

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
        /// The key used to sign the payload
        /// TODO: the type may change to `AuthorityId`
        pub AuthorisedKey get(authorised_key): Option<T::AccountId>;

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
        <T as system::Trait>::AccountId,
    {
        SetAuthority(AccountId),
        UpdateValue(Value),
    }
);

// The module's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        fn deposit_event() = default;

        // Clean the state on initialization of the block
        fn on_initialize(_block: T::BlockNumber) {
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
        }

        // Runs after every block.
        fn offchain_worker(now: T::BlockNumber) {
            // FIXME: only request a series of request at once
            // let block_number = Self::block_number();
            // if let Some(block_number) = block_number {
            //     let value = Self::values(block_number);
            //     if value.is_some() {
            //         Self::offchain(now);
            //     }
            // } else {
            //     Self::offchain(now);
			// }
			Self::offchain(now);
        }

        // Simple authority management: init authority key
        pub fn set_authority(origin) {
            // Should be protected by a root-call (e.g. through governance like `sudo`).
            // TODO: let sender = ensure_root(origin)?;
            let sender = ensure_signed(origin)?;

            <AuthorisedKey<T>>::put(sender.clone());

			Self::deposit_event(RawEvent::SetAuthority(sender));

			let json_val: JsonValue = simple_json::parse_json(&HTLC_ABI).unwrap();
        }

        pub fn submit_value(origin, value: BTCValue<T::BlockNumber>
            // signature: <T::AuthorityId as RuntimeAppPublic>::Signature
        ) {
            runtime_io::misc::print_utf8(b"submit value call happens ========");
            runtime_io::misc::print_num(value.price as u64);
            ensure_none(origin)?;

            // verify the signature
            let _public = Self::authorised_key();
            // TODO: public doesn't have `verify` function
            // let signature_valid = value.using_encoded(|encoded_value| {
            //     public.verify(&encoded_value, &signature)
            // });
            // ensure!(signature_valid, "Invalid value signature.");

            // update value in storage
            <Values<T>>::insert(value.block_number, &value);
            <PriceValue>::put(32);

            Self::deposit_event(RawEvent::UpdateValue(value.price));
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
        runtime_io::misc::print_utf8(&cmc_value.to_be_bytes());
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

        // let key = Self::authorised_key();
        // if let Some(_key) = key {
        runtime_io::misc::print_utf8(b"update btc value: ========");
        runtime_io::misc::print_num(value as u64);
        let btc_value = BTCValue {
            block_number,
            price: value,
        };
        // TODO: key doesn't have `sign` function
        // let signature = key.sign(&value.encode()).ok_or("Offchain error: signing failed!")?;
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
//        let current_session = <session::Module<T>>::current_index();
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
