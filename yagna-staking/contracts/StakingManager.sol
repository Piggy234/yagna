// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title Yagna 算力质押管理合约
 * @notice 实现房东质押、奖励发放及违规处罚(Slashing)
 */
contract StakingManager is ReentrancyGuard {
    address public admin; // 后端调度器地址 (部署者)

    struct Landlord {
        uint256 stakedAmount;  // 质押本金
        uint256 rewards;       // 待提取奖励
        bool isRegistered;     // 是否已激活节点
        uint256 slashCount;    // 惩罚次数，用于阶梯惩罚
    }

    mapping(address => Landlord) public landlords;

    event Staked(address indexed user, uint256 amount);
    event Slashed(address indexed user, uint256 amount, string reason, uint256 slashCount);
    event RewardPaid(address indexed user, uint256 amount);
    event Withdrawn(address indexed user, uint256 stakedAmount, uint256 rewards);

    modifier onlyAdmin() {
        require(msg.sender == admin, "Only admin can perform this action");
        _;
    }

    constructor() {
        admin = msg.sender;
    }

    // 1. 房东质押 (前端调用)
    function deposit() public payable {
        require(msg.value >= 0.01 ether, "Minimum stake is 0.01 MATIC");
        landlords[msg.sender].stakedAmount += msg.value;
        landlords[msg.sender].isRegistered = true;
        emit Staked(msg.sender, msg.value);
    }

    // 2. 惩罚逻辑 (后端检测到掉线时调用)
    // 创新点：阶梯惩罚，根据惩罚次数增加惩罚比例
    function slash(address _landlord, uint256 _basePercentage) public onlyAdmin {
        require(_basePercentage > 0 && _basePercentage <= 100, "Invalid percentage");
        Landlord storage landlord = landlords[_landlord];
        require(landlord.stakedAmount > 0, "No stake to slash");

        // 创新点：阶梯惩罚。实际比例 = 基础比例 + (惩罚次数 * 5%)
        uint256 effectivePercentage = _basePercentage + (landlord.slashCount * 5); 
        if (effectivePercentage > 100) effectivePercentage = 100; // 上限 100%

        uint256 slashAmount = (landlord.stakedAmount * effectivePercentage) / 100;
        
        landlord.stakedAmount -= slashAmount;
        landlord.slashCount += 1;

        // 建议使用 call 而非 transfer (更符合现代安全标准)
        (bool success, ) = payable(admin).call{value: slashAmount}("");
        require(success, "Transfer failed");

        emit Slashed(_landlord, slashAmount, "Node offline", landlord.slashCount);
    }

    // 3. 奖励发放 (任务结算时由后端调用)
    function addReward(address _landlord) public payable onlyAdmin {
        landlords[_landlord].rewards += msg.value;
        emit RewardPaid(_landlord, msg.value);
    }

    // 4. 提取本金和奖励 (房东调用)
    function withdraw() public nonReentrant {
        Landlord storage landlord = landlords[msg.sender];
        uint256 staked = landlord.stakedAmount;
        uint256 rewards = landlord.rewards;
        uint256 total = staked + rewards;
        require(total > 0, "Nothing to withdraw");

        landlord.stakedAmount = 0;
        landlord.rewards = 0;
        landlord.isRegistered = false; // 提取后取消注册
        
        payable(msg.sender).transfer(total);
        emit Withdrawn(msg.sender, staked, rewards);
    }

    // 查看节点状态
    function getStatus(address _user) public view returns (uint256 stakedAmount, uint256 rewards, uint256 slashCount) {
        Landlord memory landlord = landlords[_user];
        return (landlord.stakedAmount, landlord.rewards, landlord.slashCount);
    }
}