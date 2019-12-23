#![cfg(test)]

use super::*;

use support::{assert_ok, assert_noop, impl_outer_origin, impl_outer_event, parameter_types};
use primitives::H256;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use sp_runtime::{
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
      (4, 10000),
      (5, 10000),
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

fn prepare_dids_for_test() {
  // genesis account
  assert_ok!(DidModule::create(
    Origin::signed(1),
    b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
    1u64,
    "1".as_bytes().to_vec(),
    H256::zero(),
    Some("first".as_bytes().to_vec()),
    None
  ));

  // second account
  assert_ok!(DidModule::create(
    Origin::signed(1),
    b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
    2u64,
    "1".as_bytes().to_vec(),
    H256::zero(),
    Some("second".as_bytes().to_vec()),
    Some("first".as_bytes().to_vec())
  ));

  // lock funds
  assert_ok!(DidModule::lock(Origin::signed(2), 1000, 5));

  // third account
  assert_ok!(DidModule::create(
    Origin::signed(1),
    b"0x5e9c79234b5e55348fc60f38b28c2cc60d8bb4bd2862eae2179a05ec39e62658".to_vec(),
    3u64,
    "1".as_bytes().to_vec(),
    H256::zero(),
    Some("third".as_bytes().to_vec()),
    Some("second".as_bytes().to_vec())
  ));
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
    Some("first".as_bytes().to_vec()),
    None
  ));

  });
}

#[test]
fn same_pubkey_should_not_pass_create() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    assert_ok!(DidModule::create(
      Origin::signed(1),
      b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
      1u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("first".as_bytes().to_vec()),
      None
    ));

    assert_noop!(DidModule::create(
      Origin::signed(1),
      b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
      2u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("second".as_bytes().to_vec()),
      Some("first".as_bytes().to_vec())
    ), "did alread existed");

  });
}

#[test]
fn same_social_account_should_not_pass_create() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    assert_ok!(DidModule::create(
      Origin::signed(1),
      b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
      1u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("first".as_bytes().to_vec()),
      None
    ));

    assert_noop!(DidModule::create(
      Origin::signed(1),
      b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
      2u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("first".as_bytes().to_vec()),
      None
    ), "this social account has been bound");

  });
}

#[test]
fn superior_not_exists_should_not_pass_create() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    assert_ok!(DidModule::create(
      Origin::signed(1),
      b"0x22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f".to_vec(),
      1u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("first".as_bytes().to_vec()),
      None
    ));

    assert_noop!(DidModule::create(
      Origin::signed(1),
      b"0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d".to_vec(),
      2u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("second".as_bytes().to_vec()),
      Some("firsts".as_bytes().to_vec())
    ), "the superior does not exsit");

  });
}

#[test]
fn should_pass_update() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_ok!(DidModule::update(Origin::signed(3), 4u64));
    assert_eq!(Balances::free_balance(&3), 0);
    assert_eq!(Balances::free_balance(&4), 20000);
  });
}

#[test]
fn without_did_should_not_pass_update() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_noop!(DidModule::update(Origin::signed(4), 5u64), "did does not exist");
  });
}

#[test]
fn new_pubkey_already_has_did_should_not_pass_update() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_noop!(DidModule::update(Origin::signed(2), 3u64), "the public key has been taken");
  });
}

#[test]
fn should_pass_lock() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_ok!(DidModule::lock(Origin::signed(2), 10, 5));

    assert_eq!(Balances::free_balance(&2), 8990);
    assert_eq!(Balances::free_balance(&1), 10025); // get 25 from locked funds


    assert_ok!(DidModule::lock(Origin::signed(3), 100, 5));

    assert_ok!(DidModule::create(
      Origin::signed(1),
      b"0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20".to_vec(),
      4u64,
      "1".as_bytes().to_vec(),
      H256::zero(),
      Some("four".as_bytes().to_vec()),
      Some("third".as_bytes().to_vec())
    ));

    assert_ok!(DidModule::lock(Origin::signed(4), 100, 5));

  });
}

#[test]
fn should_not_pass_lock() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

  });
}

#[test]
fn should_pass_unlock() {
  new_test_ext().execute_with(|| {

    prepare_dids_for_test();

    Timestamp::set_timestamp(42);

    assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));

    Timestamp::set_timestamp(50);
    assert_ok!(DidModule::unlock(Origin::signed(2), 10));

    assert_eq!(Balances::free_balance(&2), 8910);
  });
}

#[test]
fn should_pass_transfer() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);

    prepare_dids_for_test();

    let memo =b"transfer test";
    let (user_key, _) = DidModule::identity(&1).unwrap();
    assert_ok!(DidModule::transfer(
      Origin::signed(2), 
      user_key,
      100, 
      memo.to_vec()
    ));

    let events = System::events();
    let (_, from_did) = DidModule::identity(&2).unwrap();
    let (_, to_did) = DidModule::identity(&1).unwrap();

    assert_eq!(
      events[events.len() - 1],
      EventRecord {
          phase: Phase::ApplyExtrinsic(0),
          event: Event::did(RawEvent::Transfered(from_did, to_did, 100, memo.to_vec())),
          topics: vec![],
      }
    );

    assert_eq!(Balances::free_balance(&2), 8900);
    assert_eq!(Balances::free_balance(&1), 10125);

    assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));
    assert_eq!(Balances::free_balance(&1), 10125);

    // test ads fee split
    let (user_key, _) = DidModule::identity(&3).unwrap();
    assert_ok!(DidModule::transfer(
      Origin::signed(1), 
      user_key,
      1000, 
      b"ads fee".to_vec()
    ));
    assert_eq!(Balances::free_balance(&3), 10800);
    assert_eq!(Balances::free_balance(&2), 9000);
  });
}

#[test]
fn should_pass_add_external_address() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_ok!(DidModule::add_external_address(Origin::signed(1), b"eos".to_vec(), EOS_ADDRESS.to_vec()));
    assert_ok!(DidModule::add_external_address(Origin::signed(1), b"eth".to_vec(), ETH_ADDRESS.to_vec()));
    assert_ok!(DidModule::add_external_address(Origin::signed(1), b"btc".to_vec(), BTC_ADDRESS.to_vec()));
  });
}

#[test]
fn should_pass_set_group_name() {
  new_test_ext().execute_with(|| {
    System::set_block_number(0);

    prepare_dids_for_test();

    assert_ok!(DidModule::lock(Origin::signed(2), 100, 5));
    assert_ok!(DidModule::set_group_name(Origin::signed(2), b"btc group".to_vec()));

  });
}
