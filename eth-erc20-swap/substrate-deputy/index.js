//ethereum
var getJSON = require('get-json');
var Web3 = require('web3');
var request = require('request');
var Hex = require('hex');
var util = require('util');

const ROPSTEN = "https://ropsten.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34";
const ROPSTEN_URL = "https://api-ropsten.etherscan.io";
const CONTRACT_ADDRESS = '0xbd261550e087f19A842e375D0031a85525B9714F';
const API_KEY = 'YourApiKeyToken';

var web3 = new Web3(new Web3.providers.HttpProvider(ROPSTEN));
var version = web3.version.api;

//substrate
const { mnemonicGenerate, blake2AsHex } = require('@polkadot/util-crypto');
const Keyring = require('@polkadot/keyring').default;
const testKeyring = require("@polkadot/keyring/testing");
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { u8aToHex, u8aToU8a, stringToHex, stringToU8a, numberToHex } =require('@polkadot/util');
const WS_PROVIDER = 'ws://127.0.0.1:9944';
//const WS_PROVIDER = 'wss://substrate.chain.pro/v2/ws';

const provider = new WsProvider(WS_PROVIDER);

//log
const fs = require('fs')
const log4js = require('log4js')
log4js.configure({
  appenders: {
      file: {
          type: 'file',
          filename: `./logs/creation.log`,
          layout: {
              type: 'pattern',
              pattern: '%d{MM/dd-hh:mm.ss.SSS} %p - %m',
          }
      }
  },
  categories: {
      default: {
          appenders: ['file'],
          level: 'debug'
      }
  }
})
const logger = log4js.getLogger()

const run = async () => {
    const api = await ApiPromise.create({
    provider,
    types: {
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

    console.log('api created-----')

    var abi = await get_contract_abi();
    var abi_json = JSON.parse(abi);
    var event_map = new Map();
    var event_name_map = new Map();
    for (index in abi_json) {
        if (abi_json[index].type == 'event') {
            var event_sign = web3.eth.abi.encodeEventSignature(abi_json[index]);
            event_map.set(event_sign, abi_json[index])
            event_name_map.set(event_sign, abi_json[index].name);
        }
    }

    var latest_block_num = await get_latest_block_num();
    console.log("latest_block_num:", latest_block_num);
    // let start = latest_block_num-100;
    // let end = latest_block_num-50;

    //从上次处理后的block_num继续查，不重不漏
    let start = 6941790;
    let end = "latest";
    let address = "5FnHzLERt8crDpCG9BGVckb6uu6P5nCEEr31RkBMh6wWFhJx";

    var enent_logs = await get_contract_logs(start, end);
    if(enent_logs.length > 0) {
        for (index in enent_logs) {
            var raw = enent_logs[index].data;
            var topics = enent_logs[index].topics;
            var emit_event_json = event_map.get(topics[0]);
            var event_name = event_name_map.get(topics[0]);
            var result = web3.eth.abi.decodeLog(emit_event_json.inputs, raw, topics);
            console.log("got event", event_name);
        }

        fs.readFile(`./keys/${address}.json`, async (err, res) => {
            if (err) return console.log(err, 'read key json failed');
            const keyring = new Keyring({ type: 'sr25519' });
            const { seed } = JSON.parse(res.toString());
            const pair = keyring.addFromMnemonic(seed);

            const nonce = await api.query.system.accountNonce(address)

            kickoff_event_fetch(pair, start, end);
        }
    }

    async function kickoff_event_fetch(adminPair, fromBlock, toBlock) {
        let url = util.format(ROPSTEN_URL+'/api?module=logs&action=getLogs&fromBlock=%s&toBlock=%s&address=%s&apikey=%s', fromBlock, toBlock, CONTRACT_ADDRESS, API_KEY);
        const name_hex = stringToHex("swap");
        const url_hex = stringToHex(url);

        //test
        //const adminId = await api.query.sudo.key();
        //const keyring = testKeyring.default();
        //const adminPair = keyring.getPair(adminId.toString());

        const unsub = await api.tx.sudo.sudo( api.tx.oracle.kickoff(adminPair.publicKey, name_hex, url_hex) )
        .signAndSend(adminPair, ({ events = [], status }) => {
            console.log('Transaction status:', status.type)
        }).catch(e => {
            console.log(e, 'kickoff_event_fetch internal error')
        })
    }

    async function get_latest_block_num() {
        var latest_block_num;
        await new Promise((resolve, reject) => {
            request(util.format(ROPSTEN_URL+'/api?module=proxy&action=eth_blockNumber&apikey=%s', API_KEY), function (error, response, data) {
                if (response.statusCode == 200) {
                    var data = JSON.parse(data)
                    resolve(data.result);
                } else {
                    reject(error)
                }
            })
        }).then(result => {
            latest_block_num = parseInt(result,16);
        }).catch(err => {
            console.log(err, 'request error')
        }) 

        return latest_block_num;
     }

    async function get_contract_logs(fromBlock, toBlock) {
        var logs;
        await new Promise((resolve, reject) => {
            request(util.format(ROPSTEN_URL+'/api?module=logs&action=getLogs&fromBlock=%s&toBlock=%s&address=%s&apikey=%s', fromBlock, toBlock, CONTRACT_ADDRESS, API_KEY), function (error, response, data) {
                var data = JSON.parse(data)
                if (data.message == 'OK') {
                    resolve(data.result);
                } else {
                    reject(error)
                }
            })
        }).then(result => {
            logs = result;
        }).catch(err => {
            console.log(err, 'request error')
        })
        return logs;
    }

    async function get_contract_abi() {
        var abi;
        await new Promise((resolve, reject) => {
            request(util.format(ROPSTEN_URL+'/api?module=contract&action=getabi&address=%s&apikey=%s', CONTRACT_ADDRESS, API_KEY), function (error, response, data) {
                if (response.statusCode == 200) {
                    var data = JSON.parse(data)
                    resolve(data.result);
                } else {
                    reject(error)
                }
            })
        }).then(result => {
            abi = result;
        }).catch(err => {
            console.log('err')
        })
        return abi;
    }
}

run().catch(console.error)
