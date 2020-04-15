import { u8aToHex, isHex } from '@polkadot/util'
import bs58 from 'bs58'
import { blake2AsHex } from '@polkadot/util-crypto'

export function getLanguage() {
	const language = (navigator.language || navigator.browserLanguage).toLowerCase()
	let locale
	if (language.indexOf('en') > -1) {
		locale = 'en'
	} else {
		locale = 'zh'
	}
	return locale
}

export function urlParse() {
	const obj = {};
	let keyValue = [];
	let key = '';
	let value = '';
	const url = window.location.href
	var paraString = url.substring(url.indexOf('?') + 1, url.length).split('&')
	for (var i in paraString) {
		keyValue = paraString[i].split('=')
		key = keyValue[0]
		value = keyValue[1]
		obj[key] = value
	}
	return obj
}

export function getRect(el) {
	if (el instanceof window.SVGElement) {
		let rect = el.getBoundingClientRect()
		return {
			top: rect.top,
			left: rect.left,
			width: rect.width,
			height: rect.height
		};
	} else {
		return {
			top: el.offsetTop,
			left: el.offsetLeft,
			width: el.offsetWidth,
			height: el.offsetHeight
		};
	}
}

export async function sleep(timeout = 200) {
	await new Promise(resolve => {
		setTimeout(() => {
			resolve();
		}, timeout)
	});
}

export function checkDeviceType() {
	const ua = navigator.userAgent
	let isMobile = false
	if ((ua.match(/(phone|pad|pod|iPhone|iPod|ios|iPad|Android|Mobile|BlackBerry|IEMobile|MQQBrowser|JUC|Fennec|wOSBrowser|BrowserNG|WebOS|Symbian|Windows Phone)/i))) {
		isMobile = true
	} else {
		isMobile = false
	}
	return isMobile
}

export function isWeixin() {
	var ua = navigator.userAgent.toLowerCase();
	var isWeixin = ua.indexOf('micromessenger') !== -1
	if (isWeixin) {
		return true
	} else {
		return false
	}
}

/**
 * 时间秒数格式化
 * @param timestamp 时间戳（单位：秒）
 * @returns {*} 格式化后的时分秒
 */
export function formatSeconds(timestamp, showSecond = false) {
	let result = ''
	let days, hours, minutes, seconds
	if (timestamp >= 86400) {
		days = Math.floor(timestamp / 86400)
		timestamp = timestamp % 86400
		result = days + '天';
		if (timestamp > 0) {
			result += ''
		}
	}
	if (timestamp >= 3600) {
		hours = Math.floor(timestamp / 3600)
		timestamp = timestamp % 3600
		if (hours < 10) {
			hours = '0' + hours
		}
		result += hours + '小时'
	}
	if (timestamp >= 60) {
		minutes = Math.floor(timestamp / 60)
		timestamp = timestamp % 60
		if (minutes < 10) {
			minutes = '0' + minutes
		}
		result += minutes + '分'
	}
	if (showSecond) {
		seconds = Math.floor(timestamp)
		if (seconds < 10) {
			seconds = '0' + seconds
		}
		result += seconds + '秒'
	}
	return result
}

export function generateMixed(n) {
	var chars = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f']
	var res = ''
	for (var i = 0; i < n; i++) {
		var id = Math.ceil(Math.random() * 15)
		res += chars[id]
	}
	return res
}

export function formatNumber(num) {
	return parseInt(num * 10 ** 15)
}

export function formatHexNumber(hexNum) {
	if (isHex(hexNum)) {
		return hexNum.toString() / 10 ** 15
	} else if (typeof hexNum === 'number') {
		return hexNum / 10 ** 15
	}
	return 0
}

export function getObjectURL(file) {
	let url = null
	if (window.createObjectURL !== undefined) {
		// basic
		url = window.createObjectURL(file)
	} else if (window.URL !== undefined) {
		// mozilla(firefox)
		url = window.URL.createObjectURL(file)
	} else if (window.webkitURL !== undefined) {
		// webkit or chrome
		url = window.webkitURL.createObjectURL(file)
	}
	return url
}

export function didToHex(did) {
	const bytes = bs58.decode(did.substring(8))
	return blake2AsHex(bytes, 256)
}

export function hexToDid(hex) {
	let did
	if (isHex(hex)) {
		const bytes = Buffer.from(hex.slice(2), 'hex')
		const address = bs58.encode(bytes)
		did = `did:prm:${address}`
	} else {
		const hexStr = u8aToHex(hex)
		const bytes = Buffer.from(hexStr.slice(2), 'hex')
		const address = bs58.encode(bytes)
		did = `did:prm:${address}`
	}
	return did
}

export function debounce(fn, delay) {
	delay = delay || 600
	let timer
	return function() {
		let ctx = this
		let args = arguments
		if (timer) {
			clearTimeout(timer)
		}
		timer = setTimeout(() => {
			timer = null
			fn.apply(ctx, args)
		}, delay)
	}
}

export function throttle(fn, interval) {
	let last
	let timer
	interval = interval || 600
	return function() {
		let ctx = this
		let args = arguments
		let now = new Date()
		if (last && now - last < interval) {
			clearTimeout(timer)
			timer = setTimeout(function() {
				last = now
				fn.apply(ctx, args)
			}, interval)
		} else {
			last = now
			fn.apply(ctx, args)
		}
	}
}
