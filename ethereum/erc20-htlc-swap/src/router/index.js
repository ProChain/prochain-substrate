import store from '@/vuex';
import i18n from '@/i18n';
import ErrorComponent from '@/components/error'

const lazyLoadView = AsyncView => {
	const AsyncHandler = () => ({
		component: AsyncView,
		error: ErrorComponent,
		delay: 200,
		timeout: 2000
	})

	return Promise.resolve({
		functional: true,
		render(h, { data, children }) {
			return h(AsyncHandler, data, children)
		}
	})
}

const router = new VueRouter({
	routes: [
		{
		      path: '/',
		      redirect: '/htlc'
		},
		{
			path: '/htlc',
			name: 'htlc',
			component: () => lazyLoadView(import('@/views/htlc')),
			meta: {
				index: 3,
				title: 'meta.htlc',
				keepAlive: true,
				requireAuth: false
			}
		}
	]
})

router.beforeEach(async(to, from, next) => {
	window.document.title = i18n.t(to.meta.title) || 'Task'
	if (to.matched.some(r => r.meta.requireAuth)) {
		if (store.state.token) {
			next()
		} else {
			next({
				name: 'home'
			})
		}
	} else {
		next()
	}
})

export default router
