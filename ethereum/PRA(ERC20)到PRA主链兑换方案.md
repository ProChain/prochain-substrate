### PRA(ERC20) 到 PRA主链兑换方案（单向原子交换）
---

#### 1. Eth合约 (代码见 https://github.com/ProChain/prochain-substrate/tree/v2.0/ethereum/erc20-htlc-swap/contracts )

- 部署HTLC合约
- 函数：htlc(), claim(), refund()
- 事件：HTLC, Claimed, Refunded
- HTLC状态：INVALID, OPEN, COMPLETED, EXPIRED

#### 2. PRA主链

- 通过off-chain worker监听eth event，聚合
- 函数：htlc(), claim(), refund()
- 事件：HTLC, Claimed, Refunded
- HTLC状态：INVALID, OPEN, COMPLETED, EXPIRED

#### 3. deputy监控

- 中心化服务
- 监控合约状态，锁仓地址余额
- 清算对账，不一致则告警
- 查询失败的swap，告警
- 一键pause合约

#### HTLC流程

##### 初始化
- 部署HTLC合约
- 准备eth地址acc1，作为兑换发起人
- erc20资产锁仓地址acc2，初始余额为0，公开作为peg资产
- 准备PRA主链收款DID: acc3，作为兑换收款人
- 主链资产锁仓DID: acc4，要求有足够余额，公开作为peg资产

##### 流程
- eth用户acc1，向Pro-ERC20合约调用approve()，授权合约可扣款金额
- eth用户acc1，调用HTLC合约函数htlc()，参数randomNumberHash, timestamp, heightSpan, praReceiverAddr, erc20Amount, praAmount等, 发起HTLC
- HTLC合约向用户acc1账号扣款erc20Amount，存在合约账号内
- PRA主链通过off-chain worker监听eth，接收新的HTLC Event
- PRA主链创建新的HTLC，两边的SwapID对应相同
- eth用户acc1，调用函数claim()，参数swapID和randomNumber，声明swap，资产由合约账号转向锁仓地址acc2
- PRA主链通过off-chain worker监听eth，接收Claimed Event，从acc4账号向收款人acc3付款主链币
- 若HTLC超时，任何eth用户可以调用refund()结束交易，参数swapID，资产由合约账号退回用户acc1

##### 配置项
- erc20 PRA和主链PRA汇率为 1:1
- heightSpan高度跨度

##### 优点
- 合约跨链，无中心化信任节点

##### 潜在风险
- acc4余额不足
- off-chain worker取数据出错
- 两边对账不一致

#### 用户入口
- 钱包DAPP（需要支持erc20）

#### 测试物料和样例

##### Ropsten测试网
- 测试网API入口 https://ropsten.infura.io/v3/32d3935c7ba0400d97a7d8f983753a34
- UserA: 0xf7FeA1722F9b27B0666919A5664BaB486a4b18D3
- UserB: 0xCF5bECb7245E2e6eE2E092F0BD63F6Bd79eF19Fe
- `ProToken`合约地址: 0xd2bc5bf7563c6d308ecb36f46f9848bb054223d1
- `ERC20HTLC`合约地址: 0x49e532fa0d95195d6a07be152e481c67715149eb
- PRA DEV地址（receiver）: 5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL

#### 根据randomNumber和timestamp，计算randomNumberHash和swapID
- 使用当前目录下的calculate-ids模块
- npm install
- 在calculate-ids.js写入randomNumber, receiver
- 执行node calculate-ids.js得到randomNumberHash, swapID

#### 扩展
- 可用于erc20-erc20的原子交换
- 可用于PRA主链token间的原子交换
