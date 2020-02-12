import axios from './axios'

const apiRandomHash = `${process.env.VUE_APP_SIGN_HOST}/api/v1/random_number_hash`
const infura = 'https://mainnet.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34'

export async function getJSONByName(name) {
	return axios.get(`${process.env.VUE_APP_HOST}/${name}.json`)
}

export async function getRandomNumberHash(randomNumber, receiver, amount) {
	return axios.get(apiRandomHash, {
		params: {
			randomNumber,
			receiver,
			amount
		}
	})
}

export async function getTransactionByHash(hash) {
	return axios.post(infura, {
		jsonrpc: '2.0',
		method: 'eth_getTransactionByHash',
		params: [hash],
		id: 1
	})
}

export async function getBlockNumber() {
	return axios.post(infura, {
		jsonrpc: '2.0',
		method: 'eth_blockNumber',
		params: [],
		id: 1
	})
}
