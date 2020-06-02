#![cfg_attr(not(feature = "std"), no_std)]
mod tests;
mod array_list;
use codec::{Decode, Encode};
use sp_std::vec::Vec;
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error,ensure,debug
};
use sp_runtime::{DispatchResult, traits::{Zero, CheckedSub, CheckedAdd, Hash}};
use frame_system::{self as system, ensure_signed};
use array_list::ArrayList;

pub trait Trait: pallet_balances::Trait + pallet_timestamp::Trait + did::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub type AdIndex = u64;
pub type ActiveIndex = u64;
type AdsActiveList = ArrayList<AdsActives, AdIndex, AdsActiveCount>;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct AdsMetadata<Balance, Moment> {
    advertiser: Vec<u8>,
    topic: Vec<u8>,
    total_amount: Balance,
    spend_amount: Balance,
    single_click_fee: Balance,
    display_page: Vec<u8>,
    landing_page: Option<Vec<u8>>,
    create_time: Moment,
    active: Option<ActiveIndex>,
    distribute_type: DistributeType,
}

#[derive(Encode, Decode , PartialEq, Eq, Clone, Debug)]
pub enum DistributeType{
    ADVERTISER,
    AGENT,
}

impl Default for DistributeType{
    fn default()-> Self{DistributeType::AGENT}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// ad does not exist
		ADNotExists,
		/// number overflow
		Overflow,
		/// dont have enough free balance
		NotEnoughBalance,
		/// group name is too long
		InvalidGroupName,
		/// you are not own the ad
		NotOwner,
        ///agent type ad need contract account signed
		NeedAgentAccountSigned,
        /// create or deposit ad min balance
		MineDeposit,
        /// ad contract did account
		ContractDidNotExists,
        /// ad status active
		Active,
        /// ad status not active
		NotActive,
		///the did type is not ad
		NotADAccount,
        ///withdraw balance time not reach
        TimeNotReach,
	}
}


decl_storage! {
    trait Store for Module<T: Trait> as AdsModule {
        pub Contract get(fn contract) config(): T::AccountId;
        pub MinDeposit get(fn min_deposit) config(): T::Balance;
        pub AdsRecords get(fn ads_records): map hasher(twox_64_concat) AdIndex => AdsMetadata<T::Balance, T::Moment>;
        pub AdsActives get(fn ads_actives): map hasher(twox_64_concat) ActiveIndex => Option<AdIndex>;
        pub AdsActiveCount get(fn ads_active_count): Option<ActiveIndex>;
        pub AdsOwner get(fn ads_owner):map hasher(twox_64_concat) AdIndex => T::Hash;
        pub AllAdsCount get(fn all_ads_count): AdIndex;
        pub OwnedAds get(fn owned_ads):map hasher(twox_64_concat) T::Hash => Vec<AdIndex>;
    }
}

decl_event! {
  pub enum Event<T>
  where
    <T as frame_system::Trait>::Hash,
    <T as pallet_balances::Trait>::Balance,
    {
        Published(Hash, Hash, Balance),
        Deposited(Hash, AdIndex ,  Balance),
        Active(AdIndex),
        Pause(AdIndex),
        Withdraw(Hash, Balance),
        Distributed(Hash, Hash, Balance),
        AdsUpdated(AdIndex),
    }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

        #[weight = 0]
        fn publish(origin, name: Vec<u8>, topic: Vec<u8>, total_amount: T::Balance, single_click_fee: T::Balance,display_page:Vec<u8>,landing_page:Option<Vec<u8>>,distribute_type:DistributeType) {
            let sender = ensure_signed(origin)?;

            ensure!(total_amount >= Self::min_deposit(), Error::<T>::MineDeposit);
            let (from_key, did) = <did::Module<T>>::identity(sender).ok_or(<did::Error<T>>::DidNotExists)?;
            ensure!(Self::is_sub(&did[..2] , "4".as_bytes()),Error::<T>::NotADAccount);
            let create_time = <pallet_timestamp::Module<T>>::get();

            let (contract, _) = <did::Module<T>>::identity(Self::contract()).ok_or(Error::<T>::ContractDidNotExists)?;
            <did::Module<T>>::transfer_by_did(from_key, contract, total_amount, "开户广告费".as_bytes().to_vec())?;

            let ads_metadata = AdsMetadata {
                advertiser: name,
                topic,
                total_amount,
                spend_amount: Zero::zero(),
                single_click_fee,
                display_page,
                landing_page,
                create_time,
                active: None,
                distribute_type,
            };
            let adid = Self::all_ads_count();
            Self::create_ad(from_key,&adid,ads_metadata)?;
            Self::active_ad(&adid)?;
            Self::deposit_event(RawEvent::Published(from_key, contract, total_amount));
            Self::deposit_event(RawEvent::Active(adid));
        }
        #[weight = 0 ]
        fn active(origin,adid:AdIndex){
            let sender = ensure_signed(origin)?;
            Self::check_ad_owner(&sender,&adid)?;
            Self::active_ad(&adid)?;
            Self::deposit_event(RawEvent::Active(adid));
        }
        #[weight = 0]
        fn pause(origin,adid:AdIndex){
            let sender = ensure_signed(origin)?;
            Self::check_ad_owner(&sender,&adid)?;
            Self::pause_ad(&adid)?;
            Self::deposit_event(RawEvent::Pause(adid));
        }

        #[weight = 0]
        fn deposit(origin, adid: AdIndex,value: T::Balance, memo: Vec<u8>) {
            let sender = ensure_signed(origin)?;
            let (user_key, _) = <did::Module<T>>::identity(&sender).ok_or(<did::Error<T>>::DidNotExists)?;
            ensure!(<did::Identity<T>>::contains_key(sender), <did::Error<T>>::DidNotExists);
            ensure!(value >= Self::min_deposit(), Error::<T>::MineDeposit);
            ensure!(<AdsRecords<T>>::contains_key(adid), Error::<T>::ADNotExists);
            let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or(Error::<T>::ContractDidNotExists)?;
            // update ads records
            <did::Module<T>>::transfer_by_did(user_key, contract_key, value, memo)?;
            let mut ads_metadata = Self::ads_records(adid);
            ads_metadata.total_amount = ads_metadata.total_amount.checked_add(&value).ok_or(Error::<T>::Overflow)?;
            <AdsRecords<T>>::insert(adid, ads_metadata);
            Self::deposit_event(RawEvent::Deposited(user_key , adid, value));
        }
//
        #[weight = 0]
        fn withdraw(origin, adid:AdIndex, value: T::Balance, memo: Vec<u8>) {
            let sender = ensure_signed(origin)?;
            Self::check_ad_owner(&sender,&adid)?;
            let (from_key, _) = <did::Module<T>>::identity(sender).ok_or(<did::Error<T>>::DidNotExists)?;
            let mut ads_metadata = Self::ads_records(adid);
            let now = <pallet_timestamp::Module<T>>::get();
            let locked_period:T::Moment = (30*24*60*60*1000u32).into();
            let lock_time = ads_metadata.create_time.checked_add(&locked_period).ok_or(Error::<T>::Overflow)?;
            ensure!(now>=lock_time,Error::<T>::TimeNotReach);
            let total_amount = ads_metadata.total_amount.checked_sub(&value).ok_or(Error::<T>::Overflow)?;
            ensure!(ads_metadata.spend_amount <= total_amount , Error::<T>::NotEnoughBalance);
            let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or(Error::<T>::ContractDidNotExists)?;
            <did::Module<T>>::transfer_by_did(contract_key, from_key, value, memo)?;
            // update ads metadata
            ads_metadata.total_amount = total_amount;
             <AdsRecords<T>>::insert(adid, ads_metadata);
            Self::deposit_event(RawEvent::Withdraw(from_key, value));
        }
//
        #[weight = 0]
		fn distribute(origin,adid: AdIndex,user: T::Hash) {
			let sender = ensure_signed(origin)?;
			let mut ads_metadata = <AdsRecords<T>>::get(adid);
			ensure!(ads_metadata.active.is_some(),Error::<T>::NotActive);
			match ads_metadata.distribute_type{
			    DistributeType::ADVERTISER=>{
                    Self::check_ad_owner(&sender,&adid)?;
			    },
			    DistributeType::AGENT=>{
                    ensure!(sender == Self::contract(),Error::<T>::NeedAgentAccountSigned);
			    }
			}
            let value = ads_metadata.single_click_fee;
            let spend = ads_metadata.spend_amount.checked_add(&value).ok_or(Error::<T>::Overflow)?;
			ensure!(spend <= ads_metadata.total_amount, Error::<T>::NotEnoughBalance);
            let (contract_key, _) = <did::Module<T>>::identity(Self::contract()).ok_or(Error::<T>::ContractDidNotExists)?;
            ensure!(<did::Metadata<T>>::contains_key(user),<did::Error<T>>::DidNotExists);
            let (from_key, _) = <did::Module<T>>::identity(sender).ok_or(<did::Error<T>>::DidNotExists)?;
			<did::Module<T>>::transfer_by_did(contract_key, user, value, "看广告收益".as_bytes().to_vec())?;
			// update ads metadata
			ads_metadata.spend_amount = spend;
			<AdsRecords<T>>::insert(adid, ads_metadata);
			Self::deposit_event(RawEvent::Distributed(from_key, user, value));
		}
//
       #[weight = 0]
		fn update_ads(origin, adid:AdIndex,name:Option<Vec<u8>>,single_click_fee: Option<T::Balance>,display_page:Option<Vec<u8>>,landing_page:Option<Vec<u8>>) {
			let sender = ensure_signed(origin)?;
            Self::check_ad_owner(&sender,&adid)?;
			// update ads records
            let mut ads_metadata = Self::ads_records(adid);
            if name.is_some(){
                ads_metadata.advertiser = name.unwrap();
            }
            if single_click_fee.is_some(){
                ads_metadata.single_click_fee = single_click_fee.unwrap();
            }
            if display_page.is_some(){
                ads_metadata.display_page = display_page.unwrap();
            }
            if landing_page.is_some(){
                ads_metadata.landing_page = landing_page;
            }
			<AdsRecords<T>>::insert(adid, ads_metadata);
			Self::deposit_event(RawEvent::AdsUpdated(adid));
		}
	}
}
impl<T: Trait> Module<T> {

    fn check_ad_owner(sender: &T::AccountId,adid:&AdIndex) ->DispatchResult{
        let (from_key, _) = <did::Module<T>>::identity(sender).ok_or(<did::Error<T>>::DidNotExists)?;
        ensure!(<AdsRecords<T>>::contains_key(adid), Error::<T>::ADNotExists);
        ensure!(<AdsOwner<T>>::get(adid) == from_key,Error::<T>::NotOwner);
        Ok(())
    }

    fn create_ad(user_key: T::Hash, adid: &AdIndex, ad: AdsMetadata<T::Balance, T::Moment>) -> DispatchResult {
        <AdsRecords<T>>::insert(adid, ad);
        <AdsOwner<T>>::insert(adid, &user_key);
//        <OwnedAdList<T>>::append(&user_key, adid.clone());
        let mut ads = Self::owned_ads(user_key);
        ads.push(adid.clone());
        let new_count = Self::all_ads_count().checked_add(1)
            .ok_or(Error::<T>::Overflow)?;
        AllAdsCount::put(new_count);
        <OwnedAds<T>>::insert(user_key,ads);
        Ok(())
    }

    fn active_ad(adid: &AdIndex) -> DispatchResult {
        let mut ads_metadata = Self::ads_records(adid);
        debug::info!("{:?}",ads_metadata.advertiser);
        ensure!(ads_metadata.active.is_none(),Error::<T>::Active);
        AdsActiveList::add(adid);
        ads_metadata.active = Some(AdsActiveList::size().checked_sub(1).ok_or(Error::<T>::Overflow)?);
        <AdsRecords<T>>::insert(adid, ads_metadata);
        Ok(())
    }

    fn pause_ad(adid: &AdIndex) -> DispatchResult {
        ensure!(<AdsRecords<T>>::contains_key(adid),<did::Error<T>>::DidNotExists);
        let mut ads_metadata = Self::ads_records(adid);
        debug::info!("{:?}",ads_metadata.advertiser);
        ensure!(ads_metadata.active.is_some(),Error::<T>::NotActive);
        let index = ads_metadata.active.unwrap();
        AdsActiveList::remove(&index);
        ads_metadata.active = None;
        <AdsRecords<T>>::insert(adid,ads_metadata);
        Ok(())
    }

    fn is_sub(mut haystack: &[u8], needle: &[u8]) -> bool {
        if needle.len() == 0 { return true; }
        while !haystack.is_empty() {
            if haystack.starts_with(needle) { return true; }
            haystack = &haystack[1..];
        }
        false
    }
}
