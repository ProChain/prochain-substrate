<template>
	<div class="htlc-component">
		<van-row>
			<van-col span="24">
				<div class="htlc" v-if="!history">
					<ValidationObserver v-slot="{ invalid }">
						<van-cell-group title="数量" :border="false">
							<ValidationProvider v-slot="{ errors }" name="amount" rules="required|min_value:0.01">
								<van-field v-model="htlcForm.amount" type="number" :error-message="errors[0]" placeholder="请输入您想要兑换的PRA数量" />
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
				<van-panel v-else title="兑换状态" :status="status" desc="您的PRA兑换状态在这里显示">
					<div slot="footer" class="btns">
						<van-button type="primary" square class="btn-submit" size="small" @click="handleClaim">确认兑换</van-button>
						<van-button type="primary" square class="btn-submit" size="small" @click="handleRefund">撤销兑换</van-button>
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
	export default {
		name: 'htlc',
		data() {
			return {
				htlcForm: {
					amount: null,
					did: '',
					randomNum: ''
				},
				showRandom: false,
				status: '进行中',
				history: null,
				timer: null
			}
		},
		components: {
			ValidationProvider,
			ValidationObserver
		},
		async mounted() {
			this.htlcForm.randomNum = '0x' + generateMixed(64)

			await App.init()
			this.checkSwap()
		},
		methods: {
			approve() {
				const amount = 500000 * 1000000000000000000
				App.praIntance.approve(App.contracts.htlcContract.address, amount).then(
					function(result) {
						if (result.receipt.status == 1) {
							console.log('status success!!')
						} else {
							console.log('status fail!!')
						}
					}
				).catch(function(err) {
					console.log(err.message)
				})
			},
			checkSwap() {
				const history = localStorage.getItem('history')
				if (history) {
					this.history = JSON.parse(history)
					this.checkTransactionStatus(this.history.tx)
				} else {
					this.approve()
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

				App.htlcIntance.htlc(randomNumberHash, timestamp, App.heightSpan, App.recipientAddr, amount, amount, did).then(result => {
					if (result.receipt.status == 1) {
						console.log('status success!!')
						this.checkTransactionStatus(result.tx)
						this.history = {
							...this.htlcForm,
							swapId,
							tx
						}
						localStorage.setItem('history', JSON.stringify(this.history))
					} else {
						console.log('status fail!!')
					}
				}).catch(function(err) {
					console.log(err.message)
				})
			},
			onConfirm(value) {
				this.htlcForm.type = value
				this.showPicker = false
			},
			handleClaim() {
				const { randomNum } = this.history
				if (App.swapID == null || App.swapID === '') {
					return this.$toast('提示 Swap Id 无效，请先发起一笔htlc兑换')
				}

				console.log('App.swapID:', App.swapID)

				App.htlcIntance.claim(App.swapID, randomNum).then(result => {
					if (result.receipt.status == 1) {
						console.log('status success!!')
						this.status = 'Waiting to receive PRA'
					} else {
						console.log('status fail!!')
					}
				}).catch(function(err) {
					console.log(err.message)
				})
			},
			handleRefund() {
				const { swapID } = this.history
				if (swapID == null || swapID === '') {
					return this.$toast('提示 Swap Id 无效，请先发起一笔htlc兑换')
				}

				App.htlcIntance.refund(swapID).then(result => {
					if (result.receipt.status == 1) {
						console.log('status success!!')
					} else {
						console.log('status fail!!')
					}
				}).catch(function(err) {
					console.log(err.message)
				})
			},
			checkTransactionStatus(hash) {
				this.timer = setInterval(async() => {
					const { result } = await getTransactionByHash(hash)
					console.log(result.blockNumber, '---data---')
					if (result.blockNumber) {
						this.status = 'Waiting to claim'
						clearInterval(this.timer)
					}
				}, 3000)
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
		.request {
			margin-top: $largeGutter * 2;
		}

		.van-panel {
			.btns {
				text-align: right;
				.van-button {
					margin-left: $mediumGutter;
				}
			}
		}
	}
</style>
