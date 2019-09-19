#[cfg(test)]
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
