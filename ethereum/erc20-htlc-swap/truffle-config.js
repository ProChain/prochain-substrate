require('dotenv').config();

const WalletProvider = require("truffle-wallet-provider");
const Wallet = require('ethereumjs-wallet');

var ropstenPrivateKey = Buffer.from(process.env["ROPSTEN_PRIVATE_KEY"], "hex");
var ropstenWallet = Wallet.fromPrivateKey(ropstenPrivateKey);
var ropstenProvider = new WalletProvider(ropstenWallet, "https://ropsten.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34");

var mainNetPrivateKey = Buffer.from(process.env["MAINNET_PRIVATE_KEY"], "hex");
var mainNetWallet = Wallet.fromPrivateKey(mainNetPrivateKey);
var mainNetProvider = new WalletProvider(mainNetWallet, "https://mainnet.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34");


module.exports = {
	// See <http://truffleframework.com/docs/advanced/configuration>
	// for more about customizing your Truffle configuration!
	networks: {
		development: {
			host: "127.0.0.1",
			port: 9545,
			network_id: "*" // Match any network id
		},
		ropsten: {
			provider: ropstenProvider,
			network_id: 3,       // Ropsten's id
			gas: 5500000,        // Ropsten has a lower block limit than mainnet
			confirmations: 2,    // # of confs to wait between deployments. (default: 0)
			timeoutBlocks: 500,  // # of blocks before a deployment times out  (minimum/default: 50)
			skipDryRun: true,     // Skip dry run before migrations? (default: false for public nets ),
			from: "0xf7FeA1722F9b27B0666919A5664BaB486a4b18D3" // the contract owner
		}
		// sokol: {
		//     provider: function () {
		//         return new HDWalletProvider(mnemonic, "https://sokol.poa.network")
		//     },
		//     network_id: 77,
		//     gasPrice: 1000000000
		// }
	}
};
