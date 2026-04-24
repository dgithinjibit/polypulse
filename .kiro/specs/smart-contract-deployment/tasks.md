# Tasks: Stellar Smart Contract Deployment

## Task 1: Set Up Deployment Environment

### 1.1 Install Stellar CLI
- [ ] Run `cargo install --locked stellar-cli --features opt`
- [ ] Wait for installation to complete (~5 minutes)
- [ ] Verify installation: `stellar --version`
- [ ] Expected output: `stellar 21.x.x` or higher

### 1.2 Configure Stellar CLI for testnet
- [ ] Add testnet network:
  ```bash
  stellar network add \
    --global testnet \
    --rpc-url https://soroban-testnet.stellar.org:443 \
    --network-passphrase "Test SDF Network ; September 2015"
  ```
- [ ] Verify network added: `stellar network ls`
- [ ] Expected: testnet in list

### 1.3 Create deployer identity
- [ ] Generate key: `stellar keys generate deployer --network testnet`
- [ ] Get address: `stellar keys address deployer`
- [ ] Save address for later use

### 1.4 Fund deployer account
- [ ] Fund from friendbot: `stellar keys fund deployer --network testnet`
- [ ] Verify balance: Check address on https://stellar.expert/explorer/testnet
- [ ] Expected: 10,000 XLM balance

### 1.5 Verify environment
- [ ] Test RPC connection: `curl https://soroban-testnet.stellar.org/health`
- [ ] Expected: `{"status":"healthy"}`
- [ ] Verify Rust toolchain: `rustc --version`
- [ ] Expected: Rust 1.74.0 or higher

## Task 2: Build Smart Contracts

### 2.1 Add WASM target
- [ ] Run `rustup target add wasm32-unknown-unknown`
- [ ] Verify target added: `rustup target list | grep wasm32`

### 2.2 Build P2P bet contract
- [ ] Navigate: `cd contracts/contracts/market`
- [ ] Build: `stellar contract build`
- [ ] Verify WASM created: `ls -lh ../../target/wasm32-unknown-unknown/release/market.wasm`
- [ ] Expected: File exists, ~150 KB

### 2.3 Build multi-pool contract
- [ ] Navigate: `cd contracts/contracts/multi-pool`
- [ ] Build: `stellar contract build`
- [ ] Verify WASM created: `ls -lh ../../target/wasm32-unknown-unknown/release/multi_pool.wasm`
- [ ] Expected: File exists, ~180 KB

### 2.4 Verify builds
- [ ] Check both WASM files exist
- [ ] Verify file sizes are reasonable (<500 KB each)
- [ ] Run `file` command to verify WASM format

## Task 3: Deploy P2P Bet Contract

### 3.1 Deploy contract
- [ ] Navigate: `cd contracts/contracts/market`
- [ ] Deploy:
  ```bash
  stellar contract deploy \
    --wasm ../../target/wasm32-unknown-unknown/release/market.wasm \
    --source deployer \
    --network testnet
  ```
- [ ] Wait for deployment (~30 seconds)
- [ ] Copy returned contract ID

### 3.2 Save contract ID
- [ ] Export: `export P2P_CONTRACT_ID="<returned_contract_id>"`
- [ ] Save to file: `echo "P2P Contract ID: $P2P_CONTRACT_ID" >> ../../DEPLOYED_CONTRACTS.txt`
- [ ] Verify saved: `cat ../../DEPLOYED_CONTRACTS.txt`

### 3.3 Verify deployment
- [ ] Test contract:
  ```bash
  stellar contract invoke \
    --id $P2P_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    --help
  ```
- [ ] Expected: List of available functions
- [ ] Verify functions include: create_bet, join_bet, report_outcome, etc.

### 3.4 Test contract function
- [ ] Invoke create_bet:
  ```bash
  stellar contract invoke \
    --id $P2P_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    create_bet \
    --creator $(stellar keys address deployer) \
    --question "Test bet" \
    --stake 10000000 \
    --end_time 1735689600
  ```
- [ ] Expected: Bet ID returned (e.g., `1`)
- [ ] Verify no errors

## Task 4: Deploy Multi-Pool Contract

### 4.1 Deploy contract
- [ ] Navigate: `cd contracts/contracts/multi-pool`
- [ ] Deploy:
  ```bash
  stellar contract deploy \
    --wasm ../../target/wasm32-unknown-unknown/release/multi_pool.wasm \
    --source deployer \
    --network testnet
  ```
- [ ] Wait for deployment (~30 seconds)
- [ ] Copy returned contract ID

### 4.2 Save contract ID
- [ ] Export: `export MULTI_POOL_CONTRACT_ID="<returned_contract_id>"`
- [ ] Save to file: `echo "Multi-Pool Contract ID: $MULTI_POOL_CONTRACT_ID" >> ../../DEPLOYED_CONTRACTS.txt`
- [ ] Verify saved: `cat ../../DEPLOYED_CONTRACTS.txt`

### 4.3 Verify deployment
- [ ] Test contract:
  ```bash
  stellar contract invoke \
    --id $MULTI_POOL_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    --help
  ```
- [ ] Expected: List of available functions
- [ ] Verify functions include: create_pool, join_pool, get_odds, etc.

### 4.4 Test contract function
- [ ] Invoke create_pool:
  ```bash
  stellar contract invoke \
    --id $MULTI_POOL_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    create_pool \
    --creator $(stellar keys address deployer) \
    --question "Test pool" \
    --end_time 1735689600
  ```
- [ ] Expected: Pool ID returned (e.g., `1`)
- [ ] Verify no errors

## Task 5: Configure Frontend Environment Variables

### 5.1 Update local .env.production
- [ ] Navigate: `cd frontend`
- [ ] Add contract IDs:
  ```bash
  cat >> .env.production << EOF
  VITE_STELLAR_MARKET_CONTRACT_ID=$P2P_CONTRACT_ID
  VITE_STELLAR_MULTIPOOL_CONTRACT_ID=$MULTI_POOL_CONTRACT_ID
  EOF
  ```
- [ ] Verify added: `cat .env.production`

### 5.2 Update 4everland environment variables
- [ ] Go to https://dashboard.4everland.org
- [ ] Select PolyPulse project
- [ ] Go to Settings → Environment Variables
- [ ] Click "Add Variable"
- [ ] Add `VITE_STELLAR_MARKET_CONTRACT_ID` with P2P contract ID
- [ ] Add `VITE_STELLAR_MULTIPOOL_CONTRACT_ID` with multi-pool contract ID
- [ ] Click "Save"

### 5.3 Verify environment variables
- [ ] Check both variables are listed
- [ ] Verify values are correct (start with 'C')
- [ ] Verify no typos in variable names

### 5.4 Trigger frontend redeploy
- [ ] Commit changes:
  ```bash
  git add frontend/.env.production contracts/DEPLOYED_CONTRACTS.txt
  git commit -m "feat: Add deployed contract IDs for testnet"
  git push origin main
  ```
- [ ] Wait for 4everland auto-deploy (~2-3 minutes)
- [ ] Verify deployment succeeded

## Task 6: Test Contracts via CLI

### 6.1 Test P2P contract - Full flow
- [ ] Create bet (already done in Task 3.4)
- [ ] Join bet:
  ```bash
  stellar contract invoke \
    --id $P2P_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    join_bet \
    --participant $(stellar keys address deployer) \
    --bet_id 1 \
    --position true
  ```
- [ ] Get bet details:
  ```bash
  stellar contract invoke \
    --id $P2P_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    get_bet \
    --bet_id 1
  ```
- [ ] Verify bet state is correct

### 6.2 Test multi-pool contract - Full flow
- [ ] Create pool (already done in Task 4.4)
- [ ] Join pool:
  ```bash
  stellar contract invoke \
    --id $MULTI_POOL_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    join_pool \
    --participant $(stellar keys address deployer) \
    --pool_id 1 \
    --position true \
    --stake 10000000
  ```
- [ ] Get odds:
  ```bash
  stellar contract invoke \
    --id $MULTI_POOL_CONTRACT_ID \
    --source deployer \
    --network testnet \
    -- \
    get_odds \
    --pool_id 1
  ```
- [ ] Verify odds calculation is correct

### 6.3 Document test results
- [ ] Create test results file
- [ ] List all tests performed
- [ ] Include expected vs actual results
- [ ] Note any issues or errors

## Task 7: End-to-End Frontend Testing

### 7.1 Prepare test environment
- [ ] Open frontend: https://polypulse-m11gtupi-dgithinjibit.ipfs.4everland.app
- [ ] Open browser console (F12)
- [ ] Check for contract ID logs
- [ ] Verify no errors in console

### 7.2 Set up Freighter wallet
- [ ] Install Freighter extension if not installed
- [ ] Switch to testnet mode
- [ ] Import deployer account or create new account
- [ ] Fund account from friendbot: https://friendbot.stellar.org?addr=<address>
- [ ] Verify balance shows in Freighter

### 7.3 Test bet creation
- [ ] Connect wallet to frontend
- [ ] Click "Create Bet" button
- [ ] Fill in form:
  - Question: "Will it rain tomorrow?"
  - Stake: 10 XLM
  - End time: Tomorrow
- [ ] Click "Create" button
- [ ] Approve transaction in Freighter
- [ ] Wait for confirmation (~5 seconds)
- [ ] Verify success message appears
- [ ] Verify bet appears in list

### 7.4 Test bet joining
- [ ] Find created bet in list
- [ ] Click "Join Bet" button
- [ ] Select position (Yes or No)
- [ ] Approve transaction in Freighter
- [ ] Wait for confirmation
- [ ] Verify participant count updates
- [ ] Verify bet state changes to "Active"

### 7.5 Test bet details
- [ ] Click on bet to view details
- [ ] Verify all information is correct:
  - Question
  - Stakes
  - Participants
  - End time
  - Current state
- [ ] Check transaction hashes are clickable
- [ ] Verify links to Stellar Explorer work

### 7.6 Document frontend testing
- [ ] Screenshot successful bet creation
- [ ] Screenshot bet details page
- [ ] Note any UI issues or bugs
- [ ] List any error messages encountered

## Task 8: Create Deployment Documentation

### 8.1 Create DEPLOYMENT_GUIDE.md
- [ ] Create file in contracts directory
- [ ] Add contract IDs section
- [ ] Add deployment date and network
- [ ] Add deployer account address
- [ ] Add build commands
- [ ] Add deployment commands
- [ ] Add test commands
- [ ] Add frontend configuration steps
- [ ] Add troubleshooting section

### 8.2 Update README.md
- [ ] Add "Deployed Contracts" section
- [ ] List contract IDs
- [ ] Add links to Stellar Explorer
- [ ] Update deployment status
- [ ] Add verification steps

### 8.3 Create CONTRACT_ADDRESSES.json
- [ ] Create JSON file with contract addresses
- [ ] Format:
  ```json
  {
    "network": "testnet",
    "deployed_at": "2026-04-24",
    "contracts": {
      "p2p_bet": "<P2P_CONTRACT_ID>",
      "multi_pool": "<MULTI_POOL_CONTRACT_ID>"
    },
    "deployer": "<deployer_address>"
  }
  ```
- [ ] Commit to repository

### 8.4 Commit documentation
- [ ] Stage all documentation files
- [ ] Commit with message: "docs: Add smart contract deployment guide"
- [ ] Push to GitHub

## Task 9: Verify Complete System

### 9.1 Verify backend
- [ ] Test health endpoint: `curl https://polypulse-backend-436v.onrender.com/health`
- [ ] Expected: `{"status":"ok"}`
- [ ] Test API endpoint: `curl https://polypulse-backend-436v.onrender.com/api/v1/auth/stellar-nonce`
- [ ] Expected: Nonce returned

### 9.2 Verify frontend
- [ ] Open frontend URL
- [ ] Verify page loads without errors
- [ ] Check browser console for errors
- [ ] Verify contract IDs are logged

### 9.3 Verify contracts
- [ ] Check P2P contract on Stellar Explorer
- [ ] Check multi-pool contract on Stellar Explorer
- [ ] Verify contracts are accessible
- [ ] Verify contract state is correct

### 9.4 Verify end-to-end flow
- [ ] Create bet via frontend
- [ ] Join bet via frontend
- [ ] Verify both transactions succeed
- [ ] Verify bet state updates correctly
- [ ] Verify UI reflects changes

## Success Criteria

- [ ] Stellar CLI installed and configured
- [ ] Deployer account funded with 10,000+ XLM
- [ ] Both contracts built successfully
- [ ] P2P bet contract deployed to testnet
- [ ] Multi-pool contract deployed to testnet
- [ ] Contract IDs saved and documented
- [ ] Frontend environment variables updated
- [ ] 4everland environment variables updated
- [ ] Frontend redeployed with new contract IDs
- [ ] CLI tests pass for both contracts
- [ ] Frontend can create bets
- [ ] Frontend can join bets
- [ ] Transactions confirm on testnet
- [ ] Documentation complete and committed
- [ ] End-to-end flow works correctly

## Estimated Time

- Task 1: 15 minutes
- Task 2: 10 minutes
- Task 3: 10 minutes
- Task 4: 10 minutes
- Task 5: 10 minutes
- Task 6: 20 minutes
- Task 7: 15 minutes
- Task 8: 10 minutes
- Task 9: 10 minutes

**Total**: ~110 minutes (~2 hours)

## Troubleshooting

### If deployment fails
- Check deployer account balance
- Verify WASM file exists and is valid
- Check network connectivity
- Retry deployment

### If contract invocation fails
- Verify contract ID is correct
- Check function name and arguments
- Verify source account has balance
- Check network configuration

### If frontend can't connect
- Verify environment variables are set
- Check contract IDs are correct
- Verify Freighter is on testnet
- Check browser console for errors

### If transaction fails
- Check account balance
- Verify transaction parameters
- Check contract state
- Review error message in console
