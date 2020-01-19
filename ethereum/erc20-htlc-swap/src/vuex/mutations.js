import * as Mutations from './constants'

export const mutations = {
	[Mutations.SET_TOKEN]: (state, token) => {
		state.token = token
	},
	showLoading: (state) => {
		state.showLoading = true
	},
	hideLoading: (state) => {
		state.showLoading = false
	}
}
