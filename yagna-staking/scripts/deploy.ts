import { ethers } from "hardhat";

async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with the account:", deployer.address);

  const Staking = await ethers.getContractFactory("StakingManager");
  // 部署
  const staking = await Staking.deploy();

  // 等待部署完成
  await staking.waitForDeployment();

  const address = await staking.getAddress();
  console.log("StakingManager deployed to:", address);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});