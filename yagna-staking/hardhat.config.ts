import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";
import * as dotenv from "dotenv";

// 加载 .env 文件中的环境变量
dotenv.config();

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.28",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    // 修复错误 2322: 不能为空对象，需指定 type
    hardhat: {
      // @ts-ignore: 这里的 type 取决于你安装的具体插件要求，通常为 edr-simulated
      type: "edr-simulated", 
    },
    // 修复错误 2322: 必须包含 type: "http"
    amoy: {
      // @ts-ignore: 显式声明类型以符合你的环境校验
      type: "http",
      url: process.env.AMOY_RPC_URL || "https://rpc-amoy.polygon.technology/",
      accounts: process.env.AMOY_PRIVATE_KEY ? [process.env.AMOY_PRIVATE_KEY] : [],
    },
  },
};

export default config;