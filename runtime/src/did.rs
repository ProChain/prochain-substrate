use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use support::traits::{Currency, ReservableCurrency};
use system::ensure_signed;
use parity_codec::{Encode, Decode};
use rstd::prelude::*;
use runtime_io::{blake2_256};
use runtime_primitives::traits::{CheckedSub, CheckedAdd, As, Hash};

pub trait Trait: balances::Trait {
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct MetadataRecord<AccountId, Hash> {
	address: AccountId,
	superior: Hash,
	creator: AccountId,
}

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

		fn create(origin, pubkey: Vec<u8>, address: T::AccountId, superior: T::Hash) -> Result {
			let sender = ensure_signed(origin)?;

			
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
					address: address.clone(),
					superior,
					creator: sender.clone(),
			};
			
			let mut buf = Vec::new();
			buf.extend_from_slice(&did_ele.encode());
			let did_hash = T::Hashing::hash(&buf[..]);
			
			// make sure the did is new
			ensure!(!<Metadata<T>>::exists(&did_hash), "did alread existed");

			<Metadata<T>>::insert(&did_hash, metadata);

			<Identity<T>>::insert(&address, &did_hash);

			<IdentityOf<T>>::insert(did_hash, &address);

			let all_did_count = Self::all_did_count();

			let new_count = all_did_count.checked_add(1)
					.ok_or("Overflow adding a new did")?;

			<AllDidCount<T>>::put(new_count);

			Self::deposit_event(RawEvent::Created(sender, did_hash));

			Ok(())
		}

		fn update(origin, to: T::AccountId) -> Result {
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
			
			Self::update_to(sender, to, did)?;

      Ok(())
		}

		fn transfer(origin, to_did: T::Hash, value: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;

			Self::_transfer(sender, to_did, value)?;

			Ok(())
		}

		fn lock(origin, value: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;

			let sender_balance = <balances::Module<T>>::free_balance(sender.clone());
			ensure!(sender_balance >= value, "you dont have enough free balance");

			let fee = <T::Balance as As<u64>>::sa(25);
			let min = <T::Balance as As<u64>>::sa(50);
			ensure!(value >= min, "you must lock at least 50 pra");

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");
			let did = Self::identity(&sender);
			let MetadataRecord { superior, .. } = Self::metadata(&did);
			
			// make sure the superior exists
			ensure!(<Metadata<T>>::exists(superior), "superior does not exsit");

			Self::_transfer(sender.clone(), superior, fee)?;

			<balances::Module<T>>::reserve(&sender, value - fee)?;
			Ok(())
		}

		fn unlock(origin, value: T::Balance) -> Result {
			let sender = ensure_signed(origin)?;

			let reserved_balance = <balances::Module<T>>::reserved_balance(sender.clone());

			ensure!(reserved_balance >= value, "unreserve funds should less than reserved funds");

			<balances::Module<T>>::unreserve(&sender, value);

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

	fn _transfer(from: T::AccountId, to_did: T::Hash, value: T::Balance) -> Result {
		let sender_balance = <balances::Module<T>>::free_balance(from.clone());
		ensure!(sender_balance >= value, "you dont have enough free balance");

		let to = Self::identity_of(to_did).ok_or("corresponding AccountId does not exsit")?;
		ensure!(from != to, "you can not send money to yourself");

		// check overflow
		let _updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
		let receiver_balance = <balances::Module<T>>::free_balance(to.clone());
		let _updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

		<balances::Module<T> as Currency<_>>::transfer(&from, &to, value)?;

		Ok(())
	}

}

#[cfg(test)]
mod tests;