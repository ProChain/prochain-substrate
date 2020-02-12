import FastClick from 'fastclick'
import App from '@/App'
import router from '@/router'
import store from '@/vuex'
import i18n from '@/i18n'
import 'amfe-flexible'
import '@/assets/css/style.customize.scss'
import '@/assets/css/reset.css'
import * as filters from '@/filters'

import '@vant/touch-emulator'
import './vee-validate'

Vue.config.productionTip = false

FastClick.prototype.onTouchEnd = function(event) {
	if (event.target.hasAttribute('type') && event.target.getAttribute('type') === 'text') {
		event.preventDefault()
		event.target.focus()
		return false
	}
}
FastClick.attach(document.body)

Object.keys(filters).forEach(k => Vue.filter(k, filters[k]))

export default new Vue({
	el: '#app',
	i18n,
	router,
	store,
	components: { App },
	template: '<App/>'
})
