use crate::check;
use codec::{Decode, Encode};
use rstd::vec::Vec;
use runtime_io::blake2_256;
use support::{
    decl_event, decl_module, decl_storage, ensure, StorageMap,
    StorageValue, traits::{Currency, ReservableCurrency}, dispatch::Result, print,
};
use sr_primitives::traits::{CheckedSub, CheckedAdd, Hash, SaturatedConversion};
use system::ensure_signed;

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
pub struct MetadataRecord<AccountId, Hash, Balance, Moment> {
	address: AccountId,
	superior: Hash,
	creator: AccountId,
	did_ele: Vec<u8>,
	max_rewards: Option<Balance>,
	locked_funds: Option<Balance>,
	locked_time: Option<Moment>,
	locked_period: Option<Moment>,
	fund_superior: bool,
	social_account: Option<Hash>,
    external_address: ExternalAddress
}

pub const MILLICENTS: u128 = 1_000_000_000_000;
pub const CENTS: u128 = 1_000 * MILLICENTS;
pub const DOLLARS: u128 = 100 * CENTS;

decl_storage! {
    trait Store for Module<T: Trait> as DidModule {
        Identity get(identity): map T::AccountId => T::Hash;
		IdentityOf get(identity_of): map T::Hash => Option<T::AccountId>;
		SocialAccount get(social_account): map T::Hash => T::Hash;
		Metadata get(metadata): map T::Hash => MetadataRecord<T::AccountId, T::Hash, T::Balance, T::Moment>;
		AllDidCount get(all_did_count): u64;
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
        Locked(AccountId, Balance, Moment, Balance),
        Unlock(AccountId, Balance),
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
			
			if let Some(mut value) = social_account {

				// let social_hash = (&value, &did_type)
				// 					.using_encoded(<T as system::Trait>::Hashing::hash);

				// bind social account
				value.append(&mut did_type.to_vec());

				let social_hash = T::Hashing::hash(&value);
				// one social account only can bind one did
				ensure!(!<SocialAccount<T>>::exists(&social_hash), "this social account has been bound"); 
				
				let superior_did;
				if let Some(mut value) = social_superior {
					value.append(&mut did_type.to_vec());

					let superior_hash = T::Hashing::hash(&value);
					ensure!(<SocialAccount<T>>::exists(&superior_hash), "the superior does not exsit"); 
					superior_did = Self::social_account(superior_hash);
				} else {
					superior_did = superior;
				};

				<SocialAccount<T>>::insert(social_hash, &did_hash);
				
				// update metadata
				let metadata = MetadataRecord {
						address: address.clone(),
						superior: superior_did,
						creator: sender.clone(),
						did_ele,
						max_rewards: None,
						locked_funds: None,
						locked_time: None,
						locked_period: None,
						fund_superior: false,
						social_account: Some(social_hash),
                        external_address: ExternalAddress {
                            btc: Vec::new(),
                            eth: Vec::new(),
                            eos: Vec::new(),
                        },
				};
				<Metadata<T>>::insert(&did_hash, metadata);

			} else {
				// update metadata
				let metadata = MetadataRecord {
						address: address.clone(),
						superior,
						creator: sender.clone(),
						did_ele,
						max_rewards: None,
						locked_funds: None,
						locked_time: None,
						locked_period: None,
						fund_superior: false,
						social_account: None,
                        external_address: ExternalAddress {
                            btc: Vec::new(),
                            eth: Vec::new(),
                            eos: Vec::new(),
                        },
				};
				<Metadata<T>>::insert(&did_hash, metadata);
			};

			// update identity record
			<Identity<T>>::insert(&address, &did_hash);

			// update identity to address map
			<IdentityOf<T>>::insert(&did_hash, &address);

			// update did count
			let all_did_count = Self::all_did_count();
			let new_count = all_did_count.checked_add(1)
					.ok_or("Overflow adding a new did")?;
			<AllDidCount>::put(new_count);

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
            <balances::Module<T> as Currency<_>>::transfer(&sender, &to, money)?;

            Self::deposit_event(RawEvent::Updated(to, did, money));
		}

        // transfer fund by did
        fn transfer(origin, to_did: T::Hash, value: T::Balance) {
			let sender = ensure_signed(origin)?;

			Self::_transfer(sender, to_did, value)?;
		}

        // lock fund
        fn lock(origin, value: T::Balance, period: T::Moment) {
			let sender = ensure_signed(origin)?;

			let sender_balance = <balances::Module<T>>::free_balance(sender.clone());
			ensure!(sender_balance >= value, "you dont have enough free balance");

			let min = Self::u64_to_balance(50 * MILLICENTS);
            let zero = Self::u64_to_balance(0);
			ensure!(value >= min, "you must lock at least 50 pra per time");

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");
			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);
			
			// make sure the superior exists
			ensure!(<Metadata<T>>::exists(metadata.superior), "superior does not exsit");

			let mut fee = Self::u64_to_balance(25 * MILLICENTS);
			if !metadata.fund_superior {
				Self::_transfer(sender.clone(), metadata.superior, fee)?;
				metadata.fund_superior = true
			} else {
				fee = Self::u64_to_balance(0);
			}

			let old_locked_fund = metadata.locked_funds.unwrap_or(zero);
			let locked_funds = old_locked_fund + value - fee;
			let max_rewards = locked_funds * Self::u64_to_balance(10);

			<balances::Module<T>>::reserve(&sender, value - fee)?;
			
			metadata.locked_time = Some(<timestamp::Module<T>>::get());
			metadata.locked_funds = Some(locked_funds);
			metadata.locked_period = Some(period.clone());
			metadata.max_rewards = Some(max_rewards);
			<Metadata<T>>::insert(did, metadata);

			Self::deposit_event(RawEvent::Locked(sender, locked_funds, period, max_rewards));
		}

        // unlock fund
        fn unlock(origin, value: T::Balance) {
			let sender = ensure_signed(origin)?;

			let reserved_balance = <balances::Module<T>>::reserved_balance(sender.clone());

			ensure!(reserved_balance == value, "unreserve funds should equal reserved funds");

			ensure!(<Identity<T>>::exists(&sender), "this account has no did yet");

			let did = Self::identity(&sender);
			let mut metadata = Self::metadata(&did);
			let now = <timestamp::Module<T>>::get();
			let unlock_time = metadata.locked_time.unwrap().checked_add(&metadata.locked_period.unwrap()).ok_or("Overflow.")?;
			ensure!(now >= unlock_time, "unlock time has not reached");
			
			metadata.locked_time = None;
			metadata.locked_funds = None;
			metadata.locked_period = None;
			metadata.max_rewards = None;

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
                    external_address.btc = address;
                    print("add btc address sucessfully");
                },
                b"eth" => {
                    ensure!(check::is_valid_eth_address(address.clone()), "invlid eth account");
                    external_address.eth = address;
                    print("add eth address sucessfully");
                },
                b"eos" => {
                    ensure!(check::is_valid_eos_address(address.clone()), "invlid eos account");
                    external_address.eos = address;
                    print("add eos address sucessfully");
                },
                _ => ensure!(false, "invlid type"),
            };

			metadata.external_address = external_address;

			<Metadata<T>>::insert(did, metadata);
		}
    }
}

impl<T: Trait> Module<T> {
    fn u64_to_balance(input: u128) -> T::Balance {
        input.saturated_into()
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
mod tests {
    use super::*;
    use tiny_keccak::keccak256;
    use runtime_io::with_externalities;
    use primitives::{H256, Blake2Hasher, crypto::Ss58Codec, crypto, ed25519, sr25519};
    use support::{impl_outer_origin, assert_ok, assert_noop, assert_err};
    use primitives::hexdisplay::HexDisplay;
    use hex_literal::hex;
    use balances;
    use keyring::Sr25519Keyring;
    use sr_primitives::{
        BuildStorage,
        traits::{BlakeTwo256, IdentityLookup},
        testing::{Digest, DigestItem, Header},
    };
    use std::fmt::Display;

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

    impl balances::Trait for Test {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
        type Event = ();
    }

    impl timestamp::Trait for Test {
        type Moment = u64;
        type OnTimestampSet = ();
    }

    impl Trait for Test {
        type Event = ();
    }

    type DidModule = Module<Test>;
    type Balance = balances::Module<Test>;
    type Timestamp = timestamp::Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut r = system::GenesisConfig::<Test>::default().build_storage().unwrap();
        r.0.extend(
            balances::GenesisConfig::<Test> {
                balances: vec![
                    (1, 100 * MILLICENTS),
                    (2, 100 * MILLICENTS),
                    (3, 100 * MILLICENTS),
                    (4, 100 * MILLICENTS),
                ],
                vesting: vec![],
                transaction_base_fee: 0,
                transaction_byte_fee: 0,
                existential_deposit: 0,
                transfer_fee: 0,
                creation_fee: 0,
            }.build_storage().unwrap().0,
        );

        r.0.into()
    }

    fn should_pass_create() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);

            let mut pubkey = hex!["e659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
        });
    }

    //	{"address":"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY","superior":"0x0000000000000000000000000000000000000000000000000000000000000000","creator":"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY","social_account":"0x6e657766656979616e67","type":"0x776563686174"}
    //0x1a0fa65f894e2eeb157baa619afec7a8e54423fe22b47484aaac29615d6d1f6a
    #[test]
    fn should_pass_identity() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
            let mut hash = DidModule::identity(42u64);
            let h = hex!["94b0a26bbe1a494310375e4b5e74ea125f786a6c1fcb02d9f99e72207bfad59a"];
            assert_eq!(h, hash.as_bytes());
        });
    }


    #[test]
    fn should_pass_identity_of() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
            let mut hash = DidModule::identity(42u64);
            let did = DidModule::identity_of(hash);
            assert_ne!(did, None);
            assert_eq!(did.unwrap(), 42u64);
        });
    }

    #[test]
    fn should_pass_all_did_count() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let r = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(r);
            let count = DidModule::all_did_count();
            assert_eq!(1, count);
        });
    }

    #[test]
    fn should_pass_metadata() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
            let mut hash = DidModule::identity(42u64);
            let data = DidModule::metadata(hash);
            let h = hex!["94b0a26bbe1a494310375e4b5e74ea125f786a6c1fcb02d9f99e72207bfad59a"];
            assert_eq!(data.superior.as_bytes(), h);
        });
    }

    #[test]
    fn should_pass_update() {
        with_externalities(&mut new_test_ext(), || {
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
            let result = DidModule::update(Origin::signed(42), 46u64);
            assert_ok!(result);
            let mut hash = DidModule::identity(42u64);
            let mut hash = DidModule::identity(46u64);
            let metadata = DidModule::metadata(hash);
            let mut pubkey = hex!["e659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e"];
            let result = DidModule::create(Origin::signed(46), pubkey.to_vec(), 46u64, "wx".as_bytes().to_vec(), H256::zero(), Some("haoming".as_bytes().to_vec()), Some(Vec::new()));
            assert_err!(result,"you already have did");
        });
    }

    #[test]
    fn should_pass_transfer_balance() {
        with_externalities(&mut new_test_ext(), || {
            let balance = Balance::free_balance(1u64);
            assert_eq!(balance, 100 * MILLICENTS);
            let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
            let result = DidModule::create(Origin::signed(42), pubkey.to_vec(), 42u64, "wx".as_bytes().to_vec(), H256::zero(), Some(Vec::new()), Some(Vec::new()));
            assert_ok!(result);
            let mut hash = DidModule::identity(42u64);
            let result = Balance::transfer(Origin::signed(1), 42u64, 10 * MILLICENTS);
            assert_ok!(result);
            let balance = Balance::free_balance(1u64);
            assert_eq!(balance, 90 * MILLICENTS);
            let balance = Balance::free_balance(42u64);
            assert_eq!(balance, 10 * MILLICENTS);
            let result = DidModule::transfer(Origin::signed(1), hash, 2 * MILLICENTS);
            assert_ok!(result);
            let balance = Balance::free_balance(42u64);
            assert_eq!(balance, 12 * MILLICENTS);
            let balance = Balance::free_balance(1u64);
            assert_eq!(balance, 88 * MILLICENTS);
        });
    }

    #[test]
    fn should_pass_lock() {
        with_externalities(&mut new_test_ext(), || {
            lock();
        });
    }
    fn lock(){
        let mut pubkey = hex!["e659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e"];
        let result = DidModule::create(Origin::signed(1), pubkey.to_vec(), 1u64, "wx".as_bytes().to_vec(), H256::zero(), Some("mm".as_bytes().to_vec()), None);
        let hash = DidModule::identity(1u64);
        let mut pubkey = hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
        let result = DidModule::create(Origin::signed(2), pubkey.to_vec(), 2u64, "wx".as_bytes().to_vec(), hash, Some("hm".as_bytes().to_vec()), Some("mm".as_bytes().to_vec()));
        let hash = DidModule::identity(2u64);
        let metadata = DidModule::metadata(hash);
        assert_ok!(result);
        let result = DidModule::lock(Origin::signed(2), 50 * MILLICENTS, 1);
        assert_ok!(result);
        let balance =  Balance::free_balance(2u64);
        let reserved_balance = Balance::reserved_balance(2u64);
        assert_eq!(reserved_balance,25*MILLICENTS);
        let balance = Balance::free_balance(1u64);
        assert_eq!(balance,125*MILLICENTS);
    }
    #[test]
    fn should_pass_unlock(){
        with_externalities(&mut new_test_ext(),||{
            Timestamp::set_timestamp(42);
            lock();
            let balance = Balance::free_balance(2u64);
            Timestamp::set_timestamp(47);
            let result = DidModule::unlock(Origin::signed(2),0*MILLICENTS);
            assert_ok!(result);
            let reserved_balance = Balance::reserved_balance(2u64);
            let balance = Balance::free_balance(2u64);
            assert_eq!(balance,50*MILLICENTS);
            let result = DidModule::unlock(Origin::signed(2),25*MILLICENTS);
            assert_ok!(result);
            let reserved_balance = Balance::reserved_balance(2u64);
            let balance = Balance::free_balance(2u64);
            assert_eq!(balance,75*MILLICENTS);
        });
    }

    fn alice_secret() -> secp256k1::SecretKey {
        secp256k1::SecretKey::parse(&keccak256(b"Alice")).unwrap()
    }

    fn alice_public() -> secp256k1::PublicKey {
        secp256k1::PublicKey::from_secret_key(&alice_secret())
    }
}