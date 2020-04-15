import moment from 'moment'

export function money(amount) {
	if (amount) {
		const newAmount = amount > 10 ** 8 ? Number(amount / 10 ** 15) : Number(amount)
		return newAmount.toFixed(2) + ' PRM'
	} else {
		return '0 PRM'
	}
}

export function person(num) {
	if (num) {
		return Math.floor(num) + '人'
	} else {
		return '0人'
	}
}

export function clip(address, start = 14, end = -6) {
	if (address) {
		return `${address.slice(0, start)}...${address.slice(end)}`
	}
	return address
}

export function date(date) {
	return moment.utc(date).local().format('YYYY-MM-DD HH:mm:ss')
}
