### PRA(ERC20) 到 PRA主链兑换方案（单向）
---

#### 1. Eth合约

- 部署HTLC合约
- 函数：htlc(), claim(), refund()
- 事件：HTLC, Claimed, Refunded
- HTLC状态：INVALID, OPEN, COMPLETED, EXPIRED

#### 2. PRA主链合约

- 部署Deputy合约
- 通过off-chain worker监听eth event，聚合
- 函数：claim(), refund()
- 事件：HTLC, Claimed, Refunded

#### 3. Event数据聚合

- 中心化服务器
- 访问N个主链数据源，取n/2+1个正确结果，存数据库
- 向off-chain worker提供服务

#### HTLC流程

##### 初始化
- 准备HTLC合约地址acc1
- erc20资产锁仓地址acc2
- 准备PRA主链收款地址acc3
- 主链资产锁仓地址acc4，要求有余额

##### 流程
- eth用户向合约地址acc1转账ERC20
- eth用户调用函数htlc()，参数randomNumberHash, timestamp, heightSpan, praReceiverAddr, erc20Amount, praAmount等, 发起HTLC
- Deputy通过off-chain worker监听eth，接收新的HTLC Event
- Deputy在PRA主链创建新的HTLC，通过swapID对应
- 主链用户acc3，调用Deputy合约的claim()，参数必须一致，声明所属权
- 任何eth用户可以调用函数claim()，参数swapID和randomNumber，确认完成交易，资产由acc1转向锁仓地址acc2
- 任何eth用户可以调用refund()，参数swapID，随时结束交易，资产由acc2转回用户acc1
- Deputy通过off-chain worker监听eth，接收Claimed Event，资产由acc4转向主链用户acc3
- 失败处理：交易超时，主链用户未在指定heightSpan出块范围内claim，任意eth用户可调用refund()结束交易

##### 潜在风险
- refund()参数太少，可能被恶意利用
- acc4余额不足

#### 用户入口
钱包DAPP

#### 测试物料
待补充
