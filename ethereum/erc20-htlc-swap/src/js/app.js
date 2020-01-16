App = {
	web3Provider: null,
	contracts: {},
	account: null,
	htlcIntance: null,
	heightSpan: 100,
	swapID: '',
	recipientAddr: '0xCF5bECb7245E2e6eE2E092F0BD63F6Bd79eF19Fe',

	init: async function () {
		return await App.initWeb3();
	},

	initWeb3: async function () {
		// Modern dapp browsers...
		if (window.ethereum) {
			App.web3Provider = window.ethereum;
			try {
				// Request account access
				await window.ethereum.enable();
			} catch (error) {
				// User denied account access...
				console.error("User denied account access")
			}
		}

		// Legacy dapp browsers...
		else if (window.web3) {
			App.web3Provider = window.web3.currentProvider;
		}
		// If no injected web3 instance is detected, fall back to Ganache
		else {
			App.web3Provider = new Web3.providers.HttpProvider('http://localhost:9545');
		}
		web3 = new Web3(App.web3Provider);

		var account = App.getAccountParam();

		if (null == account) {
			console.log("initAccount");
			App.initAccount();
		} else {
			App.account = account;
			console.log("account:" + account);
		}

		return App.initContract();
	},

	initAccount: function () {
		web3.eth.getAccounts(function (error, accounts) {
			App.account = accounts[0];
		});
	},

	initContract: function () {
		$.getJSON('ERC20HTLC.json', function (data) {
			App.contracts.htlcContract = TruffleContract(data);
			App.contracts.htlcContract.setProvider(App.web3Provider);

			App.contracts.htlcContract.deployed().then(function (instance) {
				App.htlcIntance = instance;

				return App.getHTLCStatus();
			});
		});

		$.getJSON('ProTokenMainnet.json', function (data) {
			App.contracts.praContract = TruffleContract(data);
			App.contracts.praContract.setProvider(App.web3Provider);

			App.contracts.praContract.deployed().then(function (instance) {
				App.praIntance = instance;
				return App.getPraStatus();
			});
		});

		return App.bindEvents();
	},

	getHTLCStatus: function () {
		App.htlcIntance.paused().then(function (paused) {
			console.log("is paused:", paused);
			App.paused = paused;
			if (paused) {
				alert('提示合约已经暂停');
			}
		}).catch(function (err) {
			console.log(err.message);
		});

		App.htlcIntance.praContractAddr().then(function (praContractAddr) {
			console.log("praContractAddr:", praContractAddr);
			App.praContractAddr = praContractAddr;
		}).catch(function (err) {
			console.log(err.message);
		});

		App.htlcIntance.owner().then(function (owner) {
			console.log("owner:", owner);
			App.owner = owner;
		}).catch(function (err) {
			console.log(err.message);
		});
	},

	getPraStatus: function () {
		App.praIntance.totalSupply().then(function (totalSupply) {
			console.log("totalSupply:", totalSupply.toString());
			App.totalSupply = totalSupply;
		}).catch(function (err) {
			console.log(err.message);
		});
	},

	bindEvents: function () {
		$("#approve_new").on('click', function () {
			//TODO：若approve有效，将该按钮disable，无需再次approve

			var amount = 500000 * 1000000000000000000;
			App.praIntance.approve(App.contracts.htlcContract.address, amount).then(
				function (result) {
					if (result.receipt.status == 1) {
						console.log("status success!!");
					} else {
						console.log("status fail!!");
					}
				}
			).catch(function (err) {
				console.log(err.message);
			});
		});

		$("#htlc_new").on('click', function () {
			var amount = parseFloat(document.getElementById("input_num").value);
			if (amount !== amount || amount < 0.1) {
				alert("amount must >= 0.1");
				return;
			}

			amount = amount * 1000000000000000000;

			var randomNum = document.getElementById("random_num").value;
			var did = document.getElementById("did").value;
			if (did == null || did.length < 11) {
				alert("did must be valid");
				return;
			}

			$.get("/getRandomNumberHash?randomNumber=" + randomNum + "&receiver=" + did + "&amount=" + amount, function (data) {
				if (data == null || data == "Error" || data.code != 0) {
					alert('提示 http 500 错误，请重新发起兑换');
					return;
				}

				var timestamp = data.data.timestamp;
				var randomNumberHash = data.data.randomNumberHash;
				App.swapID = data.data.swapID;

				App.htlcIntance.htlc(randomNumberHash, timestamp, App.heightSpan, App.recipientAddr, amount, amount, did).then(
					function (result) {
						if (result.receipt.status == 1) {
							console.log("status success!!");
						} else {
							console.log("status fail!!");
						}
					}
				).catch(function (err) {
					console.log(err.message);
				});
			});
		});

		$("#claim").on('click', function () {
			var randomNum = document.getElementById("random_num").value;

			if (App.swapID == null || App.swapID === '') {
				alert('提示 Swap Id 无效，请先发起一笔htlc兑换');
				return;
			}

			console.log('App.swapID:', App.swapID);

			App.htlcIntance.claim(App.swapID, randomNum).then(
				function (result) {
					if (result.receipt.status == 1) {
						console.log("status success!!");
					} else {
						console.log("status fail!!");
					}
				}
			).catch(function (err) {
				console.log(err.message);
			});
		});

		$("#refund").on('click', function () {
			if (App.swapID == null || App.swapID === '') {
				alert('提示 Swap Id 无效，请先发起一笔htlc兑换');
				return;
			}

			App.htlcIntance.refund(App.swapID).then(
				function (result) {
					if (result.receipt.status == 1) {
						console.log("status success!!");
					} else {
						console.log("status fail!!");
					}
				}
			).catch(function (err) {
				console.log(err.message);
			});
		});
	},

	getAccountParam: function () {
		var reg = new RegExp("(^|&)account=([^&]*)(&|$)");
		var r = window.location.search.substr(1).match(reg);
		if (r != null) return unescape(r[2]); return null;
	},
};

$(function () {
	$(window).load(function () {
		App.init();
	});
});
