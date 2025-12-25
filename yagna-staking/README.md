# 智能合约说明文档

## 1. 核心合约功能

合约 `StakingManager.sol` 实现了以下四个核心业务流程：

| 函数名称 | 调用者 | 描述 |
| :--- | :--- | :--- |
| **`deposit()`** | 房东 (前端) | 质押 MATIC 激活算力节点，最小质押额为 **0.01 MATIC**。 |
| **`withdraw()`** | 房东 (前端) | 提取所有质押本金和任务收益。提取后 `isRegistered` 状态失效。 |
| **`slash()`** | 管理员 (后端) | **阶梯惩罚机制**：当检测到节点掉线时，根据 `slashCount` 自动增加惩罚比例，惩罚金转入 Admin 账户。 |
| **`addReward()`** | 管理员 (后端) | 任务结算时，由后端调度器将任务报酬发放至合约中该房东的奖励余额中。 |
| **`getStatus()`** | 所有人 | 查询指定地址的质押金额、待领奖励及累计被惩罚次数。 |

---

## 2. 合约部署指南

### 环境准备
1. **安装依赖**：`npm install`。
2. **安全配置**：在根目录创建 `.env` 文件：
```bash
AMOY_PRIVATE_KEY=你的私钥
```

### 部署步骤
在终端执行以下命令：

```bash
# 1. 编译合约 (生成最新的 ABI 和字节码)
npx hardhat compile

# 2. 部署到 Amoy 测试网
npx hardhat run scripts/deploy.ts --network amoy
```

---

### 3. 前端交接说明
更新前端的配置：

修改前端调用服务中的合约地址，改为上方部署后的控制台输出地址:

`src/util/staking.ts`

```typescript
const CONTRACT_ADDRESS = "0x合约地址";
```