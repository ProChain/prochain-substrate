const crypto = require('crypto');
const bs58 = require('bs58');
const { blake2AsHex } = require('@polkadot/util-crypto')
const algorithm = 'aes-256-ctr';
let key = 'MySuperSecretKey';
key = crypto.createHash('sha256').update(String(key)).digest('base64').substr(0, 32);

function calculateRandomNumberHash(randomNumber, timestamp) {
	console.log('randomNumber ' + randomNumber);
	console.log('timestamp ' + timestamp.toString());

	const timestampHexStr = timestamp.toString(16);
	var timestampHexStrFormat = timestampHexStr;

	// timestampHexStrFormat should be the hex string of a 32-length byte array.
	// Fill 0 if the timestampHexStr length is less than 64
	for (var i = 0; i < 16 - timestampHexStr.length; i++) {
		timestampHexStrFormat = '0' + timestampHexStrFormat;
	}

	const timestampBytes = Buffer.from(timestampHexStrFormat, "hex");
	const newBuffer = Buffer.concat([Buffer.from(randomNumber.substring(2, 66), "hex"), timestampBytes]);
	const hash = crypto.createHash('sha256');
	hash.update(newBuffer);
	return "0x" + hash.digest('hex');
}

function calculateSwapID(randomNumberHash, receiver) {
	console.log('receiver ' + receiver.toString());

	const newBuffer = Buffer.concat([Buffer.from(randomNumberHash.substring(2, 66), "hex"), Buffer.from(receiver)]);
	const hash = crypto.createHash('sha256');
	hash.update(newBuffer);
	return "0x" + hash.digest('hex');
}

function didToHex(did) {
	const bytes = bs58.decode(did.substring(8))
	return blake2AsHex(bytes, 256)
}

function hexToDid(hex) {
	const bytes = Buffer.from(hex.slice(2), 'hex')
	const address = bs58.encode(bytes);
	const did = `did:pra:${address}`
	return did
}


var express = require('express');
var app = express();
app.use(express.static('src'));
app.use(express.static('contract-dist'));

app.get("/getRandomNumberHash", function (req, res) {
	var randomNumber = req.query.randomNumber;
	var receiver = req.query.receiver;
	const timestamp = Math.floor(Date.now() / 1000);

	let randomNumberHash = calculateRandomNumberHash(randomNumber, timestamp);
	console.log('randomNumberHash ' + randomNumberHash.toString('hex'));

	let swapID = calculateSwapID(randomNumberHash, receiver);
	console.log('swapID ' + swapID.toString('hex'));

	let data = { timestamp: timestamp, swapID: swapID, randomNumberHash: randomNumberHash };

	res.json({ data, message: 'success', code: 0 });
});

app.get('/', function (req, res) {
	res.render('index.html');
});


app.listen(3005, function () {
	console.log('app listening on port 3005');
});
