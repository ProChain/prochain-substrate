import { getJSONByName } from '@/util/api'

const App = {
	web3Provider: null,
	contracts: {},
	account: null,
	htlcIntance: null,
	heightSpan: 100,
	swapID: '',
	recipientAddr: '0xCF5bECb7245E2e6eE2E092F0BD63F6Bd79eF19Fe',

	async init() {
		await App.initWeb3()
	},

	async initWeb3() {
		// Modern dapp browsers...
		if (window.ethereum) {
			App.web3Provider = window.ethereum
			try {
				// Request account access
				await window.ethereum.enable()
			} catch (error) {
				// User denied account access...
				console.error('User denied account access')
			}
		} else if (window.web3) { // Legacy dapp browsers...
			App.web3Provider = window.web3.currentProvider
		} else { // If no injected web3 instance is detected, fall back to Ganache
			App.web3Provider = new Web3.providers.HttpProvider('http://localhost:9545')
		}
		App.web3 = new Web3(App.web3Provider)

		var account = App.getAccountParam()

		if (null == account) {
			console.log('initAccount')
			App.initAccount()
		} else {
			App.account = account
			console.log('account:' + account)
		}

		await App.initContract()
	},

	initAccount() {
		web3.eth.getAccounts(function(error, accounts) {
			App.account = accounts[0]
		})
	},

	async initContract() {
		const data = await getJSONByName('ERC20HTLCLite')
		App.contracts.htlcContract = TruffleContract(data)
		App.contracts.htlcContract.setProvider(App.web3Provider)

		const deployed = await App.contracts.htlcContract.deployed()
		App.htlcIntance = deployed
		App.getHTLCStatus()

		const mainNet = await getJSONByName('ProTokenMainnet')
		App.contracts.praContract = TruffleContract(mainNet)
		App.contracts.praContract.setProvider(App.web3Provider)

		const praIntance = await App.contracts.praContract.deployed()
		App.praIntance = praIntance
		App.getPraStatus()
	},

	getHTLCStatus() {
		App.htlcIntance.paused().then(function(paused) {
			console.log('is paused:', paused)
			App.paused = paused
			if (paused) {
				alert('提示合约已经暂停')
			}
		}).catch(function(err) {
			console.log(err.message)
		})

		App.htlcIntance.praContractAddr().then(function(praContractAddr) {
			console.log('praContractAddr:', praContractAddr)
			App.praContractAddr = praContractAddr
		}).catch(function(err) {
			console.log(err.message)
		})

		App.htlcIntance.owner().then(function(owner) {
			console.log('owner:', owner)
			App.owner = owner
		}).catch(function(err) {
			console.log(err.message)
		})
	},

	getPraStatus() {
		App.praIntance.totalSupply().then(function(totalSupply) {
			console.log('totalSupply:', totalSupply.toString())
			App.totalSupply = totalSupply
		}).catch(function(err) {
			console.log(err.message)
		})
	},

	getAccountParam() {
		var reg = new RegExp('(^|&)account=([^&]*)(&|$)')
		var r = window.location.search.substr(1).match(reg)
		if (r != null) return unescape(r[2])
		return null
	}
}

export default App
