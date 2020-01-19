import * as Actions from './constants'

export const actions = {
	[Actions.SET_TOKEN]: async({ commit }, token) => {
		commit(Actions.SET_TOKEN, token)
	}
}
