#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::app_crypto::{KeyTypeId, RuntimeAppPublic};
use codec::{Decode, Encode};
use primitives::offchain::{Duration, HttpRequestId, HttpRequestStatus};
use rstd::result::Result;
use rstd::vec::Vec;
use sp_runtime::{
    traits::Member,
    transaction_validity::{
        TransactionValidity, TransactionPriority, ValidTransaction, UnknownTransaction, TransactionLongevity,
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
const KEY_TX_HASH: &'static str = "transactionHash";
const KEY_TX_INDEX: &'static str = "transactionIndex";

const STATUS_OK: &'static str = "1";
const MESSAGE_OK: &'static str = "OK";
const STR_PREFIX: &'static str = "0x";

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct EventHTLC<BlockNumber, Balance, Moment>
where
    BlockNumber: PartialEq + Eq + Decode + Encode,
{
    block_number: BlockNumber,
    out_amount: Balance,
    expire_height: u32,
    random_number_hash: Vec<u8>,
    swap_id: Vec<u8>,
    timestamp: Moment,
    sender_addr: Vec<u8>,
    sender_chain_type: u64,
    receiver_addr: Vec<u8>,
    receiver_chain_type: u64,
    recipient_addr: Vec<u8>,
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

        let block_number = Self::block_number();
        ensure!(block_number.is_some(), "block number can not be empty");
        let block_number = block_number.unwrap();

        let remote_url_str: &str = core::str::from_utf8(&remote_url).unwrap();
        let res = Self::http_request_get(remote_url_str, None);

        if let Ok(buf) = res {
            let value = Self::parse_data(buf)?;

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

    fn parse_data(res: [u8; BUFFER_LEN]) -> Result<Value, &'static str> {
        runtime_io::misc::print_utf8(b"======== start parse_json");
        runtime_io::misc::print_utf8(&res);

        let json_str = core::str::from_utf8(&res);
        if let Err(err) = json_str {
            return Err("err parse from utf8");
        }

        if let Ok(json_val) = simple_json::parse_json(json_str.unwrap()) {
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

            runtime_io::misc::print_utf8(status.as_slice());
            runtime_io::misc::print_utf8(message.as_slice());
            ensure!(status == b"1", "not valid status");
            ensure!(message == b"OK", "not valid message");

            for result in results.iter() {
                let mut contract_addr = Vec::new();
                let mut topics = Vec::new();
                let mut data = Vec::new();
                let mut block_number = Vec::new();
                let mut time_stamp = Vec::new();
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
                            block_number = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
                        }
                    } else if key == KEY_TIME_STAMP {
                        if let JsonValue::String(obj) = v {
                            time_stamp = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
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
                ensure!(data.len() == 386, "not valid htlc data");
                Self::parse_htlc_event(contract_addr, topics, data, block_number, time_stamp, tx_hash, tx_index)?;
            }
        }

        Ok(1u32)
        //Err("simple_json::parse_json parse failed")
    }

    fn parse_htlc_event(contract_addr: Vec<u8>, topics: Vec<Vec<u8>>, data: Vec<u8>,
                        block_number: Vec<u8>, time_stamp: Vec<u8>, tx_hash: Vec<u8>, tx_index: Vec<u8>)
                        -> rstd::result::Result<EventHTLC<T::BlockNumber, T::Balance, T::Moment>, &'static str> {
        runtime_io::misc::print_utf8(b"======== got parsed event log");

        //indexed topics: _msgSender(Address); _receiverAddr(FixedBytes(32));_swapID(FixedBytes(32))
        let msg_sender = &topics[1][STR_PREFIX.len()..].to_vec();
        let receiver_addr = &topics[2][STR_PREFIX.len()..].to_vec();
        let swap_id = &topics[3][STR_PREFIX.len()..].to_vec();

        //unindexed:_recipientAddr(Address);_randomNumberHash(FixedBytes(32));_timestamp(Uint(64));
                    //_expireHeight(Uint(256));_outAmount(Uint(256));_praAmount(Uint(256));
        let recipient_addr = &data[STR_PREFIX.len()..65].to_vec();
        let random_num_hash = &data[STR_PREFIX.len()+65..65+64].to_vec();
        let out_amount = &data[STR_PREFIX.len()+65..65+64].to_vec();

        /*
        runtime_io::misc::print_utf8(data.as_slice());
        runtime_io::misc::print_utf8(msg_sender.as_slice());
        runtime_io::misc::print_utf8(receiver_addr.as_slice());
        runtime_io::misc::print_utf8(swap_id.as_slice());
        runtime_io::misc::print_utf8(recipient_addr.as_slice());
        runtime_io::misc::print_utf8(random_num_hash.as_slice());
        runtime_io::misc::print_utf8(time_stamp.as_slice());
        runtime_io::misc::print_utf8(tx_hash.as_slice());
        runtime_io::misc::print_utf8(tx_index.as_slice());
        */

        let ts = u64::from_str_radix(core::str::from_utf8(&time_stamp[STR_PREFIX.len()..]).unwrap(), 16);
        if let Err(_err) = ts {
            return Err("err parse ts from json");
        }

        let block_num = u32::from_str_radix(core::str::from_utf8(&block_number[STR_PREFIX.len()..]).unwrap(), 16);
        if let Err(_err) = block_num {
            return Err("err parse block_num from json");
        }

        let htlc = EventHTLC {
             block_number: T::BlockNumber::from(block_num.unwrap()),
             out_amount: T::Balance::from(100u32),
             expire_height: 100,
             random_number_hash: random_num_hash.clone(),
             swap_id: swap_id.clone(),
             timestamp: <timestamp::Module<T>>::get(),
             sender_addr: msg_sender.clone(),
             sender_chain_type: 0,
             receiver_addr: receiver_addr.clone(),
             receiver_chain_type: 1,
             recipient_addr: recipient_addr.clone(),
        };

        Ok(htlc)
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
