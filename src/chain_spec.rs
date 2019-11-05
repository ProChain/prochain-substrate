use primitives::{crypto::UncheckedInto, ed25519, sr25519, Pair};
use prochain_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig,
	SudoConfig, IndicesConfig, Perbill, Permill, TreasuryConfig, StakingConfig, SessionConfig, DemocracyConfig, GrandpaConfig, CouncilVotingConfig, did::{DOLLARS},
};
use substrate_service;

use ed25519::Public as AuthorityId;
use hex_literal::{hex_impl, hex};

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
}

fn authority_key(s: &str) -> AuthorityId {
	ed25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
		.public()
}

fn account_key(s: &str) -> AccountId {
	sr25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
		.public()
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {
		let data = r#"
		{
			"tokenDecimals": 12,
			"tokenSymbol": "PRA"
		}"#;
		let properties = serde_json::from_str(data).unwrap();
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					authority_key("Alice")
				], vec![
					account_key("Alice")
				],
					account_key("Alice")
				),
				vec![],
				None,
				None,
				None,
				properties
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					hex!("65bdf6515d15f18530a7f962b6fb0a1f746104f0bdbfab99fc620e3d1737e352").unchecked_into(), // 5EN789dARz7MZG3EB1pbGeqpVpRxjEu8dzQTgnyLTYZFtL3r
					hex!("46758e089fcdf1fa7fe48bad94f6d7e3e44cf9ae8b5af162bc3b0c74bfa53f74").unchecked_into(), // 5Df68W8T9qu8jKLJRgmpD1BfM9pcYKhrw72xkjG7nnwun2r2
					hex!("0f30261a96b72188eaa117de048a6da8af66646b50ed92d3c5f2a5e70b613ffc").unchecked_into(), // 5CQcsuwwS9WTo9p5EK38Li2uVmAHgjnA9sdq14gHsGzWd6mq
					hex!("e0c3f0fc8523ffcfba88bbbdef34c7e5f597b61bfb8a7cb67ef209246acd9a4a").unchecked_into(), // 5H9Qnn4cSQsH11p4vJ9p39TFrgsLFUyEq7b6QVTdU2qhCiju
					hex!("88894e8e3e6a19d035e02395f3b331d81b45def3cfb285bc8b6f2d128a39f904").unchecked_into(), // 5F9jA8E4GczoDpRXnvdPSgpmxJuPjfCNyznQhrprAJtvaW6V
				], vec![
					hex!("22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f").unchecked_into(), // 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
				],
					hex!("22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f").unchecked_into()
				),
				vec![],
				None,
				None,
				None,
				properties
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
			"" | "local" => Some(Alternative::LocalTestnet),
			_ => None,
		}
	}
}

fn testnet_genesis(initial_authorities: Vec<AuthorityId>, endowed_accounts: Vec<AccountId>, root_key: AccountId) -> GenesisConfig {
	const ENDOWMENT: u128 = 1_000 * DOLLARS as u128;

	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/prochain_runtime_wasm.compact.wasm").to_vec(),
			authorities: initial_authorities.clone(),
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			minimum_period: 3, // 6 second block time.
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		balances: Some(BalancesConfig {
			transaction_base_fee: 1,
			transaction_byte_fee: 0,
			existential_deposit: 0,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().cloned().map(|k|(k, ENDOWMENT)).collect(),
			vesting: vec![],
		}),
		sudo: Some(SudoConfig {
			key: root_key,
		}),
		grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().cloned().map(|x| (x, 1)).collect()
		}),
		session: Some(SessionConfig {
			validators: endowed_accounts.clone(),
			keys: endowed_accounts.iter().cloned().zip(initial_authorities.clone()).collect(),
			session_length: 6
		}),
		staking: Some(StakingConfig {
			validator_count: 5, // The ideal number of staking participants.
			minimum_validator_count: 1, // Minimum number of staking participants before emergency conditions are imposed
			sessions_per_era: 5, // The length of a staking era in sessions.
			session_reward: Perbill::from_millionths(10_000), // Maximum reward, per validator, that is provided per acceptable session.
			offline_slash: Perbill::from_percent(50_000), // Slash, per validator that is taken for the first time they are found to be offline.
			offline_slash_grace: 3, // Number of instances of offline reports before slashing begins for validators.
			bonding_duration: 30, // The length of the bonding duration in blocks.
			invulnerables: vec![], // Any validators that may never be slashed or forcibly kicked.
			stakers: vec![], // This is keyed by the stash account.
			current_era: 0, // The current era index.
			current_session_reward: 10, // Maximum reward, per validator, that is provided per acceptable session.
		}),
		democracy: Some(DemocracyConfig {
			launch_period: 1440, // How often (in blocks) new public referenda are launched.
			minimum_deposit: 10_000, // The minimum amount to be used as a deposit for a public referendum proposal.
			public_delay: 5, // The delay before enactment for all public referenda.
			max_lock_periods: 60, // The maximum number of additional lock periods a voter may offer to strengthen their vote.
			voting_period: 144, // How often (in blocks) to check for new votes.
		}),
		council_voting: Some(CouncilVotingConfig {
			cooloff_period: 360, // Period (in blocks) that a veto is in effect.
			voting_period: 60, // Period (in blocks) that a vote is open for.
			enact_delay_period: 5, // Number of blocks by which to delay enactment of successful.
		}),
		treasury: Some(TreasuryConfig {
			proposal_bond: Permill::from_millionths(50_000), // Proportion of funds that should be bonded in order to place a proposal.
			proposal_bond_minimum: 1_000_000, // Minimum amount of funds that should be placed in a deposit for making a proposal.
			spend_period: 360, // Period between successive spends.
			burn: Permill::from_millionths(100_000), // Percentage of spare funds (if any) that are burnt per spend period.
		}),
	}
}
