const PraToken = artifacts.require("ProToken");
const ERC20AtomicSwap = artifacts.require("ERC20HTLC");

//deploy htlc to ropsten
var ProTokenRopAddr = "0xd2bc5bf7563c6d308ecb36f46f9848bb054223d1";
module.exports = function (deployer) {
	// will redeploy PraToken contract
	// deployer.deploy(PraToken, "10000000000000000", "PRM Token", "8", "PRM").then(function () {
	// 	return deployer.deploy(ERC20AtomicSwap, PraToken.address);
	// });

	deployer.deploy(ERC20AtomicSwap, ProTokenRopAddr);
};

//deploy htlc to main
var ProTokenMainAddr = "0x9041Fe5B3FDEA0f5e4afDC17e75180738D877A01";
module.exports = function (deployer) {
	deployer.deploy(ERC20AtomicSwap, ProTokenMainAddr);
};
