#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::{self as system, ensure_signed};
use num_traits::float::FloatCore;
use simple_json::{self, json::JsonValue};
use sp_core::{
	crypto::KeyTypeId,
	offchain::{Duration, HttpRequestId, HttpRequestStatus},
};
use sp_runtime::DispatchResult;
use sp_std::{
	convert::{Into, TryInto},
	prelude::*,
	result::Result,
	vec::Vec,
};
use utilities::FixedU128;

pub type Price = FixedU128;
pub type CurrencyId = u32;

//Note:
// 1. price precision is 8
// 2. quote_currency is USD
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct PriceValue<Moment, AccountId> {
	currency_id: CurrencyId,
	quote_currency_id: CurrencyId,
	price: Price,
	account: AccountId,
	timestamp: Moment,
}

//  automates offchain fetching every certain blocks. Set 0 disable this feature.
pub const BLOCK_DURATION: u64 = 5;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ofpf");

pub mod crypto {
	pub use super::KEY_TYPE;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, KEY_TYPE);
}

pub const FETCHED_JOBS: [(&[u8], &[u8], &[u8]); 4] = [
	(
		b"BTC",
		b"coincap",
		b"https://api.coincap.io/v2/assets/bitcoin",
	),
	(
		b"BTC",
		b"cryptocompare",
		b"https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD",
	),
	(
		b"PRM",
		b"coinmarketcap",
		b"https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?id=2275",
	),
	(
		b"PRM",
		b"coingecko",
		b"https://api.coingecko.com/api/v3/simple/price?ids=prochain&vs_currencies=usd",
	),
];

const KEY_ID: &'static str = "id";
const KEY_PRICE_USD: &'static str = "priceUsd";
const CMC_KEY: &'static str = "X-CMC_PRO_API_KEY";
const CMC_KEY_VALUE: &'static str = "20a084fd-afdd-4c81-8e95-08868a45fcaf";

pub trait Trait: system::Trait + pallet_timestamp::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Call: From<Call<Self>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Prices {
		/// Current set of keys that may feed data
		pub Authorities get(authorities) config(): Option<T::AccountId>;

		/// The currency_id price storage
		pub Values get(values): map CurrencyId => Option<PriceValue<T::Moment, T::AccountId>>;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		NoPermission,
	}
}

decl_event!(
	pub enum Event<T>
	where
        <T as pallet_timestamp::Trait>::Moment,
        <T as system::Trait>::AccountId,
    {
        ///currency_id, quote_currency_id, price, who, timestamp
        NewPrice(CurrencyId, CurrencyId, Price, AccountId, Moment),
    }
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		fn feed_value(origin, currency_id: CurrencyId, quote_currency_id: CurrencyId, price: Price) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::can_feed_data(&sender), Error::<T>::NoPermission);

			let now = <pallet_timestamp::Module<T>>::get();

			let value = PriceValue {
				currency_id: currency_id,
				quote_currency_id: quote_currency_id,
				price: price,
				account: sender.clone(),
				timestamp: now,
			};
			<Values<T>>::insert(&currency_id, value);

			Self::deposit_event(RawEvent::NewPrice(currency_id, quote_currency_id, price, sender, now));

			Ok(())
		}

		fn offchain_worker(now: T::BlockNumber) {
			if BLOCK_DURATION > 0 && (TryInto::<u64>::try_into(now).ok().unwrap()) % BLOCK_DURATION == 0 {
				for (sym, src, url) in FETCHED_JOBS.iter() {
					let symbol = core::str::from_utf8(sym).unwrap();
					let src_str = core::str::from_utf8(src).unwrap();
					let url_str = core::str::from_utf8(src).unwrap();

					if let Ok(price) = Self::fetch_price(symbol, src_str, url_str) {
						debug::info!("fetch price OK: {:?} {:>}", &price, &url_str);
					} else {
						debug::error!("fetch price error");
					}
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn can_feed_data(who: &T::AccountId) -> bool {
		let auth = Self::authorities();
		auth.is_some() && auth.unwrap() == who.clone()
	}

	fn get_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
		let base = Self::values(base_currency_id)?;
		if base.quote_currency_id == quote_currency_id {
			return Some(base.price);
		} else {
			let quote = Self::values(quote_currency_id)?;
			quote.price.checked_div(&(base.price))
		}
	}

	fn fetch_price(symbol: &str, src: &str, url: &str) -> Result<Price, &'static str> {
		debug::info!("fetch price: {:?}:{:?}:{:?}", &symbol, &src, &url);

		let mut header: Option<(&str, &str)> = None;

		if src.as_bytes() == b"coinmarketcap" {
			header = Some((CMC_KEY, CMC_KEY_VALUE));
		}
		let res = Self::http_request_get(&url, header);
		if let Ok(buf) = res {
			let json_str = core::str::from_utf8(&buf).map_err(|_| "res from_utf8 error")?;
			let json_val: JsonValue =
				simple_json::parse_json(&json_str).map_err(|_| "JSON res parsing error")?;

			let price: Price = match src {
				src_t if src_t.as_bytes() == b"coincap" => {
					Self::parse_from_coincap(symbol, json_val)
						.map_err(|_| "parse_from_coincap error")
				}
				src_t if src_t.as_bytes() == b"cryptocompare" => {
					Self::parse_from_cryptocompare(json_val)
						.map_err(|_| "parse_from_cryptocompare error")
				}
				src_t if src_t.as_bytes() == b"coinmarketcap" => {
					Self::parse_from_cmc(symbol, json_val).map_err(|_| "parse_from_cmc error")
				}
				src_t if src_t.as_bytes() == b"coingecko" => {
					Self::parse_from_coingecko(symbol, json_val)
						.map_err(|_| "parse_from_coingecko error")
				}
				_ => Err("error Unknown src"),
			}?;
			return Ok(price);
		}
		Err("error http_request_get failed")
	}

	fn parse_from_coincap(symbol: &str, json_val: JsonValue) -> Result<Price, &'static str> {
		debug::info!("parse_from_coincap: {:?}", &json_val);

		let mut id = Vec::new();
		let mut price_usd = Vec::new();

		if json_val.get_object().len() > 0 {
			let data = json_val.get_object()[0].1.get_object();

			data.iter()
				.filter(|(k, _)| {
					let key: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					KEY_ID.as_bytes().to_vec() == key || KEY_PRICE_USD.as_bytes().to_vec() == key
				})
				.for_each(|(k, v)| {
					let vec_of_u8s: Vec<u8> = k.iter().map(|c| *c as u8).collect();
					let key = core::str::from_utf8(&vec_of_u8s).unwrap();

					if key == KEY_ID {
						if let JsonValue::String(obj) = v {
							id = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
						}
					} else if key == KEY_PRICE_USD {
						if let JsonValue::String(obj) = v {
							price_usd = obj.iter().map(|c| *c as u8).collect::<Vec<u8>>();
						}
					}
				});

			let id_str = core::str::from_utf8(&id);
			if id_str.is_err() || id_str.unwrap() != symbol {
				return Err("error parse_from_coincap, symbol not match");
			}

			let val_f64: f64 = core::str::from_utf8(&price_usd[..])
				.map_err(|_| "error value_str not valid utf8")?
				.parse::<f64>()
				.map_err(|_| "fetch_price_from_coincap: val_u8 parsing to f64 error")?;

			//Note: precision is 8
			let val_u128: u128 = (val_f64 * 100000000.).round() as u128;
			return Ok(FixedU128::from_natural(val_u128));
		}
		Err("error parse_from_coincap not valid json_val")
	}

	fn parse_from_cryptocompare(json_val: JsonValue) -> Result<Price, &'static str> {
		debug::info!("parse_from_cryptocompare: {:?}", &json_val);

		let val_f64: f64 = json_val.get_object()[0].1.get_number_f64();

		//Note: precision is 8
		let val_u128: u128 = (val_f64 * 100000000.).round() as u128;

		Ok(FixedU128::from_natural(val_u128))
	}

	fn parse_from_cmc(symbol: &str, json_val: JsonValue) -> Result<Price, &'static str> {
		debug::info!("parse_from_cmc: {:?}", &json_val);

		Err("error parse_from_cmc failed")
	}

	fn parse_from_coingecko(symbol: &str, json_val: JsonValue) -> Result<Price, &'static str> {
		debug::info!("parse_from_coingecko: {:?}", &json_val);

		Err("error parse_from_coingecko failed")
	}

	fn average_prices(prices: Vec<u128>) -> Option<Price> {
		if prices.len() == 0 {
			return None;
		}

		Some(FixedU128::from_natural(0))
	}

	fn http_request_get(uri: &str, header: Option<(&str, &str)>) -> Result<Vec<u8>, &'static str> {
		let id: HttpRequestId = sp_io::offchain::http_request_start("GET", uri, &[0]).unwrap();
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(10_000));

		if let Some((name, value)) = header {
			match sp_io::offchain::http_request_add_header(id, name, value) {
				Ok(_) => (),
				Err(_) => return Err("Add request header failed"),
			};
		}

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
}
