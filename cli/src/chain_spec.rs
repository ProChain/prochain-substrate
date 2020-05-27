// Copyright 2018-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use sc_chain_spec::ChainSpecExtension;
use sp_core::{Pair, Public, crypto::UncheckedInto, sr25519};
use serde::{Serialize, Deserialize};
use node_runtime::{
    AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, ContractsConfig, CouncilConfig, DemocracyConfig,
    GrandpaConfig, ImOnlineConfig, IndicesConfig, SessionConfig, SessionKeys, StakerStatus, StakingConfig, ElectionsPhragmenConfig, SudoConfig,
    SystemConfig, TechnicalCommitteeConfig, DidConfig, AdsConfig, WASM_BINARY,
};
use node_runtime::Block;
use node_runtime::constants::currency::*;
use sc_service::ChainType;
use hex_literal::hex;
use std::fs::File;
use std::io::Read;
use sc_telemetry::TelemetryEndpoints;
use grandpa_primitives::{AuthorityId as GrandpaId};
use sp_consensus_babe::{AuthorityId as BabeId};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};
use hex::FromHex;

pub use node_primitives::{AccountId, Balance, Signature};
pub use node_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const PRA_PROPERTIES: &str = r#"
		{
			"tokenDecimals": 15,
			"tokenSymbol": "PRM"
		}"#;

#[derive(Serialize, Deserialize)]
struct Allocation {
    balances: Vec<(String, String)>,
}

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<
    GenesisConfig,
    Extensions,
>;

/// Flaming Fir testnet generator
pub fn flaming_fir_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
}

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys { grandpa, babe, im_online, authority_discovery }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and pallet_session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

// Give each initial participant the allocation,
fn get_initial_allocation() -> Result<(Vec<(AccountId, Balance)>, Balance), String> {
    let mut file = File::open("initial_drop.json").expect("Unable to open");
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let json: Allocation = serde_json::from_str(&data).unwrap();
    let balances_json = json.balances;

    let balances: Vec<(AccountId, Balance)> = balances_json.clone().into_iter().map(|e| {
        return (
            <[u8; 32]>::from_hex(e.0).unwrap().into(),
            e.1.to_string().parse::<Balance>().unwrap(),
        );
    }).collect();

    let total: Balance = balances_json.into_iter().map(|e| {
        e.1.to_string().parse::<Balance>().unwrap()
    }).sum();
    Ok((balances, total))
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    enable_println: bool,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            // hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
            // hex!["0c98c49f1861d5f6ed9ea27230796a76878abbfbfb9716c64b2c7479a2197435"].into(),
            // hex!["74a27632efacf7bbc58a2e9f8d27a46a9f3de7d17bbd9b69da5d26b5f4b84133"].into(),
        ]
    });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 70_000_000 * DOLLARS;
    const STASH: Balance = 100_000 * DOLLARS;

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned()
                .map(|k| (k, ENDOWMENT))
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.0.clone(), session_keys(
                    x.2.clone(),
                    x.3.clone(),
                    x.4.clone(),
                    x.5.clone(),
                ))
            }).collect::<Vec<_>>(),
        }),
        pallet_staking: Some(StakingConfig {
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
            }).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_elections_phragmen: Some(ElectionsPhragmenConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
        pallet_sudo: Some(SudoConfig {
            key: root_key,
        }),
        pallet_babe: Some(BabeConfig {
            authorities: vec![],
        }),
        pallet_im_online: Some(ImOnlineConfig {
            keys: vec![],
        }),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![],
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        did: Some(DidConfig {
            genesis_account: hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
            min_deposit: 10 * DOLLARS,
            base_quota: 250,
            fee_to_previous: 25 * DOLLARS,
        }),
        ads: Some(AdsConfig {
            contract: hex!["0c98c49f1861d5f6ed9ea27230796a76878abbfbfb9716c64b2c7479a2197435"].into(),
            min_deposit: 500 * DOLLARS,
        }),
    }
}

fn development_config_genesis() -> GenesisConfig {
    let _root_key: AccountId = hex![
		// 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
		"22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"
	].into();
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        true,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    let properties = serde_json::from_str(PRA_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        None,
        properties,
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        false,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}

fn prochain_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
    let initial_allocation_json = get_initial_allocation().unwrap();
    let initial_allocation = initial_allocation_json.0;
    let initial_total = initial_allocation_json.1;

    const STASH: Balance = 10_000 * DOLLARS;
    let total_stash: Balance = 10_000 * initial_authorities.len() as u128 * DOLLARS;
    let endowed_amount: Balance = 70_000_000 * DOLLARS - initial_total - total_stash;

    let num_endowed_accounts = endowed_accounts.len();

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned()
                .map(|k| (k, endowed_amount))
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .chain(initial_allocation.iter().map(|x| (x.0.clone(), x.1.clone())))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.0.clone(), session_keys(
                    x.2.clone(),
                    x.3.clone(),
                    x.4.clone(),
                    x.5.clone(),
                ))
            }).collect::<Vec<_>>(),
        }),
        pallet_staking: Some(StakingConfig {
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
            }).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_elections_phragmen: Some(ElectionsPhragmenConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println: false, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
        pallet_sudo: Some(SudoConfig {
            key: root_key,
        }),
        pallet_babe: Some(BabeConfig {
            authorities: vec![],
        }),
        pallet_im_online: Some(ImOnlineConfig {
            keys: vec![],
        }),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![],
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        did: Some(DidConfig {
            genesis_account: hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
            min_deposit: 10 * DOLLARS,
            base_quota: 250,
            fee_to_previous: 25 * DOLLARS,
        }),
        ads: Some(AdsConfig {
            contract: hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
            min_deposit: 100 * DOLLARS,
        }),
    }
}

/// prochain testnet config
pub fn prochain_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/123.207.140.69/tcp/30333/p2p/QmaSMt4WafkcuYQNiTts3XfhmZymVVytfJaNfBYfjDMa2q".parse().unwrap(),
    ];
    // let boot_nodes = vec![];
    let properties = serde_json::from_str(PRA_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Prochain Testnet",
        "prochain_testnet",
        ChainType::Live,
        || {
            prochain_genesis(
                vec![
                    (
                        // 5GTDPKDnqavJMa9Wsqp8ospmjs3riV6zg8obQNLPqn7c3wh5
                        hex!["c21bbbee5958ccc3be18979a75c229dfb8ad9319218eddc71b0989c796408b13"].into(),
                        // 5CSMAWVYBoHpMYTjxS2AWYEo74ntuHNdQFuxzuD1Cz37ffcm
                        hex!["1081bdf73aade46b14187607f1ff944876cc886282e2af9aab0c6a5b5ddb6d02"].into(),
                        // 5FaXqX3rHbsE31FoJAZVE1w7jiRdrURL7EBbQ6SLY68nQ4W9
                        hex!["9b74552f41e4662db1193d588f752cfb4d7d4947eac64e9c71bf6a004cda2901"].unchecked_into(),
                        // 5FmzqTxeUqaj1gZeUUo8KUx61AY3NmJgHYZD686r3J6DmHiL
                        hex!["a432eb5161754017b94b7ec93d6d45b4ddc1c3137d0bef694c9211a146b95969"].unchecked_into(),
                        // 5HZAZSLmQj1WvSP1uM3LGeFqimYjkamsNsLmNedDmfbBmsL2
                        hex!["f2e1edb9244c3dc5307ae45c76aab1c2f3524ab6aa0d03152a80c0e7b70cc902"].unchecked_into(),
                        // 5CDWPLg2NMYhSPn1UVcuAR2Fzn948FJvgxsuLWrCLGexjDBd
                        hex!["06b697db1eb33fe300e9b292213da1e659f5e27f6f4cf28bb9b141bfc3af6079"].unchecked_into(),
                    ),
                    (
                        // 5ECkqhw4dCCwX6zzanbjNCgA9VxMeK6qR7snfrWefbumLBrQ
                        hex!["5e9c79234b5e55348fc60f38b28c2cc60d8bb4bd2862eae2179a05ec39e62658"].into(),
                        // 5H4F6CRnkUaMGSYckLXMViNTECpa4pC8KNAKW4X9qjKF73CC
                        hex!["dcd30ff89083ced6197c950ab9409989ebddbee39ebe3e771ad6cd352da1d178"].into(),
                        // 5CxQtKD3zcrqqGBRVN6jJqNeRWHC9WtF5ph6vN89r7EjV82R
                        hex!["276fa1242eda3dfb9bdabd3a3c87b07c7f52ef74deec6bb980867a99ef143db7"].unchecked_into(),
                        // 5DXCactxvDJtXyR4auEd3WDc2Z1ZgQWETUbCDm2o476VBX1h
                        hex!["40714e20c9ed2915de752e2a0a9d952be406afbd68820ec292c3e8016b592e62"].unchecked_into(),
                        // 5GhERWzdYM5qwQfWduULHPRKwgB9MGtmNXyresnbX4vWWv11
                        hex!["cccca9f7232c7ee9dc60dcc301425dff18087b79963020166b4b6cd432bb3075"].unchecked_into(),
                        // 5EeDYacpVo7zvjno5VGiHJocc6gvHFj3envE4enALRLxayrN
                        hex!["7207812127e3c66678df599e100b93426b2a28b2bf85fb57685351e00ab2f162"].unchecked_into(),
                    ),
                    (
                        // 5HLCVgpCPQrSXasbzHahRR9HuT1uEj2NQmMiEC4fS3J15azc
                        hex!["e8fe40d68fc1efe504b9a709bd8591e4402f3162b8297155708e34a46cd7272d"].into(),
                        // 5HYyYS4tVA5QCH1WfyfWfSrZpTnEoCq7V7MsbjgaGYU5FK4C
                        hex!["f2bcd74b3e1775d68c5aadd804500b312e05a434ec3ad4df2b1db91a666f7601"].into(),
                        // 5CYgmnEBHHxT5BdZxYqYqBgSDE1SvCY7SbT4NeLSNZX1Dm7R
                        hex!["155738a81c5eb4040922493a4f834b7b935013061af1d1ea85264addc43bf84f"].unchecked_into(),
                        // 5EHYQX755SfGXKtbaxGZARwsL2D5d1nxt3GfjshTNGypSfe5
                        hex!["6242d7c10c7887734f367c6ab2f4bbc5ef7bde7a5aff4fbbbb35cbdbc7898231"].unchecked_into(),
                        // 5Eh1BeG8xNhk6NmmK2VYVgvxnMdvvfvMQomUsN3XRktaH5v6
                        hex!["7427a13d0757415eeadeffc33c490a402f3bf46a6dfba2f6a1145ae3cd747c6d"].unchecked_into(),
                        // 5Gj1Q4eVG8tdEVyGbrLBK6mdp9df1wmsxxHMR4nn7vA1n8hw
                        hex!["ce274ef545a0ca52952d7e3043431afc1007ba4b6a440e6b60517cf817a3c03e"].unchecked_into(),
                    ), (
                        // 5DSVnbWsmju4raE6nALKAdg6iiJau87vU6cvpwexVZ3Pr2f2
                        hex!["3cdb0017aef46c82411926506f0335157cb3b706cd03f1b65c99bdf7b0288444"].into(),
                        // 5Cetf6wLBM55RxmDqrqLnLMCsaNgJRgEs5iMp945nhKZDbPz
                        hex!["1a12b3a84fba82e444b51586f62ec7ed41b8ce09b6a7bd4639dd4e4c6c782e33"].into(),
                        // 5DnEMQX5BgSJZ235UQidEDwRvt4xGjPUE8M3hsFiRpXakipz
                        hex!["4be7f3b31f770d59d29e38d19592d65ec3f1ea72c62b35df2094d093ba7b0076"].unchecked_into(),
                        // 5Dvj8ZbJc8eqJpqKLf4qLvc463iXcgwK7zbdPcuqK69n73U3
                        hex!["5262cade2d3e92a6f164c8ef93f9e5c1570761e92b5e98e31f721cf43cb9913b"].unchecked_into(),
                        // 5G3ix9U3gdMFbTQgLrvwuKyya1ok9FkECvri21cEe56968Rm
                        hex!["b0315f660d8a57f6833b9f6403ae3c2eb4a015fc0431e8f1ff24d0c65fefaf52"].unchecked_into(),
                        // 5G1BHvm3h9D3PriPiqpbQFjR9p7JPrQtwknS7VevpKqGS8ym
                        hex!["ae404eede3214ba00d5a34964820d5b6da578b8d0199527b20c27d2e0e04de29"].unchecked_into(),
                    )
                ],
                // 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
                hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
                vec![
                    // 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
                    hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
                ],
            )
        },
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(TELEMETRY_URL.to_string(), 0)])
            .expect("telemetry url is invalid; qed")),
        None,
        properties,
        Default::default(),
    )
}

/// prochain mainnet config
pub fn prochain_mainnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/123.207.140.69/tcp/30333/p2p/QmaSMt4WafkcuYQNiTts3XfhmZymVVytfJaNfBYfjDMa2q".parse().unwrap(),
    ];
    // let boot_nodes = vec![];
    let properties = serde_json::from_str(PRA_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Prochain Mainnet",
        "Dana",
        ChainType::Live,
        || {
            prochain_genesis(
                vec![
                    (
                        // 5GTDPKDnqavJMa9Wsqp8ospmjs3riV6zg8obQNLPqn7c3wh5
                        hex!["c21bbbee5958ccc3be18979a75c229dfb8ad9319218eddc71b0989c796408b13"].into(),
                        // 5CSMAWVYBoHpMYTjxS2AWYEo74ntuHNdQFuxzuD1Cz37ffcm
                        hex!["1081bdf73aade46b14187607f1ff944876cc886282e2af9aab0c6a5b5ddb6d02"].into(),
                        // 5FaXqX3rHbsE31FoJAZVE1w7jiRdrURL7EBbQ6SLY68nQ4W9
                        hex!["9b74552f41e4662db1193d588f752cfb4d7d4947eac64e9c71bf6a004cda2901"].unchecked_into(),
                        // 5FmzqTxeUqaj1gZeUUo8KUx61AY3NmJgHYZD686r3J6DmHiL
                        hex!["a432eb5161754017b94b7ec93d6d45b4ddc1c3137d0bef694c9211a146b95969"].unchecked_into(),
                        // 5HZAZSLmQj1WvSP1uM3LGeFqimYjkamsNsLmNedDmfbBmsL2
                        hex!["f2e1edb9244c3dc5307ae45c76aab1c2f3524ab6aa0d03152a80c0e7b70cc902"].unchecked_into(),
                        // 5CDWPLg2NMYhSPn1UVcuAR2Fzn948FJvgxsuLWrCLGexjDBd
                        hex!["06b697db1eb33fe300e9b292213da1e659f5e27f6f4cf28bb9b141bfc3af6079"].unchecked_into(),
                    ),
                    (
                        // 5ECkqhw4dCCwX6zzanbjNCgA9VxMeK6qR7snfrWefbumLBrQ
                        hex!["5e9c79234b5e55348fc60f38b28c2cc60d8bb4bd2862eae2179a05ec39e62658"].into(),
                        // 5H4F6CRnkUaMGSYckLXMViNTECpa4pC8KNAKW4X9qjKF73CC
                        hex!["dcd30ff89083ced6197c950ab9409989ebddbee39ebe3e771ad6cd352da1d178"].into(),
                        // 5CxQtKD3zcrqqGBRVN6jJqNeRWHC9WtF5ph6vN89r7EjV82R
                        hex!["276fa1242eda3dfb9bdabd3a3c87b07c7f52ef74deec6bb980867a99ef143db7"].unchecked_into(),
                        // 5DXCactxvDJtXyR4auEd3WDc2Z1ZgQWETUbCDm2o476VBX1h
                        hex!["40714e20c9ed2915de752e2a0a9d952be406afbd68820ec292c3e8016b592e62"].unchecked_into(),
                        // 5GhERWzdYM5qwQfWduULHPRKwgB9MGtmNXyresnbX4vWWv11
                        hex!["cccca9f7232c7ee9dc60dcc301425dff18087b79963020166b4b6cd432bb3075"].unchecked_into(),
                        // 5EeDYacpVo7zvjno5VGiHJocc6gvHFj3envE4enALRLxayrN
                        hex!["7207812127e3c66678df599e100b93426b2a28b2bf85fb57685351e00ab2f162"].unchecked_into(),
                    ),
                    (
                        // 5HLCVgpCPQrSXasbzHahRR9HuT1uEj2NQmMiEC4fS3J15azc
                        hex!["e8fe40d68fc1efe504b9a709bd8591e4402f3162b8297155708e34a46cd7272d"].into(),
                        // 5HYyYS4tVA5QCH1WfyfWfSrZpTnEoCq7V7MsbjgaGYU5FK4C
                        hex!["f2bcd74b3e1775d68c5aadd804500b312e05a434ec3ad4df2b1db91a666f7601"].into(),
                        // 5CYgmnEBHHxT5BdZxYqYqBgSDE1SvCY7SbT4NeLSNZX1Dm7R
                        hex!["155738a81c5eb4040922493a4f834b7b935013061af1d1ea85264addc43bf84f"].unchecked_into(),
                        // 5EHYQX755SfGXKtbaxGZARwsL2D5d1nxt3GfjshTNGypSfe5
                        hex!["6242d7c10c7887734f367c6ab2f4bbc5ef7bde7a5aff4fbbbb35cbdbc7898231"].unchecked_into(),
                        // 5Eh1BeG8xNhk6NmmK2VYVgvxnMdvvfvMQomUsN3XRktaH5v6
                        hex!["7427a13d0757415eeadeffc33c490a402f3bf46a6dfba2f6a1145ae3cd747c6d"].unchecked_into(),
                        // 5Gj1Q4eVG8tdEVyGbrLBK6mdp9df1wmsxxHMR4nn7vA1n8hw
                        hex!["ce274ef545a0ca52952d7e3043431afc1007ba4b6a440e6b60517cf817a3c03e"].unchecked_into(),
                    ), (
                        // 5DSVnbWsmju4raE6nALKAdg6iiJau87vU6cvpwexVZ3Pr2f2
                        hex!["3cdb0017aef46c82411926506f0335157cb3b706cd03f1b65c99bdf7b0288444"].into(),
                        // 5Cetf6wLBM55RxmDqrqLnLMCsaNgJRgEs5iMp945nhKZDbPz
                        hex!["1a12b3a84fba82e444b51586f62ec7ed41b8ce09b6a7bd4639dd4e4c6c782e33"].into(),
                        // 5DnEMQX5BgSJZ235UQidEDwRvt4xGjPUE8M3hsFiRpXakipz
                        hex!["4be7f3b31f770d59d29e38d19592d65ec3f1ea72c62b35df2094d093ba7b0076"].unchecked_into(),
                        // 5Dvj8ZbJc8eqJpqKLf4qLvc463iXcgwK7zbdPcuqK69n73U3
                        hex!["5262cade2d3e92a6f164c8ef93f9e5c1570761e92b5e98e31f721cf43cb9913b"].unchecked_into(),
                        // 5G3ix9U3gdMFbTQgLrvwuKyya1ok9FkECvri21cEe56968Rm
                        hex!["b0315f660d8a57f6833b9f6403ae3c2eb4a015fc0431e8f1ff24d0c65fefaf52"].unchecked_into(),
                        // 5G1BHvm3h9D3PriPiqpbQFjR9p7JPrQtwknS7VevpKqGS8ym
                        hex!["ae404eede3214ba00d5a34964820d5b6da578b8d0199527b20c27d2e0e04de29"].unchecked_into(),
                    )
                ],
                // 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
                hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
                vec![
                    // 5CrRpNbQBTiBmTjpUgJ6mH9YRmopVweLsjffVz7muskYEo2r
                    hex!["22df4b685df33f070ae6e5ee27f745de078adff099d3a803ec67afe1168acd4f"].into(),
                ],
            )
        },
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(TELEMETRY_URL.to_string(), 0)])
            .expect("telemetry url is invalid; qed")),
        None,
        properties,
        Default::default(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full, new_light};
    use sc_service_test;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![
                authority_keys_from_seed("Alice"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            None,
            false,
        )
    }


    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        sc_service_test::connectivity(
            integration_test_config_with_two_authorities(),
            |config| new_full(config),
            |config| new_light(config),
        );
    }
}
