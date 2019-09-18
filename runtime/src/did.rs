use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use support::traits::{Currency, ReservableCurrency};
use system::ensure_signed;
use parity_codec::{Encode, Decode};
use rstd::prelude::*;
use runtime_io::{blake2_256};
use runtime_primitives::traits::{CheckedSub, CheckedAdd, As, Hash};

/// The module's configuration trait.
pub trait Trait: balances::Trait {
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct MetadataRecord<AccountId, Hash> {
	account: Vec<u8>,
	superior: Hash,
	account_id: AccountId,
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as DidModule {
		// Just a dummy storage item. 
		// Here we are declaring a StorageValue, `Identity` as a Option<u32>
		// `get(identity)` is the default getter which returns either the stored `u32` or `None` if nothing stored
		Identity get(identity): map T::AccountId => T::Hash;
		IdentityOf get(identity_of): map T::Hash => Option<T::AccountId>;
		Metadata get(metadata): map T::Hash => MetadataRecord<T::AccountId, T::Hash>;
		AllDidCount get(all_did_count): u64;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		fn create(origin, pubkey: Vec<u8>, superior: T::Hash) -> Result {
			let _sender = ensure_signed(origin)?;

			// let _som = Ss58Codec::from_ss58check(&_sender).unwrap().0.into();
			
			// 通过公钥生成hash值
			let mut hash = blake2_256(&pubkey);
			runtime_io::print("hash");

			// did的类型
			let did_ele = b"wechat";
			let mut did_ele = did_ele.to_vec();

			// 	截取第一步生成的hash的前20位，将did类型附加在最前面
			did_ele.append(&mut hash[..20].to_vec());

			// 将第二步生成的hash再次hash
			let mut ext_hash = blake2_256(&did_ele[..]);

			// 截取第三步生成的hash的前4位，并附加到第二步生成的hash后面
			did_ele.append(&mut ext_hash[..4].to_vec());
			
			// Replace all metadata
			let metadata = MetadataRecord {
					account: did_ele.to_vec(),
					superior,
					account_id: _sender.clone(),
			};
			
			let mut buf = Vec::new();
			buf.extend_from_slice(&did_ele.encode());
			let did_hash = T::Hashing::hash(&buf[..]);

			<Metadata<T>>::insert(&did_hash, metadata);

			<Identity<T>>::insert(&_sender, &did_hash);

			<IdentityOf<T>>::insert(did_hash, &_sender);

			let all_did_count = Self::all_did_count();

			let new_count = all_did_count.checked_add(1)
					.ok_or("Overflow adding a new did")?;

			<AllDidCount<T>>::put(new_count);

			Self::deposit_event(RawEvent::Created(_sender, did_hash));

			Ok(())
		}

		fn update(origin, to: T::AccountId, did: T::Hash) -> Result {
			let _sender = ensure_signed(origin)?;

			ensure!(<Identity<T>>::exists(_sender.clone()), "this account has not did yet");
			ensure!(<Metadata<T>>::exists(did), "did does not exsit");

			// 更新account映射
			<Identity<T>>::remove(_sender.clone());
			<Identity<T>>::insert(to.clone(), &did);

			// 更新did对应的accountid
			<IdentityOf<T>>::insert(did.clone(), to.clone());

			let mut metadata = Self::metadata(did);
			metadata.account_id = to.clone();

			<Metadata<T>>::insert(did, metadata);
			
			Self::update_to(_sender, to, did)?;

      Ok(())
		}

		fn transfer(origin, to_did: T::Hash, value: T::Balance) -> Result {
			let _sender = ensure_signed(origin)?;

			let sender_balance = <balances::Module<T>>::free_balance(_sender.clone());
			ensure!(sender_balance >= value, "you dont have enough free balance");

			let to = Self::identity_of(to_did).ok_or("corresponding AccountId does not exsit")?;
			ensure!(_sender != to, "you can not send money to yourself");

			// check overflow
			let _updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
			let receiver_balance = <balances::Module<T>>::free_balance(to.clone());
			let _updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

			<balances::Module<T> as Currency<_>>::transfer(&_sender, &to, value)?;

			Ok(())
		}
	}
}

decl_event! {
  pub enum Event<T>
  where 
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as balances::Trait>::Balance,
    {
      Created(AccountId, Hash),
			Updated(AccountId, Hash, Balance),
    }
}

impl<T: Trait> Module<T> {
	fn update_to(from: T::AccountId, to: T::AccountId, did: T::Hash) -> Result {
		// transfer funds
		let money = <T::Balance as As<u64>>::sa(1020);
		<balances::Module<T> as Currency<_>>::transfer(&from, &to, money)?;

		Self::deposit_event(RawEvent::Updated(to, did, money));

    Ok(())
  }

	fn _lock(from_did: T::Hash, value: T::Balance) -> Result {
		let from = Self::identity_of(from_did).ok_or("corresponding AccountId does not exsit")?;
		<balances::Module<T>>::reserve(&from, value)?;
		Ok(())
	}

	fn _unlock(to_did: T::Hash, value: T::Balance) -> Result {
		let to = Self::identity_of(to_did).ok_or("corresponding AccountId does not exsit")?;
		<balances::Module<T>>::unreserve(&to, value);
		Ok(())
	}

}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl Trait for Test {
		type Event = ();
	}
	type DidModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_identity`
			// calling the `do_identity` function with a value 42
			assert_ok!(DidModule::do_identity(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(DidModule::identity(), Some(42));
		});
	}
}
