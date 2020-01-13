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

const run = async () => {
	let randomNumber = "0xaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd";

	// counted by second
	const timestamp = Math.floor(Date.now() / 1000);

	let randomNumberHash = calculateRandomNumberHash(randomNumber, timestamp);
	console.log('randomNumberHash ' + randomNumberHash.toString('hex'));

	//var did = "0x0190556d561e7761381590fdfd1b5a1dd52e976e6c9bba825d";
	//console.log("did_raw:", did);
	//var receiver = hexToDid(did);
	let receiver = "did:pra:Lt23xGimVoUNvZ3EXM9FcgBsJXzrSaUo8p";

	let id = calculateSwapID(randomNumberHash, receiver);
	console.log('swapID ' + id.toString('hex'));

	var did_raw = didToHex('did:pra:Lt23xGimVoUNvZ3EXM9FcgBsJXzrSaUo8p');
	console.log("didRaw:", did_raw);
}

run();
