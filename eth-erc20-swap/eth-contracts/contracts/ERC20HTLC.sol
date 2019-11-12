pragma solidity 0.5.8;

interface ERC20 {
    function totalSupply() external view returns (uint);
    function balanceOf(address who) external view returns (uint);
    function transfer(address to, uint value) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint);
    function transferFrom(address from, address to, uint value) external returns (bool);
    function approve(address spender, uint value) external returns (bool);
}

contract ERC20HTLC {

    struct Swap {
        uint256 outAmount;
        uint256 expireHeight;
        bytes32 randomNumberHash;
        uint64  timestamp;
        address senderAddr;    //The swap creator ethereum address
        uint256 senderChainType;
        string  receiverAddr;  //The PRA address to swap out
        uint256 receiverChainType;
        address recipientAddr; //The ethereum address to lock swapped assets
    }

    enum States {
        INVALID,
        OPEN,
        COMPLETED,
        EXPIRED
    }

    enum ChainTypes {
        ETH,
        PRA
    }

    // Events
    event HTLC(address indexed _msgSender, address _recipientAddr, string indexed _receiverAddr, bytes32 indexed _swapID, bytes32 _randomNumberHash, uint64 _timestamp, uint256 _expireHeight, uint256 _outAmount, uint256 _praAmount);
    event Claimed(address indexed _msgSender, address _recipientAddr, string indexed _receiverAddr, bytes32 indexed _swapID, bytes32 _randomNumber);
    event Refunded(address indexed _msgSender, address _recipientAddr, string indexed _receiverAddr, bytes32 indexed _swapID, bytes32 _randomNumberHash);
    
    // Storage, key: swapID
    mapping (bytes32 => Swap) private swaps;
    mapping (bytes32 => States) private swapStates;

    address public PraContractAddr;

    /// @notice Throws if the swap is not open.
    modifier onlyOpenSwaps(bytes32 _swapID) {
        require(swapStates[_swapID] == States.OPEN, "swap is not opened");
        _;
    }

    /// @notice Throws if the swap is already expired.
    modifier onlyAfterExpireHeight(bytes32 _swapID) {
        require(block.number >= swaps[_swapID].expireHeight, "swap is not expired");
        _;
    }

    /// @notice Throws if the expireHeight is reached
    modifier onlyBeforeExpireHeight(bytes32 _swapID) {
        require(block.number < swaps[_swapID].expireHeight, "swap is already expired");
        _;
    }

    /// @notice Throws if the random number is not valid.
    modifier onlyWithRandomNumber(bytes32 _swapID, bytes32 _randomNumber) {
        require(swaps[_swapID].randomNumberHash == sha256(abi.encodePacked(_randomNumber, swaps[_swapID].timestamp)), "invalid randomNumber");
        _;
    }

    /// @param _praContract The PRA contract address
    constructor(address _praContract) public {
        PraContractAddr = _praContract;
    }

    //TODO: init set recipientAddr

    /// @notice htlt locks asset to contract address and create an atomic swap.
    ///
    /// @param _randomNumberHash The hash of the random number and timestamp
    /// @param _timestamp Counted by second
    /// @param _heightSpan The number of blocks to wait before the asset can be returned to sender
    /// @param _recipientAddr The ethereum address to lock swapped assets.
    /// @param _outAmount PRA ERC20 asset to swap out.
    /// @param _praAmount PRA asset to swap in.
    /// @param _receiverAddr PRA DID to swap in.
    function htlc(
        bytes32 _randomNumberHash,
        uint64  _timestamp,
        uint256 _heightSpan,
        address _recipientAddr,
        uint256 _outAmount,
        uint256 _praAmount,
        string  calldata _receiverAddr
    ) external  returns (bool) {
        bytes32 swapID = calSwapID(_randomNumberHash, msg.sender);
        require(swapStates[swapID] == States.INVALID, "swap is opened previously");
        // Assume average block time interval is 10 second
        // The heightSpan period should be more than 10 minutes and less than one week
        require(_heightSpan >= 60 && _heightSpan <= 60480, "_heightSpan should be in [60, 60480]");
        require(_recipientAddr != address(0), "_recipientAddr should not be zero");
        require(_outAmount > 0, "_outAmount must be more than 0");
        require(_timestamp > now - 1800 && _timestamp < now + 900, "Timestamp can neither be 15 minutes ahead of the current time, nor 30 minutes later");
        require(_outAmount == _praAmount, "_outAmount must be equal _praAmount");
        //TODO: check _receiverAddr is valid

        // Store the details of the swap.
        Swap memory swap = Swap({
            outAmount: _outAmount,
            expireHeight: _heightSpan + block.number,
            randomNumberHash: _randomNumberHash,
            timestamp: _timestamp,
            senderAddr: msg.sender,
            senderChainType: uint256(ChainTypes.ETH),
            receiverAddr: _receiverAddr,
            receiverChainType: uint256(ChainTypes.PRA),
            recipientAddr: _recipientAddr
        });

        swaps[swapID] = swap;
        swapStates[swapID] = States.OPEN;

        // Transfer pra token to the swap contract
        require(ERC20(PraContractAddr).transferFrom(msg.sender, address(this), _outAmount), "failed to transfer client asset to swap contract");

        // Emit initialization event
        emit HTLC(msg.sender, _recipientAddr, _receiverAddr, swapID, _randomNumberHash, _timestamp, swap.expireHeight, _outAmount, _praAmount);
        
        return true;
    }

    /// @notice claim claims the previously locked asset.
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    /// @param _randomNumber The random number
    function claim(bytes32 _swapID, bytes32 _randomNumber) external onlyOpenSwaps(_swapID) onlyBeforeExpireHeight(_swapID) onlyWithRandomNumber(_swapID, _randomNumber) returns (bool) {
        // Complete the swap.
        swapStates[_swapID] = States.COMPLETED;

        address recipientAddr = swaps[_swapID].recipientAddr;
        string  memory receiverAddr = swaps[_swapID].receiverAddr;
        //uint256 receiverChainType = swaps[_swapID].receiverChainType;
        //uint256 senderChainType = swaps[_swapID].senderChainType;
        uint256 outAmount = swaps[_swapID].outAmount;
        //bytes32 randomNumberHash = swaps[_swapID].randomNumberHash;

        // Pay erc20 token to recipient
        require(ERC20(PraContractAddr).transfer(recipientAddr, outAmount), "Failed to transfer locked asset to recipient");

        // delete closed swap
        delete swaps[_swapID];

        // Emit completion event
        emit Claimed(msg.sender, recipientAddr, receiverAddr, _swapID, _randomNumber);

        return true;
    }

    /// @notice refund refunds the previously locked asset.
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    function refund(bytes32 _swapID) external onlyOpenSwaps(_swapID) onlyAfterExpireHeight(_swapID) returns (bool) {
        // Expire the swap.
        swapStates[_swapID] = States.EXPIRED;

        address swapSender = swaps[_swapID].senderAddr;
        string  memory receiverAddr = swaps[_swapID].receiverAddr;
        uint256 outAmount = swaps[_swapID].outAmount;
        bytes32 randomNumberHash = swaps[_swapID].randomNumberHash;

        // refund erc20 token to swap creator
        require(ERC20(PraContractAddr).transfer(swapSender, outAmount), "Failed to transfer locked asset back to swap creator");

        // delete closed swap
        delete swaps[_swapID];

        // Emit expire event
        emit Refunded(msg.sender, swapSender, receiverAddr, _swapID, randomNumberHash);

        return true;
    }

    /// @notice query an atomic swap by randomNumberHash
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    function queryOpenSwap(bytes32 _swapID) external view returns(bytes32 _randomNumberHash, uint64 _timestamp, uint256 _expireHeight, uint256 _outAmount, address _sender, address _recipient) {
        Swap memory swap = swaps[_swapID];
        return (
            swap.randomNumberHash,
            swap.timestamp,
            swap.expireHeight,
            swap.outAmount,
            swap.senderAddr,
            swap.recipientAddr
        );
    }

    /// @notice Checks whether a swap with specified swapID exist
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    function isSwapExist(bytes32 _swapID) external view returns (bool) {
        return (swapStates[_swapID] != States.INVALID);
    }

    /// @notice Checks whether a swap is refundable or not.
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    function refundable(bytes32 _swapID) external view returns (bool) {
        return (block.number >= swaps[_swapID].expireHeight && swapStates[_swapID] == States.OPEN);
    }

    /// @notice Checks whether a swap is claimable or not.
    ///
    /// @param _swapID The hash of randomNumberHash, swap creator and swap recipient
    function claimable(bytes32 _swapID) external view returns (bool) {
        return (block.number < swaps[_swapID].expireHeight && swapStates[_swapID] == States.OPEN);
    }

    /// @notice Calculate the swapID from randomNumberHash and swapCreator
    ///
    /// @param _randomNumberHash The hash of random number and timestamp.
    /// @param _swapSender The creator of swap.
    function calSwapID(bytes32 _randomNumberHash, address _swapSender) public pure returns (bytes32) {
        return sha256(abi.encodePacked(_randomNumberHash, _swapSender));
    }
}
