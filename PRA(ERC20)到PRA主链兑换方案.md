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

- eth用户向合约转账ERC20
- eth用户调用函数htlc()，参数randomNumberHash, timestamp, heightSpan, praReceiverAddr, erc20Amount, praAmount等, 发起HTLC
- Deputy通过off-chain worker监听eth，接收新的HTLC Event
- Deputy在PRA主链创建新的HTLC，通过swapID对应
- 主链用户调用Deputy合约的claim()，参数必须一致，声明所属权
- eth用户调用函数claim()，参数randomNumber，确认完成交易
- eth用户可以随时调用refund()结束交易
- 失败处理：交易超时，主链用户未在指定heightSpan出块范围内claim，eth用户可调用refund()结束交易

#### 用户入口
钱包DAPP

#### 测试物料
待补充
