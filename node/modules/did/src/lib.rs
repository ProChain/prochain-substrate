#![cfg_attr(not(feature = "std"), no_std)]

mod harsh;
mod check;
mod tests;

use codec::{Decode, Encode};
use sp_std::vec::Vec;
use frame_support::{
	decl_event, decl_module, decl_storage, decl_error, ensure,
	traits::{Currency, ReservableCurrency, ExistenceRequirement, Get},
};
use sp_runtime::{
	RuntimeDebug, DispatchResult, Permill,
	traits::{CheckedSub, CheckedAdd, Hash, SaturatedConversion,}
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_io::hashing::blake2_256;
use harsh::{HarshBuilder};

pub trait Trait: pallet_balances::Trait + pallet_timestamp::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub type Did = Vec<u8>;

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct ExternalAddress {
	btc: Vec<u8>,
	eth: Vec<u8>,
	eos: Vec<u8>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct LockedRecords<Balance, Moment> {
	locked_time: Moment,
	locked_period: Moment,
	locked_funds: Balance,
	rewards_ratio: u64,
	max_quota: u64,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct UnlockedRecords<Balance, Moment> {
	unlocked_time: Moment,
	unlocked_funds: Balance,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct MetadataRecord<AccountId, Hash, Balance, Moment> {
	address: AccountId,
	superior: Hash,
	creator: AccountId,
	did: Did,
	locked_records: Option<LockedRecords<Balance, Moment>>,
	unlocked_records: Option<UnlockedRecords<Balance, Moment>>,
	is_partner: bool,
	social_account: Option<Hash>,
	subordinate_count: u64,
	group_name: Option<Vec<u8>>,
	external_address: ExternalAddress
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// invlid type
		InvalidType,
		/// did does not exist
		DidNotExists
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DidModule {
		pub GenesisAccount get(genesis_account) config(): T::AccountId;
		pub BaseQuota get(base_quota) config(): u64;
		pub MinDeposit get(min_deposit) config(): T::Balance;
		pub FeeToPrevious get(fee_to_previous) config(): T::Balance;

		pub Identity get(identity): map T::AccountId => Option<(T::Hash, Did)>;
		pub IdentityOf get(identity_of): map T::Hash => Option<T::AccountId>;
		pub SocialAccount get(social_account): map T::Hash => T::Hash;
		pub Metadata get(metadata): map T::Hash => MetadataRecord<T::AccountId, T::Hash, T::Balance, T::Moment>;

		pub AllDidCount get(all_did_count): u64;
		pub UserKeys get(key_by_index): map T::Hash => T::Hash;
		pub DidIndices get(index_by_key) : map T::Hash => Vec<u8>;
		
		pub Authorities get(authorities): Option<T::AccountId>;
	}
}

decl_event! {
  pub enum Event<T>
  where
    <T as frame_system::Trait>::AccountId,
    <T as pallet_balances::Trait>::Balance,
    <T as pallet_timestamp::Trait>::Moment,
    {
        Created(Did, Vec<u8>, Did),
        Updated(Did, AccountId, Balance),
        Locked(Did, Balance, Moment, Moment, u64, u64),
        Unlocked(Did, Balance, Moment),
		Transfered(Did, Did, Balance, Vec<u8>),
		AddressAdded(Did, Vec<u8>, Vec<u8>),
		GroupNameSet(Did, Vec<u8>),
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn create(origin, pubkey: Vec<u8>, address: T::AccountId, did_type: Vec<u8>, superior: T::Hash, social_account: Option<Vec<u8>>, social_superior: Option<Vec<u8>>) {
			let sender = ensure_signed(origin)?;

			let did = Self::generate_did(&pubkey, &did_type);
			let user_key = T::Hashing::hash(&did);

			// make sure the did is new
			ensure!(!<Metadata<T>>::exists(&user_key), "did alread existed");
			ensure!(!<Identity<T>>::exists(&address), "you already have did");

			let mut superior_key = superior;
			let mut social_account_hash = None;

			if let Some(mut value) = social_account {
				// bind social account
				value.append(&mut did_type.to_vec());

				let social_hash = T::Hashing::hash(&value);
				social_account_hash = Some(social_hash);

				// one social account only can bind one did
				ensure!(!<SocialAccount<T>>::exists(&social_hash), "this social account has been bound");

				if let Some(mut value) = social_superior {
					value.append(&mut did_type.to_vec());

					let superior_hash = T::Hashing::hash(&value);
					ensure!(<SocialAccount<T>>::exists(&superior_hash), "the superior does not exsit");
					superior_key = Self::social_account(superior_hash);
				};
			}

			let mut superior_did = Vec::new();
			if <Metadata<T>>::exists(&superior_key) {
				let mut superior_metadata = Self::metadata(superior_key);
				if superior_metadata.address != Self::genesis_account() {
					let subordinate_count = superior_metadata.subordinate_count.checked_add(1).ok_or("overflow")?;

					ensure!(superior_metadata.locked_records.is_some(), "the superior does not locked funds");

					let locked_records = superior_metadata.locked_records.unwrap();
					let LockedRecords { max_quota, .. } = locked_records;
					ensure!(subordinate_count <= max_quota, "the superior's subordinate exceeds max quota");

					superior_metadata.subordinate_count = subordinate_count;
					superior_metadata.locked_records = Some(locked_records);
					superior_did = superior_metadata.did.clone();
					<Metadata<T>>::insert(&superior_key, superior_metadata);
				}
			}

			if social_account_hash.is_some() {
				let social_hash = social_account_hash.unwrap();
				<SocialAccount<T>>::insert(social_hash, &user_key);
			}

			// update metadata
			let metadata = MetadataRecord {
				address: address.clone(),
				superior: superior_key,
				creator: sender.clone(),
				did: did.clone(),
				locked_records: None,
				social_account: social_account_hash,
				unlocked_records: None,
				is_partner: false,
				subordinate_count: 0,
				group_name: None,
				external_address: ExternalAddress {
					btc: Vec::new(),
					eth: Vec::new(),
					eos: Vec::new(),
				},
			};
			<Metadata<T>>::insert(&user_key, metadata);

			// update address => did
			<Identity<T>>::insert(&address, (&user_key, &did));

			// update user_key => address
			<IdentityOf<T>>::insert(&user_key, &address);

			// update did count
			let all_did_count = Self::all_did_count();
			let new_count = all_did_count.checked_add(1)
					.ok_or("Overflow adding a new did")?;
			<AllDidCount>::put(new_count);

			let harsher = HarshBuilder::new().salt("prochain did").length(6).init().unwrap();
			let idx = harsher.encode(&[all_did_count]).unwrap();
			let idx_hash = T::Hashing::hash(&idx);

			<UserKeys<T>>::insert(&idx_hash, &user_key);
			<DidIndices<T>>::insert(&user_key, idx);

			// broadcast event
			Self::deposit_event(RawEvent::Created(did, pubkey, superior_did));
		}

		pub fn update(origin, to: T::AccountId) {
			let sender = ensure_signed(origin)?;

			// make sure did exists and new pubkey has not been bound
			let (user_key, did) = Self::identity(&sender).ok_or("did does not exist")?;
			ensure!(Self::identity(&to).is_none(), "the public key has been taken");

			let money = <pallet_balances::Module<T>>::free_balance(&sender)
					- T::TransferFee::get()
					- T::CreationFee::get();
			<pallet_balances::Module<T> as Currency<_>>::transfer(&sender, &to, money, ExistenceRequirement::AllowDeath,)?;

			// update address => did map
			<Identity<T>>::remove(&sender);
			<Identity<T>>::insert(&to, (&user_key, &did));

			// update user_key => address
			<IdentityOf<T>>::insert(user_key, &to);

			let mut metadata = Self::metadata(&user_key);
			metadata.address = to.clone();

			<Metadata<T>>::insert(user_key, metadata);

			Self::deposit_event(RawEvent::Updated(did, to, money));
		}

		// transfer fund by did
		pub fn transfer(origin, to_user: T::Hash, value: T::Balance, memo: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			let (from_user, _) = Self::identity(sender).ok_or("did does not exist")?;
			Self::transfer_by_did(from_user, to_user, value, memo)?;
		}

		// lock fund
		pub fn lock(origin, value: T::Balance, period: T::Moment) {
			let sender = ensure_signed(origin)?;

			let sender_balance = <pallet_balances::Module<T>>::free_balance(sender.clone());
			ensure!(sender_balance >= value + T::TransferFee::get(), "you dont have enough free balance");

			let (user_key, did) = Self::identity(&sender).ok_or("this account has no did yet")?;
			let mut metadata = Self::metadata(&user_key);

			// make sure the superior exists
			ensure!(<Metadata<T>>::exists(metadata.superior), "superior does not exsit");

			let level2_metadata = Self::metadata(metadata.superior);

			let locked_funds;
			let mut rewards_ratio = 20;// basis rewards_ratio is 20%

			if !metadata.is_partner {
				ensure!(value >= Self::min_deposit(), "you must lock at least 50 pra first time");

				let fee = Self::fee_to_previous();

				locked_funds = value - fee;

				let memo = "抵押分成".as_bytes().to_vec();


				// second level division
				if level2_metadata.superior != Default::default() {
					let fee1 = Permill::from_percent(80) * fee;
					let fee2 = Permill::from_percent(20) * fee;

					Self::transfer_by_did(user_key, metadata.superior, fee1, memo.clone())?;
					Self::transfer_by_did(user_key, level2_metadata.superior, fee2, memo.clone())?;
				} else {
					Self::transfer_by_did(user_key, metadata.superior, fee, memo)?;
				}

				<pallet_balances::Module<T>>::reserve(&sender, locked_funds)?;
				metadata.is_partner = true;
			} else {
				let locked_records = metadata.locked_records.unwrap();

				let old_locked_funds = locked_records.locked_funds;
				locked_funds = old_locked_funds + value;

				<pallet_balances::Module<T>>::reserve(&sender, value)?;
			}

			let max_quota = Self::balance_to_u64(locked_funds) * 10;

			if max_quota >= metadata.subordinate_count {
				rewards_ratio = 20;
			};

			let locked_time = <pallet_timestamp::Module<T>>::get();
			metadata.locked_records = Some(LockedRecords {
				locked_funds,
				rewards_ratio,
				max_quota,
				locked_time,
				locked_period: period,
			});

			<Metadata<T>>::insert(user_key, metadata);

			Self::deposit_event(RawEvent::Locked(did, locked_funds, locked_time, period, rewards_ratio, max_quota));
		}

		// unlock fund
		fn unlock(origin, value: T::Balance) {
			let sender = ensure_signed(origin)?;

			let reserved_balance = <pallet_balances::Module<T>>::reserved_balance(&sender);
			ensure!(reserved_balance >= value, "unreserved funds should equal or less than reserved funds");

			let (user_key, did) = Self::identity(&sender).ok_or("this account has no did yet")?;
			let mut metadata = Self::metadata(&user_key);
			ensure!(metadata.locked_records.is_some(), "you didn't lock funds before");

			let mut locked_records = metadata.locked_records.unwrap();
			let LockedRecords { locked_time, locked_period, locked_funds, .. } = locked_records;
			let now = <pallet_timestamp::Module<T>>::get();
			let unlock_till_time = locked_time.checked_add(&locked_period).ok_or("Overflow.")?;

			ensure!(now >= unlock_till_time, "unlock time has not reached");

			let unlocked_time = <pallet_timestamp::Module<T>>::get();
			let unlocked_records = UnlockedRecords {
				unlocked_time,
				unlocked_funds: value,
			};

			let new_locked_funds = locked_funds - value;
			let new_max_quota = Self::balance_to_u64(new_locked_funds) * 10;
			let rewards_ratio = if new_max_quota >= metadata.subordinate_count { 20 } else { 100 * (1 - new_max_quota / metadata.subordinate_count) as u64 };

			locked_records = LockedRecords {
				locked_funds: new_locked_funds,
				rewards_ratio,
				max_quota: new_max_quota,
				.. locked_records
			};

			metadata.unlocked_records = Some(unlocked_records);
			metadata.locked_records = Some(locked_records);

			<Metadata<T>>::insert(user_key, metadata);

			<pallet_balances::Module<T>>::unreserve(&sender, value);

			Self::deposit_event(RawEvent::Unlocked(did, value, unlocked_time));
		}

		// add external address
		fn add_external_address(origin, add_type: Vec<u8>, address: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			let (user_key, did) = Self::identity(&sender).ok_or("this account has no did yet")?;
			let mut metadata = Self::metadata(&user_key);
			let mut external_address = metadata.external_address;

			match &add_type[..] {
				b"btc" => {
					check::from(address.clone()).map_err(|_| "invlid bitcoin address")?;
					external_address.btc = address.clone();
				},
				b"eth" => {
					ensure!(check::is_valid_eth_address(address.clone()), "invlid eth account");
					external_address.eth = address.clone();
				},
				b"eos" => {
					ensure!(check::is_valid_eos_address(address.clone()), "invlid eos account");
					external_address.eos = address.clone();
				},
				_ => Err(Error::<T>::InvalidType)?,
			};

			metadata.external_address = external_address;

			<Metadata<T>>::insert(user_key, metadata);

			Self::deposit_event(RawEvent::AddressAdded(did, add_type, address));
		}

		fn set_group_name(origin, name: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			let (user_key, did) = Self::identity(&sender).ok_or("this account has no did yet")?;
			let mut metadata = Self::metadata(&user_key);

			ensure!(name.len() < 50, "group name is too long");
			ensure!(metadata.locked_records.is_some(), "you are not eligible to set group name");

			metadata.group_name = Some(name.clone());

			<Metadata<T>>::insert(user_key, metadata);

			Self::deposit_event(RawEvent::GroupNameSet(did, name));
		}
		
		fn init(origin, auth: T::AccountId) {
			ensure_root(origin)?;
			<Authorities<T>>::put(auth);
		}
		
		fn judge(origin, account: T::AccountId) {
			let sender = ensure_signed(origin)?;
			
			if Self::is_authority(&sender) {
				let (user_key, _) = Self::identity(&account).ok_or(Error::<T>::DidNotExists)?;
				let mut metadata = Self::metadata(&user_key);
				metadata.creator = account;
				<Metadata<T>>::insert(user_key, metadata);
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn u128_to_balance(input: u128) -> T::Balance {
		input.saturated_into()
	}

	fn balance_to_u64(input: T::Balance) -> u64 {
		input.saturated_into::<u64>()
	}

	fn is_sub(mut haystack: &[u8], needle: &[u8]) -> bool {
		if needle.len() == 0 { return true; }
		while !haystack.is_empty() {
			if haystack.starts_with(needle) { return true; }
			haystack = &haystack[1..];
		}
		false
	}
	
	fn is_authority(who: &T::AccountId) -> bool {
		let auth = Self::authorities();
		auth.is_some() && auth.unwrap() == who.clone()
	}

	fn generate_did(pubkey: &[u8], did_type: &[u8]) -> Vec<u8> {
		// 通过公钥生成hash值
		let mut hash = blake2_256(pubkey);

		// did的类型
		let did = did_type;
		let mut did = did.to_vec();

		// 	截取第一步生成的hash的前20位，将did类型附加在最前面
		did.append(&mut hash[..20].to_vec());

		// 将第二步生成的hash再次hash
		let mut ext_hash = blake2_256(&did[..]);

		// 截取第三步生成的hash的前4位，并附加到第二步生成的hash后面
		did.append(&mut ext_hash[..4].to_vec());

		did
	}
}

impl<T: Trait> Module<T> {
	pub fn transfer_by_did(from_user: T::Hash, to_user: T::Hash, value: T::Balance, memo: Vec<u8>) -> DispatchResult {
		ensure!(<Metadata<T>>::exists(&to_user), "dest did does not exist");
		ensure!(from_user != to_user, "you can not send money to yourself");

		// get sender balance and check
		let MetadataRecord { address: from_address, did: from_did, .. } = Self::metadata(&from_user);
		let sender_balance = <pallet_balances::Module<T>>::free_balance(&from_address);
		ensure!(sender_balance > value, "you dont have enough free balance");

		// get receiver balance
		let MetadataRecord { address: to_address, did: to_did, superior, .. } = Self::metadata(&to_user);
		let receiver_balance = <pallet_balances::Module<T>>::free_balance(&to_address);

		// check overflow
		sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
		receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

		// proceeds split
		let fee_type = b"ads";
		if Self::is_sub(&memo, fee_type) {
			let superior_address = Self::identity_of(superior).ok_or("superior does not find")?;
			
			let MetadataRecord { locked_records, ..} = Self::metadata(superior);
			let rewards_ratio = if locked_records.is_some() { locked_records.unwrap().rewards_ratio } else { 0 };
			
			let fee_to_superior = value.clone() * Self::u128_to_balance(rewards_ratio.into()) / Self::u128_to_balance(100);
			let fee_to_user = value.clone() * Self::u128_to_balance((100 - rewards_ratio).into()) / Self::u128_to_balance(100);

			<pallet_balances::Module<T> as Currency<_>>::transfer(&from_address, &superior_address, fee_to_superior, ExistenceRequirement::AllowDeath)?;
			<pallet_balances::Module<T> as Currency<_>>::transfer(&from_address, &to_address, fee_to_user, ExistenceRequirement::AllowDeath)?;
		} else {
			<pallet_balances::Module<T> as Currency<_>>::transfer(&from_address, &to_address, value, ExistenceRequirement::AllowDeath)?;
		}

		Self::deposit_event(RawEvent::Transfered(from_did, to_did, value, memo));

		Ok(())
	}
}
