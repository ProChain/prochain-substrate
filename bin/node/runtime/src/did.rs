use crate::check;
use codec::{Decode, Encode};
use rstd::vec::Vec;
use support::{
	decl_event, decl_module, decl_storage, ensure, traits::{Currency, ReservableCurrency, ExistenceRequirement}, dispatch::Result, print,
};
use sr_primitives::traits::{CheckedSub, CheckedAdd, Hash, SaturatedConversion};
use system::ensure_signed;
use runtime_io::hashing::blake2_256;
use harsh::{HarshBuilder};

pub trait Trait: balances::Trait + timestamp::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct ExternalAddress {
	btc: Vec<u8>,
	eth: Vec<u8>,
	eos: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct LockedRecords<Balance, Moment> {
	locked_time: Moment,
	locked_period: Moment,
	locked_funds: Balance,
	rewards_ratio: u64,
	max_quota: u64,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct UnlockRecords<Balance, Moment> {
	unlock_time: Moment,
	unlock_funds: Balance,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct MetadataRecord<AccountId, Hash, Balance, Moment> {
	address: AccountId,
	superior: Hash,
	creator: AccountId,
	did_ele: Vec<u8>,
	locked_records: Option<LockedRecords<Balance, Moment>>,
	unlock_records: Option<UnlockRecords<Balance, Moment>>,
	social_account: Option<Hash>,
	subordinate_count: u64,
	group_name: Option<Vec<u8>>,
	external_address: ExternalAddress
}

decl_storage! {
    trait Store for Module<T: Trait> as DidModule {
			GenesisAccount get(genesis_account) config(): T::AccountId;
			BaseQuota get(base_quota) config(): u64;
			MinDeposit get(min_deposit) config(): T::Balance;
			FeeToPrevious get(fee_to_previous) config(): T::Balance;

      Identity get(identity): map T::AccountId => T::Hash;
			IdentityOf get(identity_of): map T::Hash => Option<T::AccountId>;
			SocialAccount get(social_account): map T::Hash => T::Hash;
			Metadata get(metadata): map T::Hash => MetadataRecord<T::AccountId, T::Hash, T::Balance, T::Moment>;

			AllDidCount get(all_did_count): u64;
			AllDidsArray get(did_by_index): map Vec<u8> => T::Hash;
      AllDidsIndex: map T::Hash => Vec<u8>;
    }
}

decl_event! {
  pub enum Event<T>
  where
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as balances::Trait>::Balance,
    <T as timestamp::Trait>::Moment,
    {
        Created(AccountId, Hash),
        Updated(AccountId, Hash, Balance),
        Locked(AccountId, Balance, Moment),
        Unlock(AccountId, Balance),
				Transfered(Hash, Hash, Balance, Vec<u8>),
				AddressAdded(AccountId, Vec<u8>, Vec<u8>),
				GroupNameSet(AccountId, Vec<u8>),
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn create(origin, pubkey: Vec<u8>, address: T::AccountId, did_type: Vec<u8>, superior: T::Hash, social_account: Option<Vec<u8>>, social_superior: Option<Vec<u8>>) {
			let sender = ensure_signed(origin)?;

			let did_ele = Self::generate_did(&pubkey, &did_type);

			let did_hash = T::Hashing::hash(&did_ele);

			// make sure the did is new
			ensure!(!<Metadata<T>>::exists(&did_hash), "did alread existed");
			ensure!(!<Identity<T>>::exists(&address), "you already have did");

			let mut superior_did = superior;
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
					superior_did = Self::social_account(superior_hash);
				};
			}

			let mut superior_metadata = Self::metadata(superior_did);
			if superior_metadata.address != Self::genesis_account() && <Metadata<T>>::exists(&superior_did){
				let subordinate_count = superior_metadata.subordinate_count.checked_add(1).ok_or("overflow")?;

				ensure!(superior_metadata.locked_records.is_some(), "the superior does not locked funds");

				let locked_records = superior_metadata.locked_records.unwrap();
				let LockedRecords { max_quota, .. } = locked_records;
				ensure!(subordinate_count <= max_quota, "the superior's subordinate exceeds max quota");

				superior_metadata.subordinate_count = subordinate_count;
				superior_metadata.locked_records = Some(locked_records);
				<Metadata<T>>::insert(&superior_did, superior_metadata);
			}
			
			if social_account_hash.is_some() {
				let social_hash = social_account_hash.unwrap();
				<SocialAccount<T>>::insert(social_hash, &did_hash);
			}

			// update metadata
			let metadata = MetadataRecord {
					address: address.clone(),
					superior: superior_did,
					creator: sender.clone(),
					did_ele,
					locked_records: None,
					social_account: social_account_hash,
					unlock_records: None,
					subordinate_count: 0,
					group_name: None,
					external_address: ExternalAddress {
						btc: Vec::new(),
						eth: Vec::new(),
						eos: Vec::new(),
					},
			};
			<Metadata<T>>::insert(&did_hash, metadata);

			// update identity record
			<Identity<T>>::insert(&address, &did_hash);

			// update identity to address map
			<IdentityOf<T>>::insert(&did_hash, &address);

			// update did count
			let all_did_count = Self::all_did_count();
			let new_count = all_did_count.checked_add(1)
					.ok_or("Overflow adding a new did")?;
			<AllDidCount>::put(new_count);

			let harsh = HarshBuilder::new().salt("prochain did").length(4).init().unwrap();
			let idx = harsh.encode(&[all_did_count]).unwrap();

			<AllDidsArray<T>>::insert(&idx, &did_hash);
			<AllDidsIndex<T>>::insert(&did_hash, idx);

			// broadcast event
			Self::deposit_event(RawEvent::Created(sender, did_hash));
		}

		fn update(origin, to: T::AccountId) {
			let sender = ensure_signed(origin)?;

			ensure!(<Identity<T>>::exists(sender.clone()), "this account has no did yet");

			// get current did
			let did = Self::identity(&sender);
			ensure!(<Metadata<T>>::exists(did), "did does not exsit");
			ensure!(!<Identity<T>>::exists(&to), "the public key has been taken");

			// 更新account映射
			<Identity<T>>::remove(sender.clone());
			<Identity<T>>::insert(to.clone(), &did);

			// 更新did对应的accountid
			<IdentityOf<T>>::insert(did.clone(), to.clone());

			let mut metadata = Self::metadata(did);
			metadata.address = to.clone();

			<Metadata<T>>::insert(did, metadata);

			let money = <balances::Module<T>>::free_balance(sender.clone());
			<balances::Module<T> as Currency<_>>::transfer(&sender, &to, money, ExistenceRequirement::AllowDeath,)?;

			Self::deposit_event(RawEvent::Updated(to, did, money));
		}

		// transfer fund by did
		fn transfer(origin, to_did: T::Hash, value: T::Balance, memo: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			ensure!(<Identity<T>>::exists(sender.clone()), "you have no did yet");

			Self::_transfer(sender, to_did, value, memo)?;
		}

		// lock fund
		fn lock(origin, value: T::Balance, period: T::Moment) {
			let sender = ensure_signed(origin)?;

			let sender_balance = <balances::Module<T>>::free_balance(sender.clone());
			ensure!(sender_balance >= value, "you dont have enough free balance");
			ensure!(value >= Self::min_deposit(), "you must lock at least 50 pra per time");
			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");

			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);

			// make sure the superior exists
			ensure!(<Metadata<T>>::exists(metadata.superior), "superior does not exsit");
			
			let mut fee = Self::fee_to_previous();
			let mut locked_funds = value - fee;
			let mut max_quota = Self::balance_to_u64(locked_funds / Self::min_deposit()) * Self::base_quota();
			let mut rewards_ratio = 20;// basis rewards_ratio is 20%

			if metadata.locked_records.is_none() {
				let memo = "新群主抵押分成".as_bytes().to_vec();

				Self::_transfer(sender.clone(), metadata.superior, fee, memo)?;
			} else {
				fee = Self::u128_to_balance(0);
				
				let locked_records = metadata.locked_records.unwrap();

				let old_locked_funds = locked_records.locked_funds;
				locked_funds = old_locked_funds + value;

				max_quota = Self::balance_to_u64(locked_funds / Self::min_deposit()) * Self::base_quota();
				
				if max_quota >= metadata.subordinate_count {
					rewards_ratio = 20;
				};
			}

			metadata.locked_records = Some(LockedRecords {
				locked_funds,
				rewards_ratio,
				max_quota,
				locked_time: <timestamp::Module<T>>::get(),
				locked_period: period.clone(),
			});

			<balances::Module<T>>::reserve(&sender, value - fee)?;

			<Metadata<T>>::insert(did, metadata);

			Self::deposit_event(RawEvent::Locked(sender, locked_funds, period));
		}

		// unlock fund
		fn unlock(origin, value: T::Balance) {
			let sender = ensure_signed(origin)?;

			let reserved_balance = <balances::Module<T>>::reserved_balance(sender.clone());

			ensure!(reserved_balance >= value, "unreserved funds should equal or less than reserved funds");

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");

			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);
			ensure!(metadata.locked_records.is_some(), "you didn't lock funds before");
			
			let mut locked_records = metadata.locked_records.unwrap();
			let LockedRecords { locked_time, locked_period, locked_funds, .. } = locked_records;
			let now = <timestamp::Module<T>>::get();
			let unlock_time = locked_time.checked_add(&locked_period).ok_or("Overflow.")?;

			ensure!(now >= unlock_time, "unlock time has not reached");

			let unlock_records = UnlockRecords {
				unlock_time,
				unlock_funds: value,
			};

			let new_locked_funds = locked_funds - value;
			let new_max_quota = Self::balance_to_u64(new_locked_funds / Self::min_deposit()) as u64 * Self::base_quota();
			let rewards_ratio = if new_max_quota >= metadata.subordinate_count { 20 } else { 100 * (1 - new_max_quota / metadata.subordinate_count) as u64 };

			locked_records = LockedRecords {
				locked_funds: new_locked_funds,
				rewards_ratio,
				max_quota: new_max_quota,
				.. locked_records
			};

			metadata.unlock_records = Some(unlock_records);
			metadata.locked_records = Some(locked_records);

			<Metadata<T>>::insert(did, metadata);

			<balances::Module<T>>::unreserve(&sender, value);

			Self::deposit_event(RawEvent::Unlock(sender, value));
		}

		// add external address
		fn add_external_address(origin, add_type: Vec<u8>, address: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");

			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);
			let mut external_address = metadata.external_address;

						match &add_type[..] {
								b"btc" => {
										check::from(address.clone()).map_err(|_| "invlid bitcoin address")?;
										external_address.btc = address.clone();
										print("add btc address sucessfully");
								},
								b"eth" => {
										ensure!(check::is_valid_eth_address(address.clone()), "invlid eth account");
										external_address.eth = address.clone();
										print("add eth address sucessfully");
								},
								b"eos" => {
										ensure!(check::is_valid_eos_address(address.clone()), "invlid eos account");
										external_address.eos = address.clone();
										print("add eos address sucessfully");
								},
								_ => ensure!(false, "invlid type"),
						};

			metadata.external_address = external_address;

			<Metadata<T>>::insert(did, metadata);

			Self::deposit_event(RawEvent::AddressAdded(sender, add_type, address));
		}

		fn set_group_name(origin, name: Vec<u8>) {
			let sender = ensure_signed(origin)?;

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");

			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);

			ensure!(name.len() < 50, "group name is too long");
			ensure!(metadata.locked_records.is_some(), "you are not eligible to set group name");

			metadata.group_name = Some(name.clone());

			<Metadata<T>>::insert(did, metadata);

			Self::deposit_event(RawEvent::GroupNameSet(sender, name));
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

	fn generate_did(pubkey: &[u8], did_type: &[u8]) -> Vec<u8> {
		// 通过公钥生成hash值
		let mut hash = blake2_256(pubkey);

		// did的类型
		let did_ele = did_type;
		let mut did_ele = did_ele.to_vec();

		// 	截取第一步生成的hash的前20位，将did类型附加在最前面
		did_ele.append(&mut hash[..20].to_vec());

		// 将第二步生成的hash再次hash
		let mut ext_hash = blake2_256(&did_ele[..]);

		// 截取第三步生成的hash的前4位，并附加到第二步生成的hash后面
		did_ele.append(&mut ext_hash[..4].to_vec());

		did_ele
	}

	fn _transfer(from: T::AccountId, to_did: T::Hash, value: T::Balance, memo: Vec<u8>) -> Result {
		let sender_balance = <balances::Module<T>>::free_balance(from.clone());
		ensure!(sender_balance >= value, "you dont have enough free balance");

		let to = Self::identity_of(to_did).ok_or("corresponding AccountId does not exsit")?;
		ensure!(from != to, "you can not send money to yourself");

		let from_did = Self::identity(&from);
		
		// check overflow
		let receiver_balance = <balances::Module<T>>::free_balance(to.clone());
		sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
		receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

		// proceeds split
		let fee_type = b"ads";
		if Self::is_sub(&memo, fee_type) {
			let MetadataRecord { superior, .. } = Self::metadata(&to_did);
			let superior_address = Self::identity_of(superior).ok_or("superior AccountId does not find")?;
			
			let MetadataRecord { locked_records, ..} = Self::metadata(&superior);
			let rewards_ratio = if locked_records.is_some() { locked_records.unwrap().rewards_ratio } else { 0 };
			
			let fee_to_superior = value * Self::u128_to_balance(rewards_ratio.into()) / Self::u128_to_balance(100);
			let fee_to_user = value * Self::u128_to_balance((100 - rewards_ratio).into()) / Self::u128_to_balance(100);

			<balances::Module<T> as Currency<_>>::transfer(&from, &superior_address, fee_to_superior, ExistenceRequirement::AllowDeath)?;
			<balances::Module<T> as Currency<_>>::transfer(&from, &to, fee_to_user, ExistenceRequirement::AllowDeath)?;
		} else {
			<balances::Module<T> as Currency<_>>::transfer(&from, &to, value, ExistenceRequirement::AllowDeath)?;
		}

		Self::deposit_event(RawEvent::Transfered(from_did, to_did, value, memo));

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use support::{assert_ok, assert_noop, impl_outer_origin, impl_outer_event, parameter_types};
	use primitives::H256;
	// The testing primitives are very useful for avoiding having to work with signatures
	// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
	use sr_primitives::{
		Perbill, testing::Header, traits::{BlakeTwo256, IdentityLookup},
	};
	use system::{EventRecord, Phase};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	mod did {
		pub use super::super::*;
	}

	impl_outer_event! {
		pub enum Event for Test {
			did<T>, balances<T>,
		}
	}
	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: u32 = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::one();
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Call = ();
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}
	parameter_types! {
		pub const ExistentialDeposit: u64 = 0;
		pub const TransferFee: u64 = 0;
		pub const CreationFee: u64 = 0;
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = Event;
		type TransferPayment = ();
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type TransferFee = TransferFee;
		type CreationFee = CreationFee;
	}

	parameter_types! {
		pub const MinimumPeriod: u64 = 1;
	}

	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
		type MinimumPeriod = MinimumPeriod;
	}

	parameter_types! {
		pub const ReservationFee: u64 = 2;
		pub const MinLength: usize = 3;
		pub const MaxLength: usize = 16;
		pub const One: u64 = 1;
	}

	impl Trait for Test {
		type Event = Event;
	}

	const EOS_ADDRESS: &[u8; 12] = b"praqianchang";
	const BTC_ADDRESS: &[u8; 34] = b"1N75dvASxn1CCjaeguyqvwXLXJun9e54mM";
	const ETH_ADDRESS: &[u8; 40] = b"cb222a32df146ef7e3ac63725dad0fd978d33ce2";

	type DidModule = Module<Test>;
	type System = system::Module<Test>;
	type Balances = balances::Module<Test>;
	type Timestamp = timestamp::Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		// We use default for brevity, but you can configure as desired if needed.
		balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10000),
				(2, 10000),
				(3, 10000),
			],
			vesting: vec![],
		}.assimilate_storage(&mut t).unwrap();

		GenesisConfig::<Test> {
			genesis_account: 1u64,
			min_deposit: 50,
			base_quota: 250,
			fee_to_previous: 25,
		}.assimilate_storage(&mut t).unwrap();

		t.into()
	}

	#[test]
	fn should_pass_create() {
		new_test_ext().execute_with(|| {
			System::set_block_number(0);

			// genesis account
			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			// second account
			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
				2u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("s".as_bytes().to_vec()),
				Some("f".as_bytes().to_vec())
			));

		});
	}

	#[test]
	fn should_pass_update() {
		new_test_ext().execute_with(|| {
			System::set_block_number(0);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::update(Origin::signed(1), 2u64));
			assert_eq!(Balances::free_balance(&1), 0);
			assert_eq!(Balances::free_balance(&2), 20000);
		});
	}

	#[test]
	fn should_pass_lock() {
		new_test_ext().execute_with(|| {
			System::set_block_number(0);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
				2u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("s".as_bytes().to_vec()),
				Some("f".as_bytes().to_vec())
			));

			assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));

			assert_noop!(DidModule::lock(Origin::signed(2), 10, 5), "you must lock at least 50 pra per time");

			assert_eq!(Balances::free_balance(&2), 9900);
			assert_eq!(Balances::free_balance(&1), 10025); // get 25 from locked funds

		});
	}

	#[test]
	fn should_pass_unlock() {
		new_test_ext().execute_with(|| {

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
				2u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("s".as_bytes().to_vec()),
				Some("f".as_bytes().to_vec())
			));

			Timestamp::set_timestamp(42);
			assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));

			Timestamp::set_timestamp(50);
			assert_ok!(DidModule::unlock(Origin::signed(2), 10));

			assert_eq!(Balances::free_balance(&2), 9910);
		});
	}

	#[test]
	fn should_pass_transfer() {
		new_test_ext().execute_with(|| {
			System::set_block_number(1);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
				2u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("s".as_bytes().to_vec()),
				Some("f".as_bytes().to_vec())
			));

			let memo =b"transfer test";
			assert_ok!(DidModule::transfer(
				Origin::signed(2), 
				DidModule::identity(&1), 
				100, 
				memo.to_vec()
			));

			let events = System::events();
			let from_did = DidModule::identity(&2);
			assert_eq!(
				events[events.len() - 1],
				EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: Event::did(RawEvent::Transfered(from_did, DidModule::identity(&1), 100, memo.to_vec())),
						topics: vec![],
				}
			);

			assert_eq!(Balances::free_balance(&2), 9900);
			assert_eq!(Balances::free_balance(&1), 10100);

			assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));
			assert_eq!(Balances::free_balance(&1), 10125);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x5e9c79234b5e55348fc60f38b28c2cc60d8bb4bd2862eae2179a05ec39e62658".to_vec(),
				3u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("n".as_bytes().to_vec()),
				Some("s".as_bytes().to_vec())
			));

			// test ads fee split
			assert_ok!(DidModule::transfer(
				Origin::signed(1), 
				DidModule::identity(&3), 
				1000, 
				b"ads fee".to_vec()
			));
			assert_eq!(Balances::free_balance(&3), 10800);
			assert_eq!(Balances::free_balance(&2), 10000);
		});
	}

	#[test]
	fn should_pass_add_external_address() {
		new_test_ext().execute_with(|| {
			System::set_block_number(0);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::add_external_address(Origin::signed(1), b"eos".to_vec(), EOS_ADDRESS.to_vec()));
			assert_ok!(DidModule::add_external_address(Origin::signed(1), b"eth".to_vec(), ETH_ADDRESS.to_vec()));
			assert_ok!(DidModule::add_external_address(Origin::signed(1), b"btc".to_vec(), BTC_ADDRESS.to_vec()));
		});
	}

	#[test]
	fn should_pass_set_group_name() {
		new_test_ext().execute_with(|| {
			System::set_block_number(0);

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
				1u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("f".as_bytes().to_vec()),
				None
			));

			assert_ok!(DidModule::create(
				Origin::signed(1),
				b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
				2u64,
				"1".as_bytes().to_vec(),
				H256::zero(),
				Some("s".as_bytes().to_vec()),
				Some("f".as_bytes().to_vec())
			));

			assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));
			assert_ok!(DidModule::set_group_name(Origin::signed(2), b"btc group".to_vec()));

		});
	}
}
