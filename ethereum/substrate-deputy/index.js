//ethereum web3
let Web3 = require('web3');
let util = require('util');
let request = require('request');

const API_KEY = 'T845RJWFC5DV7F5Y4QZPZXK1AQF5ZENUHT';

//ropsten
//const ROPSTEN = "https://ropsten.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34";
//const CONTRACT_ADDRESS = '0x49e532fa0d95195d6a07be152e481c67715149eb';
//const API_URL = "https://api-ropsten.etherscan.io";
//let web3 = new Web3(new Web3.providers.HttpProvider(ROPSTEN));
//const WS_PROVIDER = 'ws://127.0.0.1:9944';

//mainnet
const MAINNET = "https://mainnet.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34";
const CONTRACT_ADDRESS = '0x415379f5d396feab48cd26d6ba5e5afdbe9c5e15';
let web3 = new Web3(new Web3.providers.HttpProvider(MAINNET));
const API_URL = "https://api-cn.etherscan.com";
const WS_PROVIDER = 'wss://substrate.chain.pro/v2/ws';

//substrate
const Keyring = require('@polkadot/keyring').default;
//const testKeyring = require("@polkadot/keyring/testing");
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { stringToHex } = require('@polkadot/util');
const globalNonce = {}

//block number step
const FETCH_STEP = 12;
const SLEEP_TIME = 5;
const LOOP_TIME = 6;

const provider = new WsProvider(WS_PROVIDER);

const AUTH_ADDRESS = "5FnHzLERt8crDpCG9BGVckb6uu6P5nCEEr31RkBMh6wWFhJx";

const fs = require('fs')
const datastore = require('nedb-promise')
const log4js = require('log4js')
log4js.configure({
	appenders: {
		file: {
			type: 'dateFile',
			filename: `./logs/deputy.log`,
			layout: {
				type: 'pattern',
				pattern: '%d{yyyy/MM/dd-hh:mm.ss} %p - %m',
			},
			alwaysIncludePattern: true,
			daysToKeep: 100
		},
		console: { type: 'console' },
		error_file: {
			type: 'dateFile',
			filename: `./logs/error.log`,
			alwaysIncludePattern: true,
			daysToKeep: 100,
			layout: {
				type: 'pattern',
				pattern: '%d{yyyy/MM/dd-hh:mm.ss} %p - %m',
			}
		}
	},
	categories: {
		default: {
			appenders: ['file', 'console'],
			level: 'debug'
		},
		error_log: { appenders: ['error_file'], level: 'error' }
	}
})
const logger = log4js.getLogger()
logger.level = 'debug';

//const sleep = (waitTimeInMs) => new Promise(resolve => setTimeout(resolve, waitTimeInMs));
let sleep = require('sleep');

const run = async () => {
	let db = datastore({ filename: './nedb/datafile', autoload: true })
	const api = await ApiPromise.create({
		provider,
		types: {
			"Did": "Vec<u8>",
			"ExternalAddress": {
				"btc": "Vec<u8>",
				"eth": "Vec<u8>",
				"eos": "Vec<u8>"
			},
			"LockedRecords": {
				"locked_time": "Moment",
				"locked_period": "Moment",
				"locked_funds": "Balance",
				"rewards_ratio": "u64",
				"max_quota": "u64"
			},
			"UnlockRecords": {
				"unlock_time": "Moment",
				"unlock_funds": "Balance"
			},
			"MetadataRecord": {
				"address": "AccountId",
				"superior": "Hash",
				"creator": "AccountId",
				"did_ele": "Vec<u8>",
				"locked_records": "Option<LockedRecords<Balance, Moment>>",
				"unlock_records": "Option<UnlockRecords<Balance, Moment>>",
				"social_account": "Option<Hash>",
				"subordinate_count": "u64",
				"group_name": "Option<Vec<u8>>",
				"external_address": "ExternalAddress"
			},
			"AdsMetadata": {
				"advertiser": "Vec<u8>",
				"topic": "Vec<u8>",
				"total_amount": "Balance",
				"surplus": "Balance",
				"gas_fee_used": "Balance",
				"single_click_fee": "Balance",
				"create_time": "Moment",
				"period": "Moment"
			},
			"EventHTLC": {
				"eth_contract_addr": "Vec<u8>",
				"htlc_block_number": "BlockNumber",
				"event_block_number": "BlockNumber",
				"expire_height": "u32",
				"random_number_hash": "Vec<u8>",
				"swap_id": "Hash",
				"event_timestamp": "u64",
				"htlc_timestamp": "u64",
				"sender_addr": "Vec<u8>",
				"sender_chain_type": "HTLCChain",
				"receiver_addr": "AccountId",
				"receiver_chain_type": "HTLCChain",
				"recipient_addr": "Vec<u8>",
				"out_amount": "Balance",
				"event_type": "HTLCType"
			},
			"HTLCChain": {
				"_enum": [
					"ETHMain",
					"PRA"
				]
			},
			"HTLCStates": {
				"_enum": [
					"INVALID",
					"OPEN",
					"COMPLETED",
					"EXPIRED"
				]
			},
			"EventLogSource": {
				"event_name": "Vec<u8>",
				"event_url": "Vec<u8>"
			},
			"HTLCType": {
				"_enum": [
					"HTLC",
					"Claimed",
					"Refunded"
				]
			}
		}
	})
	logger.info('api created -----')

	let abi = await get_contract_abi()
	let abi_json = JSON.parse(abi);
	let event_map = new Map();
	let event_name_map = new Map();
	for (index in abi_json) {
		if (abi_json[index].type == 'event') {
			let event_sign = await web3.eth.abi.encodeEventSignature(abi_json[index]);
			event_map.set(event_sign, abi_json[index])
			event_name_map.set(event_sign, abi_json[index].name);
		}
	}

	//key pair
	let res = fs.readFileSync(`./keys/${AUTH_ADDRESS}.json`, 'utf8');
	const keyring = new Keyring({ type: 'sr25519' });
	const { seed } = JSON.parse(res.toString());
	const pair = keyring.addFromMnemonic(seed);

	let latest_block_num = await get_latest_block_num()
	let from = latest_block_num - FETCH_STEP - 1;
	let to = from + FETCH_STEP;

	while (true) {
		//sleep 60s to fetch event data
		sleep.sleep(LOOP_TIME);

		//从上次处理后的block_num继续查
		let htlc_events = await db.findOne({ event_type: 'htlc_events' })
		let record_id = "";
		if (htlc_events === null) {
			let result = await db.insert({ event_type: 'htlc_events', from: from, to: to, });

			record_id = result._id;
			logger.info("init insert _id ", record_id);
		} else {
			from = htlc_events.from;
			to = htlc_events.to;
			record_id = htlc_events._id;
		}

		latest_block_num = await get_latest_block_num()
		if (to + FETCH_STEP / 2 >= latest_block_num) {
			logger.info("to + FETCH_STEP", to, FETCH_STEP / 2, " >= latest_block_num ", latest_block_num, ", will continue");
			continue;
		}

		logger.info("from:", from, ", to:", to);

		let enent_logs = await get_contract_logs(from, to);
		if (enent_logs != undefined) {
			for (index in enent_logs) {
				let topics = enent_logs[index].topics;
				let event_name = event_name_map.get(topics[0]);
				logger.info("get_contract_logs: ", event_name);
			}

			try {
				const nonce = await api.query.system.accountNonce(pair.address);
				let post_data = '{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0x415379f5d396feab48cd26d6ba5e5afdbe9c5e15", "fromBlock":"0x' + from.toString(16) + '","toBlock":"0x' + to.toString(16) + '"}],"id": 1}';
				console.log('post_data: ', post_data, ", nonce: ", nonce.toString());

				await api.tx.oracle.kickoff(stringToHex("infura"), stringToHex(MAINNET), stringToHex(post_data))
					.signAndSend(pair, { nonce }, ({ events = [], status }) => {
						logger.info("Transaction status:", status.type);

						let success = true;
						if (status.isFinalized) {
							logger.info("Completed at block hash", status.asFinalized.toHex());
							logger.info("Events:");

							events.forEach(({ phase, event: { data, method, section } }) => {
								logger.info('phase:\t', phase.toString(), `: ${section}.${method}`, data.toString())
								if (method.includes('ExtrinsicFailed')) {
									success = false
								}
							});
						}

						if (status.error) {
							logger.error(`status error, submit result failed`);
							success = false;
						}

						if (success) {
							from = to + 1;
							to = from + FETCH_STEP;
							db.update({ _id: record_id }, { $set: { from: from, to: to } });
							logger.info("success, then update _id", record_id);

							//sleep 5s to flush db data
							sleep.sleep(SLEEP_TIME);
						}
					}).catch(e => {
						logger.error(e, 'kickoff_event_fetch internal error');
					})
			}
			catch (error) {
				logger.error(error, 'sign error-----')
			}
		} else {
			from = to + 1;
			to = from + FETCH_STEP;
			db.update({ _id: record_id }, { $set: { from: from, to: to } });
			logger.info("get empty logs, then update _id", record_id);

			//sleep 5s to flush db data
			sleep.sleep(SLEEP_TIME);
		}
	}

	// let enent_logs = await get_contract_logs(from, to);
	// if (enent_logs != undefined) {
	// 	for (index in enent_logs) {
	// 		let raw = enent_logs[index].data;
	// 		let topics = enent_logs[index].topics;
	// 		let emit_event_json = event_map.get(topics[0]);
	// 		let event_name = event_name_map.get(topics[0]);
	// 		let result = web3.eth.abi.decodeLog(emit_event_json.inputs, raw, topics);
	// 		logger.info("got events", event_name);
	// 	}

	// 	let res = fs.readFileSync(`./keys/${AUTH_ADDRESS}.json`, 'utf8');
	// 	const keyring = new Keyring({ type: 'sr25519' });
	// 	const { seed } = JSON.parse(res.toString())
	// 	const pair = keyring.addFromMnemonic(seed)
	// 	const nonce = await api.query.system.accountNonce(pair.address);

	// 	let url = util.format(API_URL + '/api?module=logs&action=getLogs&fromBlock=%s&toBlock=%s&address=%s&apikey=%s', from, to, CONTRACT_ADDRESS, API_KEY);
	// 	const name_hex = stringToHex("swap");
	// 	const url_hex = stringToHex(url);

	// 	logger.info("url:", url);
	// 	api.tx.oracle.kickoff(name_hex, url_hex)
	// 		.signAndSend(pair, { nonce }, ({ events = [], status }) => {
	// 			logger.info("Transaction status:", status.type);
	// 			let success = true;

	// 			if (status.isFinalized) {
	// 				logger.info("Completed at block hash", status.asFinalized.toHex());
	// 				logger.info("Events:");

	// 				events.forEach(({ phase, event: { data, method, section } }) => {
	// 					logger.info('\t', phase.toString(), `: ${section}.${method}`, data.toString())
	// 					if (method.includes('ExtrinsicFailed')) {
	// 						success = false
	// 					}
	// 				});

	// 				if (success) {
	// 					from = to + 1;
	// 					to = from + FETCH_STEP;
	// 					db.update({ _id: record_id }, { $set: { from: from, to: to } });
	// 					logger.info("inside update _id", record_id);
	// 				}

	// 				sleep(5000).then(() => {
	// 					logger.info("sleep 5s to flush db data");
	// 					process.exit(0);
	// 				})
	// 			}
	// 		}).catch(e => {
	// 			logger.error(e, 'kickoff_event_fetch internal error');
	// 		})
	// }

	// from = to + 1;
	// to = from + FETCH_STEP;
	// db.update({ _id: record_id }, { $set: { from: from, to: to } });
	// logger.info("outside update, from:", from, ", to:", to);

	// sleep(5000).then(() => {
	// 	logger.info("sleep 5s to flush db data");
	// 	process.exit(0);
	// })

	async function get_latest_block_num() {
		let latest_block_num;
		await new Promise((resolve, reject) => {
			request(util.format(API_URL + '/api?module=proxy&action=eth_blockNumber&apikey=%s', API_KEY), function (error, response, data) {
				if (response.statusCode == 200) {
					let data_json = JSON.parse(data)
					resolve(data_json.result);
				} else {
					reject(error)
				}
			})
		}).then(result => {
			latest_block_num = parseInt(result, 16);
		}).catch(err => {
			logger.info(err, 'get_latest_block_num return error')
		})

		return latest_block_num;
	}

	async function get_contract_logs(fromBlock, toBlock) {
		let logs;
		await new Promise((resolve, reject) => {
			request(util.format(API_URL + '/api?module=logs&action=getLogs&fromBlock=%s&toBlock=%s&address=%s&apikey=%s', fromBlock, toBlock, CONTRACT_ADDRESS, API_KEY), function (error, response, data) {
				let data_json = JSON.parse(data);
				if (data_json.message == 'OK') {
					resolve(data_json.result);
				} else {
					reject(error)
				}
			})
		}).then(result => {
			logs = result;
		}).catch(err => {
			if (err != null) {
				logger.info(err, 'get_contract_logs return error')
			}
		})
		return logs;
	}

	async function get_contract_abi() {
		let abi;
		await new Promise((resolve, reject) => {
			request(util.format(API_URL + '/api?module=contract&action=getabi&address=%s&apikey=%s', CONTRACT_ADDRESS, API_KEY), function (error, response, data) {
				if (response != null && response.statusCode != null && response.statusCode == 200) {
					let data_json = JSON.parse(data)
					resolve(data_json.result);
				} else {
					reject(error)
				}
			})
		}).then(result => {
			abi = result;
		}).catch(err => {
			logger.info(err, 'get_contract_abi return error')
		})
		return abi;
	}
}

run().catch(console.error);
