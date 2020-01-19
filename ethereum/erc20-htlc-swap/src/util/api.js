import axios from './axios'

const apiRandomHash = `${process.env.VUE_APP_SIGN_HOST}/api/v1/random_number_hash`

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
