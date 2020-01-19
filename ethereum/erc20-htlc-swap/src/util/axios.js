import axios from 'axios'
import store from '@/vuex'
import { getLanguage } from './common'
import vm from '../main'

const WHITELIST = [
	'get_team_logo_by_symbol',
	'dana.prabox.net'
]
let needLoadingRequestCount = 0

const startLoading = () => {
	console.log('showLoading =============')
	store.commit('showLoading')
}

const endLoading = () => {
	console.log('hideLoading==========')
	store.commit('hideLoading')
}

const tryCloseLoading = () => {
	if (needLoadingRequestCount === 0) {
		endLoading()
	}
}

const checkWhitelist = (url) => {
	let isWhitelisted = false
	for (let i in WHITELIST) {
		if (url.includes(WHITELIST[i])) {
			isWhitelisted = true
			break
		}
	}
	return isWhitelisted
}

const showFullScreenLoading = () => {
	if (needLoadingRequestCount === 0) {
		startLoading()
	}
	needLoadingRequestCount++
}

const tryHideFullScreenLoading = () => {
	if (needLoadingRequestCount <= 0) return
	needLoadingRequestCount--
	if (needLoadingRequestCount === 0) {
		setTimeout(() => {
			tryCloseLoading()
		}, 300)
	}
}

// 设置 POST 请求头
axios.defaults.headers.post['Content-Type'] = 'application/x-www-form-urlencoded'

// 设置语言
const borwserLang = getLanguage() || 'zh';
axios.defaults.headers['Accept-Language'] = borwserLang

// 表示跨域请求时是否需要使用凭证
axios.defaults.withCredentials = false

// 配置 CORS 跨域
axios.defaults.crossDomain = true

// 设置超时
axios.defaults.timeout = 5000

// 拦截器的说明
// 1、interceptor必须在请求前设置才有效。
// 2、直接为axios全局对象创建interceptor， 会导致全局的axios发出的请求或接收的响应都会被拦截到， 所以应该使用axios.create() 来创建单独的axios实例。

// 创建axios实例
let instance = axios.create({
	baseURL: process.env.VUE_APP_END_POINT
});

// Add a request interceptor
instance.interceptors.request.use(config => {
	if (store.state.token && !checkWhitelist(config.url)) {
		config.headers.Authorization = `Bearer ${store.state.token}`
	}
	if (!checkWhitelist(config.url)) {
		showFullScreenLoading()
	}
	return config
}, (error) => {
	tryHideFullScreenLoading()
	vm.$toast('request error')
	return Promise.reject(error)
})

// Add a response interceptor
instance.interceptors.response.use(response => {
	if (!response.config.hideLoading) {
		tryHideFullScreenLoading()
	}
	return response.data
}, error => {
	tryHideFullScreenLoading()
	if (error && error.response) {
		switch (error.response.status) {
		case 400:
			error.message = '错误请求'
			break
		case 401:
			error.message = '未授权，请重新登录'
			break
		case 403:
			error.message = '拒绝访问'
			break
		case 404:
			error.message = '请求错误,未找到该资源'
			break
		case 405:
			error.message = '请求方法未允许'
			break
		case 408:
			error.message = '请求超时'
			break
		case 500:
			error.message = '服务器端出错'
			break
		case 501:
			error.message = '网络未实现'
			break
		case 502:
			error.message = '网络错误'
			break
		case 503:
			error.message = '服务不可用'
			break
		case 504:
			error.message = '网络超时'
			break
		case 505:
			error.message = 'http版本不支持该请求'
			break
		default:
			error.message = `连接错误${error.response.status}`
		}
		let errorData = {
			status: error.response.status,
			message: error.message,
			config: error.response.config
		}
		console.log(errorData, 'error msg-----------')
		vm.$toast(errorData.message)
	} else {
		vm.$toast('请求出错,请刷新页面重试')
	}
	return Promise.reject(error)
})

export default instance
// export default {
// 	get: (url, config) => instance.get(url, { ...defaultConfig,
// 		...config
// 	}),
// 	put: (url, data, config) => instance.put(url, data, { ...defaultConfig,
// 		...config
// 	}),
// 	post: (url, data, config) => instance.post(url, data, { ...defaultConfig,
// 		...config
// 	}),
// 	patch: (url, data, config) => instance.patch(url, data, { ...defaultConfig,
// 		...config
// 	}),
// 	delete: (url, data, config) => instance.delete(url, { ...defaultConfig,
// 		...config
// 	})
// }
