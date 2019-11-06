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
- HTLC状态：INVALID, OPEN, COMPLETED, EXPIRED

#### 3. Event数据聚合

- 中心化服务器
- 访问N个主链数据源，取n/2+1个正确结果，存数据库
- 向off-chain worker提供服务
- 若时间段内新数据数过低，告警
- 若时间段内查询数过低，告警

#### 4. admin监控

- 中心化服务器
- 监控合约状态，锁仓地址余额
- 清算对账，不一致则告警
- 查询失败的swap，告警
- 一键pause合约

#### HTLC流程

##### 初始化
- 准备HTLC合约地址acc1
- erc20资产锁仓地址acc2，公开
- 准备PRA主链收款地址acc3
- 主链资产锁仓地址acc4，要求有足够余额，公开作为peg资产

##### 流程
- eth用户向合约地址acc1转账ERC20
- eth用户调用函数htlc()，参数randomNumberHash, timestamp, heightSpan, praReceiverAddr, erc20Amount, praAmount等, 发起HTLC
- Deputy通过off-chain worker监听eth，接收新的HTLC Event
- Deputy在PRA主链创建新的HTLC，两边的swapID对应相同
- 主链用户acc3，调用Deputy合约的claim()，参数swapID和randomNumber等，声明swap所属权
- 任何eth用户可以调用函数claim()，参数swapID和randomNumber，确认完成swap，资产由acc1转向锁仓地址acc2
- 超时后，任何eth用户可以调用refund()结束交易，参数swapID，资产由acc2转回用户acc1
- Deputy通过off-chain worker监听eth，接收Claimed Event，资产由acc4转向主链用户acc3
- 失败处理：交易超时，主链用户未在指定heightSpan出块范围内claim

##### 配置项
- erc20 PRA和主链PRA汇率为 1:1
- heightSpan高度跨度
- 过期时间跨度

##### 优点
- 合约跨链，无中心化信任节点

##### 潜在风险
- acc4余额不足
- off-chain worker取数据出错
- 两边对账不一致

#### 用户入口
- 钱包DAPP
- PRA主链SDK

#### 测试物料和样例
待补充

#### 扩展
- 用于erc20-erc20的原子交换
- 用户PRA主链token间的原子交换
