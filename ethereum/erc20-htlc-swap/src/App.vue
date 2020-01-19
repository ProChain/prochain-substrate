<template>
	<div id="app">
		<Loading v-if="showLoading"></Loading>
		<transition v-on:after-enter="afterEnter" :name="transitionName">
			<keep-alive :exclude="shouldNotKeepAlive">
				<router-view class="router-view" />
			</keep-alive>
		</transition>
	</div>
</template>
<script>
	import { mapState } from 'vuex'
	import Loading from './components/loading'
	export default {
		name: 'App',
		data() {
			const { options: { routes } } = this.$router
			let excludedComponents = [];

			routes.forEach(v => v.meta && !v.meta.keepAlive && excludedComponents.push(v.name))

			return {
				transitionName: '',
				shouldNotKeepAlive: excludedComponents
			}
		},
		components: {
			Loading
		},
		computed: {
			...mapState([
				'showLoading'
			])
		},
		watch: {
			$route(to, from) {
				// 如果to索引大于from索引,判断为前进状态,反之则为后退状态
				if (to.meta.index > from.meta.index) {
					this.transitionName = 'vux-pop-in'
				} else if (to.meta.index < from.meta.index) {
					this.transitionName = 'vux-pop-out'
				} else {
					this.transitionName = 'vux-none'
				}
			}
		},
		methods: {
			afterEnter(el) {
				if (el.classList.contains('loading')) {
					this.transitionName = 'vux-none'
				}
			}
		}
	}
</script>
<style lang="scss">
	#app,
	html,
	body {
		width: 100%;
		height: 100%;
		position: absolute;
		top: 0;
		left: 0;
		overflow: hidden;
	}

	.router-view {
		width: 100%;
		height: 100%;
		position: absolute;
		transition: all .3s cubic-bezier(.55, 0, .1, 1);
		top: 0;
		bottom: 0;
		margin: 0 auto;
		overflow-y: auto;
		overflow-x: hidden;
		background: #f8f8f8;
		-webkit-overflow-scrolling: touch;

		&.loading {
			transition: none;
		}
	}

	.vux-pop-out-enter-active,
	.vux-pop-out-leave-active,
	.vux-pop-in-enter-active,
	.vux-pop-in-leave-active {
		will-change: transform;
		transition: all 300ms;
		height: 100%;
		position: absolute;
		backface-visibility: hidden;
		perspective: 1000;
	}

	.vux-pop-out-enter {
		opacity: 0;
		/* transform: translate3d(-100%, 0, 0); */
		transform: translate3d(0%, 0, 0);
	}

	.vux-pop-out-leave-active {
		opacity: 0;
		transform: translate3d(100%, 0, 0);
	}

	.vux-pop-in-enter {
		opacity: 0;
		transform: translate3d(100%, 0, 0);
	}

	.vux-pop-in-leave-active {
		opacity: 0;
		/* transform: translate3d(-100%, 0, 0); */
		transform: translate3d(0%, 0, 0);
	}

	/* no transition*/
	.vux-none-enter-active,
	.vux-none-leave-active {
		transition: none;
	}

	.vux-none-enter,
	.vux-none-leave-active {
		opacity: 1;
	}
</style>
