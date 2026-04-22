# Frontend Integration

This directory contains TypeScript types and utilities for integrating the Soroban contracts with the React frontend.

## Overview

The frontend will interact with Soroban contracts using:
- `@creit.tech/stellar-wallets-kit` for wallet connections
- `@stellar/stellar-sdk` for transaction building
- Horizon API for querying blockchain state

## Directory Structure

```
frontend-integration/
├── types/           # TypeScript type definitions matching contract types
├── services/        # Service classes for contract interaction
└── config/          # Network and contract configuration
```

## Usage

These files will be copied or imported into the main frontend application (`frontend/src/`) during the frontend integration phase.

## Implementation Status

This directory structure is prepared for Task 5 (Frontend Integration). The actual implementation will be completed in that task.

## Next Steps

1. Install required packages in frontend:
   ```bash
   npm install @creit.tech/stellar-wallets-kit @stellar/stellar-sdk
   ```

2. Copy types and services to frontend src directory

3. Configure contract addresses from deployment

4. Implement wallet connection and contract invocation
