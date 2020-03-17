#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

use codec::{Decode, Encode};
use sp_std::vec::Vec;
use frame_support::{
	decl_event, decl_module, decl_storage, ensure,
};
use sp_runtime::traits::{Zero, CheckedSub, CheckedAdd};
use frame_system::{self as system, ensure_signed};

pub trait Trait: pallet_balances::Trait + pallet_timestamp::Trait + did::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct AdsMetadata<Balance, Moment> {
	advertiser: Vec<u8>,
  topic: Vec<u8>,
  total_amount: Balance,
  surplus: Balance,
  gas_fee_used: Balance,
  single_click_fee: Balance,
  create_time: Moment,
  period: Moment,
}

decl_storage! {
    trait Store for Module<T: Trait> as AdsModule {
        Contract get(fn contract) config(): T::AccountId;
        MinDeposit get(fn min_deposit) config(): T::Balance;

        AdsRecords get(fn ads_records): map hasher(blake2_256) T::Hash => AdsMetadata<T::Balance, T::Moment>;
        AllAdsCount get(fn all_ads_count): u64;
    }
}

decl_event! {
  pub enum Event<T>
  where
    <T as frame_system::Trait>::Hash,
    <T as pallet_balances::Trait>::Balance,
		<T as pallet_timestamp::Trait>::Moment,
    {
        Published(Hash, Hash, Balance),
        Deposited(Hash, Hash, Balance),
        Withdrawl(Hash, Balance),
        Distributed(Hash, Hash, Balance),
        AdsUpdated(Hash, Balance, Moment),
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;
    
        fn publish(origin, name: Vec<u8>, topic: Vec<u8>, total_amount: T::Balance, single_click_fee: T::Balance, period: T::Moment) {
            let sender = ensure_signed(origin)?;

            ensure!(total_amount >= Self::min_deposit(), "min deposit 500 pra");

            let (from_key, _) = <did::Module<T>>::identity(sender).ok_or("did does not exists")?;
            let create_time = <pallet_timestamp::Module<T>>::get();

            let (contract, _) = <did::Module<T>>::identity(Self::contract()).ok_or("cant find did")?;
            <did::Module<T>>::transfer_by_did(from_key, contract, total_amount, "开户广告费".as_bytes().to_vec())?;

            let ads_metadata = AdsMetadata {
                advertiser: name,
                topic,
                total_amount,
                surplus: total_amount,
                gas_fee_used: Zero::zero(),
                single_click_fee,
                create_time,
                period
            };

            <AdsRecords<T>>::insert(from_key, ads_metadata);

            // update count
            let all_ads_count = Self::all_ads_count();
            let new_count = all_ads_count.checked_add(1)
                    .ok_or("Overflow adding new ads")?;
            <AllAdsCount>::put(new_count);

            Self::deposit_event(RawEvent::Published(from_key, contract, total_amount));
        }

        fn deposit(origin, value: T::Balance, memo: Vec<u8>) {
            let sender = ensure_signed(origin)?;

            let (user_key, _) = <did::Module<T>>::identity(&sender).ok_or("from did does not exist")?;

            ensure!(<did::Identity<T>>::contains_key(sender), "did does not exists");
            ensure!(value >= Self::min_deposit(), "min deposit 100 pra");
            ensure!(<AdsRecords<T>>::contains_key(user_key), "you haven't published ads");

            let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or("contract did does not find")?;

            <did::Module<T>>::transfer_by_did(user_key, contract_key, value, memo)?;

            // update ads records
            let mut ads_metadata = Self::ads_records(user_key);
            ads_metadata.total_amount = ads_metadata.total_amount.checked_add(&value).ok_or("overflow")?;
            ads_metadata.surplus = ads_metadata.surplus.checked_add(&value).ok_or("overflow")?;

            <AdsRecords<T>>::insert(user_key, ads_metadata);

            Self::deposit_event(RawEvent::Deposited(user_key, contract_key, value));
        }

        fn withdraw(origin, value: T::Balance, memo: Vec<u8>) {
            let sender = ensure_signed(origin)?;

            let (from_key, _) = <did::Module<T>>::identity(&sender).ok_or("from did cant find")?;

            ensure!(<did::Identity<T>>::contains_key(sender), "did does not exists");
            ensure!(<AdsRecords<T>>::contains_key(from_key), "you haven't published ads");

            let mut ads_metadata = Self::ads_records(from_key);

            ensure!(ads_metadata.surplus >= value, "withdrawl money is larger than your surplus");

            let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or("contract did not found")?;

            <did::Module<T>>::transfer_by_did(contract_key, from_key, value, memo)?;

            // update ads metadata
            ads_metadata.total_amount = ads_metadata.total_amount.checked_sub(&value).ok_or("overflow")?;
            ads_metadata.surplus = ads_metadata.surplus.checked_sub(&value).ok_or("overflow")?;

            <AdsRecords<T>>::insert(from_key, ads_metadata);

            Self::deposit_event(RawEvent::Withdrawl(from_key, value));
        }

		fn distribute(origin, publisher: T::Hash, user: T::Hash, value: T::Balance) {
			let sender = ensure_signed(origin)?;

			ensure!(sender == Self::contract(), "you have no access to use the funds");

			let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or("contract did not found")?;

			ensure!(<AdsRecords<T>>::contains_key(publisher), "the account hadn't published ads yet");
            ensure!(<did::Metadata<T>>::contains_key(user), "the user does not have did yet");
			
			let mut ads_metadata = Self::ads_records(publisher);

			ensure!(ads_metadata.surplus >= value, "your surplus is not enough");

			<did::Module<T>>::transfer_by_did(contract_key, user, value, "看广告收益".as_bytes().to_vec())?;

			// update ads metadata
			ads_metadata.surplus = ads_metadata.surplus.checked_sub(&value).ok_or("overflow")?;

			<AdsRecords<T>>::insert(publisher, ads_metadata);

			Self::deposit_event(RawEvent::Distributed(publisher, user, value));
		}

		fn update_ads(origin, single_click_fee: T::Balance, period: T::Moment) {
			let sender = ensure_signed(origin)?;

			let (from_key, _) = <did::Module<T>>::identity(sender).ok_or("from did cant find")?;

			ensure!(<AdsRecords<T>>::contains_key(from_key), "you haven't published ads");

			// update ads records
			let mut ads_metadata = Self::ads_records(from_key);
			ads_metadata.single_click_fee = single_click_fee;
			ads_metadata.period = period;

			<AdsRecords<T>>::insert(from_key, ads_metadata);

			Self::deposit_event(RawEvent::AdsUpdated(from_key, single_click_fee, period));
		}
	}
}
