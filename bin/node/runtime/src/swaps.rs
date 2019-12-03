use codec::{Decode, Encode};
use sp_runtime::traits::{Zero, Bounded, Member, SimpleArithmetic};
use support::{
    decl_event, decl_module, decl_storage, ensure, Parameter, StorageMap, StorageValue,
    traits::{Currency, WithdrawReason, WithdrawReasons, ExistenceRequirement},
};
use rstd::result;
use rstd::vec::Vec;
use system::{ensure_signed, ensure_root};

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct HTLC<BlockNumber, Balance, Hash, Moment, AccountId>
where
    BlockNumber: PartialEq + Eq + Decode + Encode,
{
    block_number: BlockNumber,
    out_amount: Balance,
    expire_height: BlockNumber,
    random_number_hash: Hash,
    swap_id: Hash,
    timestamp: Moment,
    sender_addr: Vec<u8>,
    sender_chain_type: u64,
    receiver_addr: AccountId,
    receiver_chain_type: u64,
    recipient_addr: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum States {
    INVALID,
    OPEN,
    COMPLETED,
    EXPIRED,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub enum ChainTypes {
    /// Ethereum
    ETH,
    /// Prochain
    PRA,
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

pub trait Trait: balances::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    //type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;
    type Currency: Currency<Self::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as SWAPS {
        /// Stores the locked pra tokens
        pub PraTokenAddr get(pra_token_addr): Option<T::AccountId>;

        /// Key: swap_id, Value: HTLC
        pub SwapData get(swap_data): map T::Hash => Option<HTLC<T::BlockNumber, T::Balance, T::Hash, T::Moment, T::AccountId>>;

        /// Key: swap_id, Value: States
        pub SwapStates get(swap_states): map T::Hash => Option<States>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
		<T as balances::Trait>::Balance,
		<T as system::Trait>::Hash,
		<T as timestamp::Trait>::Moment,
		<T as system::Trait>::BlockNumber
    {
        /// Set PraTokenAddr
        INIT(AccountId),

        /// Init HTLC, params: Sender, Receiver, SwapID, RandomNumberHash, Timestamp, ExpireHeight, OutAmount
        HTLC(AccountId, AccountId, Hash, Hash, Moment, BlockNumber, Balance),

		/// Claim, params: Sender
        Claimed(AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Set PraTokenAddr
        pub fn init(origin, pra_token_addr: T::AccountId) {
			// TODO: add AuthorisedKey and use ensure_signed()
            ensure_root(origin)?;

            Self::deposit_event(RawEvent::INIT(pra_token_addr));
        }

        /// Add HTLC and lock asset
        pub fn htlc(origin,
                msg_sender: Vec<u8>,
                receiver_addr: T::AccountId,
                swap_id: T::Hash,
                random_number_hash: T::Hash,
                timestamp: T::Moment,
                expire_height: T::BlockNumber,
                out_amount: T::Balance,
                pra_amount: T::Balance) {
            let sender = ensure_signed(origin)?;
            ensure!(sender != receiver_addr, "Needs different account");
            ensure!(!out_amount.is_zero(), "Transfer amount should be non-zero");
            ensure!(out_amount == pra_amount, "Needs same amount");

            //let _ = <balances::Module<T> as Currency<_>>::withdraw(&sender, out_amount, WithdrawReason::TransactionPayment, ExistenceRequirement::KeepAlive)?;

            Self::deposit_event(RawEvent::HTLC(sender, receiver_addr, swap_id, random_number_hash, timestamp, expire_height, out_amount));
		}

		/// claims the previously locked asset
		pub fn claim(origin, swap_id: T::Hash) {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_swap_exist(swap_id), "Invalid swap id");
			ensure!(Self::is_claimable(swap_id), "Swap is not claimable");

			let htlc = Self::swap_data(swap_id).unwrap();

			//T::Currency::ensure_can_withdraw(&sender);

			// transfer locked asset to receiver
			//Self::transfer_to(&sender, &(htlc.receiver_addr.clone()), htlc.out_amount);

			Self::deposit_event(RawEvent::Claimed(sender));
		}
    }
}

impl<T: Trait> Module<T> {
	/// HTLC exists
    fn is_swap_exist(swap_id: T::Hash) -> bool {
        let state = Self::swap_states(swap_id);

        state.is_some() && state.unwrap() != States::INVALID
    }

	/// is HTLC claimable
    fn is_claimable(swap_id: T::Hash) -> bool {
        let state = Self::swap_states(swap_id);

        if state.is_some() && state.unwrap() == States::OPEN {
            let swap = Self::swap_data(swap_id);
            if swap.is_some() && <system::Module<T>>::block_number() < swap.unwrap().expire_height {
                return true;
            }
        }
        return false;
	}

	fn transfer_to(sender: T::AccountId, receiver: T::AccountId, amount: BalanceOf<T>) {
		//TODO: check if could keep sender alive

		T::Currency::transfer(&sender, &receiver, amount, ExistenceRequirement::KeepAlive);
	}

	/// query swap by swap_id
	fn query_open_swap(swap_id: T::Hash) -> result::Result<HTLC<T::BlockNumber, T::Balance, T::Hash, T::Moment, T::AccountId>, &'static str> {
		//let _ = ensure_signed(origin)?;

		let state = Self::swap_states(swap_id);
		let swap = Self::swap_data(swap_id);

		ensure!(state.is_some(), "Invalid swap_states");
		ensure!(swap.is_some(), "Invalid swap_data");

		Ok(swap.unwrap())
	}
}
