use babe_primitives::AuthorityId as BabeId;
use grandpa_primitives::AuthorityId as GrandpaId;
use primitives::{Pair, Public};
use prochain_runtime::{
    AccountId, BabeConfig, BalancesConfig, ContractsConfig, GenesisConfig, GrandpaConfig, IndicesConfig, SudoConfig,
    SystemConfig, DemocracyConfig, SessionConfig, Perbill, StakingConfig, CouncilConfig, TechnicalCommitteeConfig, SessionKeys, WASM_BINARY, StakerStatus, did::{MILLICENTS, DOLLARS},
};
use substrate_service;

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

fn session_keys(grandpa: GrandpaId, babe: BabeId) -> SessionKeys {
	SessionKeys { grandpa, babe, }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId) {
    (
        get_from_seed::<AccountId>(&format!("{}//stash", seed)),
        get_from_seed::<AccountId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
    )
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
                || {
                    testnet_genesis(
                        vec![get_authority_keys_from_seed("Alice")],
                        get_from_seed::<AccountId>("Alice"),
                        vec![
                            get_from_seed::<AccountId>("Alice"),
                            get_from_seed::<AccountId>("Bob"),
                            get_from_seed::<AccountId>("Charlie"),
                            get_from_seed::<AccountId>("Dave"),
                            get_from_seed::<AccountId>("Eve"),
                            get_from_seed::<AccountId>("Ferdie"),
                            get_from_seed::<AccountId>("Alice//stash"),
                            get_from_seed::<AccountId>("Bob//stash"),
                            get_from_seed::<AccountId>("Charlie//stash"),
                            get_from_seed::<AccountId>("Dave//stash"),
                            get_from_seed::<AccountId>("Eve//stash"),
                            get_from_seed::<AccountId>("Ferdie//stash"),
                        ],
                        true,
                    )
                },
                vec![],
                None,
                None,
                None,
                properties,
            ),
            Alternative::LocalTestnet => ChainSpec::from_genesis(
                "Local Testnet",
                "local_testnet",
                || {
                    testnet_genesis(
                        vec![
                            get_authority_keys_from_seed("Alice"),
                            get_authority_keys_from_seed("Bob"),
                        ],
                        get_from_seed::<AccountId>("Alice"),
                        vec![
                            get_from_seed::<AccountId>("Alice"),
                            get_from_seed::<AccountId>("Bob"),
                            get_from_seed::<AccountId>("Charlie"),
                            get_from_seed::<AccountId>("Dave"),
                            get_from_seed::<AccountId>("Eve"),
                            get_from_seed::<AccountId>("Ferdie"),
                            get_from_seed::<AccountId>("Alice//stash"),
                            get_from_seed::<AccountId>("Bob//stash"),
                            get_from_seed::<AccountId>("Charlie//stash"),
                            get_from_seed::<AccountId>("Dave//stash"),
                            get_from_seed::<AccountId>("Eve//stash"),
                            get_from_seed::<AccountId>("Ferdie//stash"),
                        ],
                        true,
                    )
                },
                vec![],
                None,
                None,
                None,
                properties,
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

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    const ENDOWMENT: u128 = 1_000 * DOLLARS;
    const STASH: u128 = 1 * DOLLARS;

    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        indices: Some(IndicesConfig {
			ids: endowed_accounts.iter().cloned()
				.chain(initial_authorities.iter().map(|x| x.0.clone()))
				.collect::<Vec<_>>(),
		}),
        balances: Some(BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
			vesting: vec![],
		}),
        sudo: Some(SudoConfig { key: root_key }),
        babe: Some(BabeConfig {
            authorities: vec![],
        }),
        democracy: Some(DemocracyConfig::default()),
        staking: Some(StakingConfig {
			current_era: 0,
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
			}).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			.. Default::default()
		}),
        session: Some(SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), session_keys(x.2.clone(), x.3.clone()))
			}).collect::<Vec<_>>(),
		}),
        grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        collective_Instance1: Some(CouncilConfig {
			members: vec![],
			phantom: Default::default(),
		}),
		collective_Instance2: Some(TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		}),
        contracts: Some(ContractsConfig {
			current_schedule: Default::default(),
			gas_price: 1 * MILLICENTS,
		}),
    }
}
