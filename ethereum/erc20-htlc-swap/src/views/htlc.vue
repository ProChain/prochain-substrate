<template>
	<div class="htlc-component">
		<van-row>
			<van-col span="24">
				<div class="htlc" v-if="status !== 1">
					<ValidationObserver v-slot="{ invalid }" ref="form">
						<van-cell-group title="数量" :border="false">
							<ValidationProvider v-slot="{ errors }" name="amount" rules="required|min_value:0.01">
								<van-field v-model="htlcForm.amount" type="number" :error-message="errors[0]" placeholder="请输入您想要兑换的PRA数量" />
								<van-cell title="ERC20 PRA余额" :value="balance" />
							</ValidationProvider>
						</van-cell-group>
						<van-cell-group v-if="showRandom" title="随机数" :border="false">
							<ValidationProvider v-slot="{ errors }" name="randomNum" rules="required">
								<van-field v-model="htlcForm.randomNum" :error-message="errors[0]" placeholder="请输入随机数" />
							</ValidationProvider>
						</van-cell-group>
						<van-cell-group title="收款账号" :border="false">
							<ValidationProvider v-slot="{ errors }" name="did" rules="required">
								<van-field v-model="htlcForm.did" :error-message="errors[0]" placeholder="请输入您的DID" />
							</ValidationProvider>
						</van-cell-group>
						<van-button type="primary" square class="btn-submit request" size="large" :disabled="invalid" @click="handleSubmit">发起兑换</van-button>
					</ValidationObserver>
				</div>
				<van-panel v-else title="兑换状态" :status="statusText">
					<van-loading v-if="status === 0">等待进行下一步操作...</van-loading>
					<span v-else-if="status === 1">请点击“确认兑换”按钮完成兑换</span>
					<span v-else>等待收款</span>
					<div slot="footer" class="btns">
						<van-button type="primary" square class="btn-submit" size="small" :disabled="status !== 1" @click="handleClaim">确认兑换</van-button>
						<van-button type="primary" square class="btn-submit" size="small" :disabled="status === 2" @click="handleRefund">撤销兑换</van-button>
					</div>
				</van-panel>
				<van-popup v-model="showPicker" position="bottom">
					<van-picker show-toolbar :columns="columns" @cancel="showPicker = false" @confirm="onConfirm" />
				</van-popup>
			</van-col>
		</van-row>
	</div>
</template>
<script>
	import { ValidationObserver, ValidationProvider } from 'vee-validate'
	import { generateMixed } from '@/util/common'
	import { getRandomNumberHash, getTransactionByHash } from '@/util/api'
	import App from '@/util/app'
	const initiaState = {
		amount: null,
		did: '',
		randomNum: ''
	}
	export default {
		name: 'htlc',
		data() {
			return {
				htlcForm: { ...initiaState },
				showRandom: false,
				status: 0, // 0 未开始, 1 htlc, 2 claimed
				history: {},
				timer: null,
				balance: 0
			}
		},
		components: {
			ValidationProvider,
			ValidationObserver
		},
		computed: {
			statusText() {
				let text = '未开始'
				switch (this.status) {
				case 1:
					text = '等待确认兑换'
					break
				case 2:
					text = '兑换中'
					break
				case 3:
					text = '已撤销兑换'
					break
				}
				return text
			}
		},
		async mounted() {
			this.htlcForm.randomNum = '0x' + generateMixed(64)

			await App.init()
			await this.checkSwap()
			// get pra balance
			App.praIntance.balanceOf(App.account).then(balance => {
				this.balance = (balance.toNumber() / 10 ** 18).toFixed(3)
			})
		},
		methods: {
			approve() {
				const amount = 500000 * 1000000000000000000
				App.praIntance.approve(App.contracts.htlcContract.address, amount).then(result => {
					if (result.receipt.status == 1) {
						console.log('status success!!')
					} else {
						console.log('status fail!!')
					}
				}).catch(function(err) {
					console.log(err.message)
				})
			},
			async checkSwap() {
				const history = localStorage.getItem('history')
				if (history) {
					this.history = JSON.parse(history)
					this.checkTransactionStatus(this.history.tx)
				} else {
					const allowance = await App.praIntance.allowance(App.account, App.contracts.htlcContract.address)
					if (allowance <= 0) this.approve()
				}
			},
			async handleSubmit() {
				let amount = parseFloat(this.htlcForm.amount)
				amount = amount * 1000000000000000000

				const { randomNum, did } = this.htlcForm
				const data = await getRandomNumberHash(randomNum, did, amount)
				console.log(data, 'data-----')
				if (data == null || data == 'Error' || data.code != 0) {
					return
				}

				const { data: { timestamp, randomNumberHash, swapID } } = data
				App.swapID = swapID
				this.$notify({
					message: '正在为您发起兑换，可能会花几分钟时间，请不要关闭页面',
					duration: 0
				})
				this.$store.commit('showLoading')
				try {
					const result = await App.htlcIntance.htlc(randomNumberHash, timestamp, App.heightSpan, App.recipientAddr, amount, amount, did)
					this.$store.commit('hideLoading')
					this.$notify.clear()
					if (result.receipt.status == 1) {
						console.log('status success!!')
						this.history = {
							...this.htlcForm,
							swapID,
							tx: result.tx,
							status: 1
						}
						this.status = 1
						localStorage.setItem('history', JSON.stringify(this.history))
						this.htlcForm = { ...initiaState }
						this.timer = setInterval(() => {
							this.checkTransactionStatus(result.tx)
						}, 2000)
					}
				} catch (e) {
					this.$store.commit('hideLoading')
					console.log(e)
				}
			},
			async handleClaim() {
				const { randomNum, swapID } = this.history
				console.log(swapID, App.swapID, '---swap')
				if (swapID == null || swapID === '') {
					return this.$toast('提示 Swap Id 无效，请先发起一笔htlc兑换')
				}

				console.log('App swapID:', swapID)
				this.$notify({
					message: '正在确认兑换，请不要关闭页面',
					duration: 0
				})
				this.$store.commit('showLoading')
				try {
					const result = await App.htlcIntance.claim(swapID, randomNum)
					this.$store.commit('hideLoading')
					this.$notify.clear()
					if (result.receipt.status == 1) {
						console.log('status success!!')
						this.$toast('已确认兑换，等待钱包收款')
						// remove swap history
						this.status = 2
						this.history = {}
						localStorage.removeItem('history')
					}
				} catch (e) {
					this.$store.commit('hideLoading')
					console.log(e)
				}
			},
			async handleRefund() {
				const { swapID } = this.history
				if (swapID == null || swapID === '') {
					return this.$toast('提示 Swap Id 无效，请先发起一笔htlc兑换')
				}

				this.$notify({
					message: '正在撤销兑换，请不要关闭页面',
					duration: 0
				})
				this.$store.commit('showLoading')
				try {
					const result = await App.htlcIntance.refund(swapID)
					this.$store.commit('hideLoading')
					this.$notify.clear()
					if (result.receipt.status == 1) {
						console.log('status success!!')
						// remove swap history
						this.status = 0
						this.history = {}
						localStorage.removeItem('history')
					}
				} catch  (e) {
					this.$store.commit('hideLoading')
					console.log(e)
				}
			},
			async checkTransactionStatus(hash) {
				const { result } = await getTransactionByHash(hash)
				console.log(result.blockNumber, '---data---')
				if (result.blockNumber) {
					this.status = 1
					clearInterval(this.timer)
				}
			}
		},
		destroyed() {
			clearInterval(this.timer)
		}
	}
</script>
<style lang="scss">
	@import '../assets/css/variables.scss';

	.htlc-component {
		font-size: $smallFontSize;
		.request {
			margin-top: $largeGutter * 2;
		}

		.van-panel {
			.van-panel__content {
				padding: $largeGutter $mediumGutter;
			}
			.btns {
				text-align: right;

				.van-button {
					margin-left: $mediumGutter;
				}
			}
		}
	}
</style>
