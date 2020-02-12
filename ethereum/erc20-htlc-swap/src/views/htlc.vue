<template>
	<div class="htlc-component">
		<van-row>
			<van-col span="24">
				<div class="htlc">
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
				<van-panel v-if="history.tx" title="兑换状态" desc="需等待18个区块确认" :status="statusText">
					<div>数量：<span>{{ history.amount }} </span>PRA</div>
					<div>DID：<span>{{ history.did | clip(18, -10) }}</span></div>
					<div>区块确认：<span>{{ confirmations }} </span>个</div>
				</van-panel>
			</van-col>
		</van-row>
	</div>
</template>
<script>
	import { ValidationObserver, ValidationProvider } from 'vee-validate'
	import { generateMixed } from '@/util/common'
	import { getRandomNumberHash, getTransactionByHash, getBlockNumber } from '@/util/api'
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
				history: {
					status: 0 // 0 pending, 1 success, 2 complete
				},
				timer: null,
				timer1: null,
				balance: 0,
				txHash: '',
				currentBlocknumber: null,
				txBlocknumber: null
			}
		},
		components: {
			ValidationProvider,
			ValidationObserver
		},
		computed: {
			statusText() {
				let text = ''
				switch (this.history.status) {
				case 0:
					text = 'Pending'
					break
				case 1:
					text = 'Success'
					break
				case 2:
					text = 'Complete'
					break
				}
				return text
			},
			confirmations() {
				if (this.currentBlocknumber > 0 && this.txBlocknumber > 0 && this.currentBlocknumber >= this.txBlocknumber) {
					return this.currentBlocknumber - this.txBlocknumber
				}
				return 0
			}
		},
		async mounted() {
			this.htlcForm.randomNum = '0x' + generateMixed(64)
			try {
				await App.init()
				await this.checkSwap()
			} catch (e) {
				console.log(e)
			}
			// get pra balance
			App.praIntance.balanceOf(App.account).then(balance => {
				this.balance = (balance.toNumber() / 10 ** 18).toFixed(3)
			})
			this.updateConfirmations()
			this.timer1 = setInterval(this.updateConfirmations, 10000)
		},
		methods: {
			approve() {
				this.$dialog.confirm({
					title: '温馨提示',
					message: '首次兑换需要获得您的授权，该操作不会扣除您的ETH，只会消耗少量gas费',
					messageAlign: 'left'
				}).then(() => {
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
				}).catch(console.log)
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
					return this.$toast('提示 http 500 错误，请重新发起兑换')
				}

				const { data: { timestamp, randomNumberHash, swapID } } = data
				App.swapID = swapID
				this.$notify({
					message: '正在为您发起兑换，可能会花几分钟时间，请不要关闭页面',
					duration: 0
				})
				this.$store.commit('showLoading')
				try {
					const result = await App.htlcIntance.htlc(randomNumberHash, timestamp, App.heightSpan, amount, amount, did)
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
						localStorage.setItem('history', JSON.stringify(this.history))
						// reset form
						this.htlcForm = { ...initiaState }
						this.$refs.form.reset()
						// check transaction
						this.checkTransactionStatus(result.tx)
					}
				} catch (e) {
					this.$store.commit('hideLoading')
					this.$notify.clear()
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
					this.$notify.clear()
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
					this.$notify.clear()
					console.log(e)
				}
			},
			checkTransactionStatus(hash) {
				clearInterval(this.timer)
				this.timer = setInterval(async() => {
					const { result } = await getTransactionByHash(hash)
					if (result.blockNumber) {
						this.txBlocknumber = web3.toDecimal(result.blockNumber)
						this.status = 2
						this.history.block
						clearInterval(this.timer)
					}
				}, 1000)
			},
			async updateConfirmations() {
				const { result } = await getBlockNumber()
				this.currentBlocknumber = web3.toDecimal(result)
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
			margin-top: $largeGutter*2;
			.van-panel__content {
				font-size: $mediumFontSize;
				padding: $mediumGutter $largeGutter $largeGutter;
				line-height: $largeGutter*1.3;
				div {
					span {
						color: #3498db;
						font-weight: bold;
					}
				}
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
