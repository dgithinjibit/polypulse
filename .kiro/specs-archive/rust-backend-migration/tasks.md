continueconti# Implementation Plan: Rust Backend Migration for PolyPulse

## Overview

This implementation plan breaks down the migration of PolyPulse from Python Django to Rust/Axum into discrete, executable tasks. The plan follows a logical progression: core API endpoints first, then social features, challenges, admin operations, WebSocket support, and finally testing. Each task builds on previous work and references specific requirements from the requirements document.

## Tasks

- [x] 1. Implement Poll Management API Endpoints
  - [x] 1.1 Implement create poll endpoint (POST /api/v1/polls)
    - Parse and validate CreatePollRequest (title, description, options, closes_at, category)
    - Check daily poll creation limit from user profile
    - Insert poll record with status="open"
    - Insert poll options (2-10 options, validate is_yes flags)
    - Create associated market with initial liquidity_b and shares_outstanding
    - Return poll ID and details
    - _Requirements: 1.1, 1.2, 1.6, 1.7_
  
  - [x] 1.2 Implement list polls endpoint (GET /api/v1/polls)
    - Parse query parameters (category, status, creator_id, limit, offset)
    - Query polls with filters and pagination
    - Join with poll_options and markets for price data
    - Calculate current prices using LMSR for each poll
    - Return paginated poll list with options and prices
    - _Requirements: 1.3_
  
  - [x] 1.3 Implement get poll detail endpoint (GET /api/v1/polls/:id)
    - Query poll by ID with options
    - Query associated market and calculate current prices
    - Query bet statistics (total bets, unique bettors)
    - Query user's position if authenticated
    - Return comprehensive poll details
    - _Requirements: 1.4_
  
  - [x] 1.4 Implement poll auto-close background task
    - Create scheduled task that runs every minute
    - Query polls where status="open" AND closes_at <= NOW()
    - Update status to "closed" for expired polls
    - Log closed poll IDs
    - _Requirements: 1.5_

- [x] 2. Implement Betting and Market Operations
  - [x] 2.1 Implement place bet endpoint (POST /api/v1/bets)
    - Parse PlaceBetRequest (poll_id, option_id, amount)
    - Validate amount >= 1.0 and amount <= user.balance
    - Start database transaction with SELECT FOR UPDATE on market and user
    - Verify poll status is "open"
    - Calculate shares using LMSR cost function
    - Deduct amount from user balance
    - Insert bet record
    - Upsert market_position (update option_shares and option_spent)
    - Update market shares_outstanding
    - Insert wallet_transaction with type="bet"
    - Commit transaction
    - Broadcast price update via WebSocket (stub for now)
    - Return bet confirmation with shares received
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 24.1, 24.4, 24.5_
  
  - [x] 2.2 Write unit tests for bet placement
    - Test successful bet placement
    - Test insufficient balance rejection
    - Test closed market rejection
    - Test minimum bet amount enforcement
    - Test transaction rollback on error
    - _Requirements: 3.1, 3.2, 3.3, 3.7, 3.8_
  
  - [x] 2.3 Implement sell shares endpoint (POST /api/v1/bets/sell)
    - Parse SellSharesRequest (poll_id, option_id, shares)
    - Start database transaction with SELECT FOR UPDATE
    - Query user's market_position
    - Verify user owns sufficient shares for option_id
    - Calculate refund amount using LMSR cost function
    - Credit refund to user balance
    - Update market_position (reduce option_shares and option_spent)
    - Update market shares_outstanding
    - Insert wallet_transaction with type="refund"
    - Commit transaction
    - Broadcast price update via WebSocket (stub for now)
    - Return sell confirmation with refund amount
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 24.4, 24.5_
  
  - [x] 2.4 Implement get user positions endpoint (GET /api/v1/positions)
    - Query all market_positions for authenticated user
    - Join with polls and markets
    - For each position, calculate current value using current market prices
    - Calculate profit/loss as current_value - option_spent
    - Filter out positions with zero shares
    - Return positions with poll details, shares, value, and P&L
    - _Requirements: 7.1, 7.2, 7.3, 7.4_
  
  - [x] 2.5 Implement get market prices endpoint (GET /api/v1/markets/:poll_id/prices)
    - Query market by poll_id
    - Calculate current prices for all options using LMSR
    - Return prices as map {option_id: price}
    - _Requirements: 2.3, 2.7_
  
  - [x] 2.6 Implement get price history endpoint (GET /api/v1/markets/:poll_id/history)
    - Query market_price_snapshots for poll's market
    - Order by created_at descending
    - Support pagination (limit, offset)
    - Return price snapshots with timestamps
    - _Requirements: 2.3_

- [x] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Implement Admin Market Resolution
  - [x] 4.1 Implement resolve poll endpoint (POST /api/v1/polls/:id/resolve)
    - Verify user is admin (is_staff=true)
    - Parse ResolvePollRequest (winning_option_id)
    - Start database transaction with SELECT FOR UPDATE on poll
    - Verify poll status is "closed"
    - Verify closes_at has passed
    - Verify winning_option_id belongs to poll
    - Update poll.winning_option_id and status="resolved"
    - Query all market_positions with shares in winning option
    - For each winning position, calculate payout as shares * 1.0
    - Credit each winner's balance
    - Insert wallet_transaction with type="win" for each winner
    - Create notification for each winner
    - Update user profile statistics (streaks, accuracy)
    - Commit transaction
    - Broadcast resolution event via WebSocket (stub for now)
    - Return resolution summary (winners count, total payout)
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 24.3, 24.4, 24.5_
  
  - [ ]* 4.2 Write unit tests for poll resolution
    - Test successful resolution with multiple winners
    - Test admin permission enforcement
    - Test closed status requirement
    - Test closes_at validation
    - Test payout calculation correctness
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.10_
  
  - [x] 4.3 Implement suspend poll endpoint (POST /api/v1/polls/:id/suspend)
    - Verify user is admin
    - Start database transaction
    - Update poll status to "suspended"
    - Commit transaction
    - Return success message
    - _Requirements: 6.1, 6.2_
  
  - [x] 4.4 Implement cancel poll endpoint (POST /api/v1/polls/:id/cancel)
    - Verify user is admin
    - Start database transaction with SELECT FOR UPDATE
    - Update poll status to "cancelled"
    - Query all bets for this poll
    - For each bet, credit user with original bet amount
    - Insert wallet_transaction with type="refund" for each bet
    - Update all market_positions to zero shares
    - Create notification for each participant
    - Commit transaction
    - Return cancellation summary
    - _Requirements: 6.3, 6.4, 6.5, 6.6, 6.7, 24.4, 24.5_

- [x] 5. Implement Challenge System
  - [x] 5.1 Implement create challenge endpoint (POST /api/v1/challenges)
    - Parse CreateChallengeRequest (question, amount, creator_choice, opponent_id, is_open, poll_id, expires_at)
    - Validate amount >= 1.0
    - Verify creator has sufficient balance
    - If poll_id provided, verify poll exists and is open
    - If direct challenge, verify opponent_id is provided
    - Insert challenge record with status="pending"
    - If direct challenge, create notification for opponent
    - Return challenge details
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_
  
  - [x] 5.2 Implement list challenges endpoint (GET /api/v1/challenges)
    - Parse query parameters (status, is_open, creator_id, opponent_id)
    - Query challenges with filters
    - Join with users for creator and opponent details
    - Support pagination
    - Return challenge list
    - Create a tailgate for the developer
    - _Requirements: 8.1, 8.2_
  
  - [x] 5.3 Implement get challenge detail endpoint (GET /api/v1/challenges/:id)
    - Query challenge by ID
    - Join with users for creator and opponent details
    - If poll_id exists, join with poll details
    - Return comprehensive challenge details
    - _Requirements: 8.1, 8.2_
  
  - [x] 5.4 Implement accept challenge endpoint (POST /api/v1/challenges/:id/accept)
    - Start database transaction with SELECT FOR UPDATE on challenge and both users
    - Verify challenge status is "pending"
    - Verify challenge has not expired (expires_at > NOW())
    - Verify both creator and acceptor have sufficient balance
    - Deduct amount from both users
    - Insert wallet_transaction with type="challenge_stake" for both users
    - Update challenge status to "accepted"
    - Update challenge.opponent_id if open challenge
    - Create notification for creator
    - Commit transaction
    - Return success message
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 24.2, 24.4, 24.5_
  
  - [x] 5.5 Implement resolve challenge endpoint (POST /api/v1/challenges/:id/resolve)
    - Parse ResolveChallengeRequest (winner_id or use poll result)
    - Start database transaction with SELECT FOR UPDATE
    - Verify challenge status is "accepted"
    - If poll_id exists, determine winner based on poll.winning_option_id and creator_choice
    - If manual resolution, use provided winner_id
    - Credit winner with 2 * amount
    - Insert wallet_transaction with type="challenge_win"
    - Update challenge status to "resolved", set winner_id and resolved_at
    - Create notifications for both participants
    - Commit transaction
    - Return resolution result
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8, 24.4, 24.5_
  
  - [x] 5.6 Implement cancel challenge endpoint (POST /api/v1/challenges/:id/cancel)
    - Verify user is challenge creator
    - Verify challenge status is "pending"
    - Update challenge status to "cancelled"
    - If direct challenge, create notification for opponent
    - Return success message
    - _Requirements: 11.1, 11.2, 11.3, 11.4_
  
  - [ ]* 5.7 Write integration tests for challenge flow
    - Test complete challenge lifecycle (create → accept → resolve)
    - Test open challenge acceptance
    - Test challenge expiration
    - Test cancellation
    - Test balance checks
    - _Requirements: 8.1-8.8, 9.1-9.8, 10.1-10.8, 11.1-11.4_

- [x] 6. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Implement Comments and Social Features
  - [x] 7.1 Implement list comments endpoint (GET /api/v1/polls/:poll_id/comments)
    - Query all comments for poll_id where parent_id IS NULL (top-level)
    - For each top-level comment, recursively query replies
    - Join with users for author details
    - Query like counts for each comment
    - If authenticated, query which comments user has liked
    - Build comment tree structure
    - Return nested comment tree
    - _Requirements: 12.3, 13.3, 13.4_
  
  - [x] 7.2 Implement create comment endpoint (POST /api/v1/polls/:poll_id/comments)
    - Parse CreateCommentRequest (content, parent_id)
    - Validate content is not empty
    - Insert comment record
    - Parse content for @mentions using regex
    - For each mentioned username, query user and create notification
    - Return comment details
    - _Requirements: 12.1, 12.2, 12.4, 12.5, 12.6_
  
  - [x] 7.3 Implement toggle like endpoint (POST /api/v1/comments/:id/like)
    - Query if like exists for (comment_id, user_id)
    - If exists, delete like record (unlike)
    - If not exists, insert like record (like)
    - Return new like status and count
    - _Requirements: 13.1, 13.2_
  
  - [ ]* 7.4 Write unit tests for mention parsing
    - Test single mention extraction
    - Test multiple mentions extraction
    - Test mentions with special characters
    - Test no mentions case
    - _Requirements: 12.4, 12.5_

- [x] 8. Implement Wallet and M-Pesa Integration
  - [x] 8.1 Implement get balance endpoint (GET /api/v1/wallet/balance)
    - Query user balance
    - Return balance
    - _Requirements: 14.1_
  
  - [x] 8.2 Implement get transactions endpoint (GET /api/v1/wallet/transactions)
    - Parse query parameters (transaction_type, limit, offset)
    - Query wallet_transactions for user with filters
    - Order by created_at descending
    - Support pagination
    - Return transaction list
    - _Requirements: 14.2, 14.3, 14.4_
  
  - [x] 8.3 Implement initiate M-Pesa deposit endpoint (POST /api/v1/wallet/mpesa/deposit)
    - Parse MpesaDepositRequest (phone, amount)
    - Validate phone number format (Kenyan format)
    - Validate amount > 0
    - Insert mpesa_transaction record with status="pending"
    - Call MpesaService::initiate_stk_push with phone and amount
    - Store CheckoutRequestID and MerchantRequestID in mpesa_transaction
    - Return CheckoutRequestID to user for status polling
    - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5, 15.6_
  
  - [x] 8.4 Implement M-Pesa callback endpoint (POST /api/v1/wallet/mpesa/callback)
    - Parse MpesaCallback payload
    - Verify callback signature (if Safaricom provides signature)
    - Query mpesa_transaction by CheckoutRequestID
    - If result indicates success (ResultCode=0):
      - Start database transaction
      - Credit user balance with amount
      - Insert wallet_transaction with type="deposit"
      - Update mpesa_transaction status to "completed"
      - Commit transaction
    - If result indicates failure:
      - Update mpesa_transaction status to "failed"
    - Return 200 OK acknowledgment
    - _Requirements: 15.7, 15.8, 15.9, 15.10, 15.11, 24.4, 24.5_
  
  - [x] 8.5 Implement get M-Pesa status endpoint (GET /api/v1/wallet/mpesa/status/:checkout_id)
    - Query mpesa_transaction by checkout_request_id
    - Verify transaction belongs to authenticated user
    - Return transaction status and details
    - _Requirements: 15.12_
  
  - [ ]* 8.6 Write integration tests for M-Pesa flow (mocked)
    - Mock Daraja API responses
    - Test successful deposit flow
    - Test failed deposit handling
    - Test callback processing
    - Test status polling
    - _Requirements: 15.1-15.12_

- [x] 9. Implement Notifications
  - [x] 9.1 Implement list notifications endpoint (GET /api/v1/notifications)
    - Parse query parameters (limit, offset)
    - Query notifications for user ordered by created_at descending
    - Join with users for actor details
    - Support pagination
    - Return notification list
    - _Requirements: 16.1, 16.2_
  
  - [x] 9.2 Implement mark notification read endpoint (POST /api/v1/notifications/:id/read)
    - Verify notification belongs to user
    - Update is_read to true
    - Return success message
    - _Requirements: 16.3_
  
  - [x] 9.3 Implement mark all read endpoint (POST /api/v1/notifications/read-all)
    - Update all notifications for user where is_read=false to is_read=true
    - Return success message
    - _Requirements: 16.4_
  
  - [x] 9.4 Implement get unread count endpoint (GET /api/v1/notifications/unread-count)
    - Count notifications where user_id=user AND is_read=false
    - Return count
    - _Requirements: 16.5_

- [x] 10. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. Implement WebSocket Real-Time Updates
  - [x] 11.1 Implement WebSocket connection handler
    - Parse JWT from query parameter
    - Validate JWT and extract user claims
    - Upgrade HTTP connection to WebSocket
    - Generate unique connection ID
    - Register connection in Redis with user_id mapping
    - Start ping/pong heartbeat loop (60 second interval)
    - Handle incoming subscription messages
    - Handle connection close and cleanup
    - _Requirements: 19.1, 19.2, 19.3, 19.4, 19.5, 19.7_
  
  - [x] 11.2 Implement WebSocket subscription management
    - Define subscription message types (subscribe_poll, unsubscribe_poll)
    - On subscribe_poll, add connection to Redis set for poll_id
    - On unsubscribe_poll, remove connection from Redis set
    - Enforce max 5 concurrent connections per user
    - _Requirements: 19.6, 20.1_
  
  - [x] 11.3 Implement WebSocket broadcasting for poll events
    - Create broadcast_to_poll function using Redis pub/sub
    - Integrate with bet placement to broadcast price updates
    - Integrate with poll resolution to broadcast resolution events
    - Integrate with comment creation to broadcast comment events
    - Define message schemas for each event type
    - _Requirements: 20.2, 20.3, 20.4, 20.7_
  
  - [x] 11.4 Implement WebSocket user notifications
    - Create send_to_user function using Redis pub/sub
    - Integrate with notification creation to send real-time notifications
    - Integrate with challenge creation to send challenge invites
    - Send to all active connections for user
    - _Requirements: 16.6, 20.5, 20.6, 20.7_
  
  - [ ]* 11.5 Write integration tests for WebSocket
    - Test connection authentication
    - Test subscription management
    - Test event broadcasting
    - Test user notifications
    - Test connection limits
    - _Requirements: 19.1-19.7, 20.1-20.7_

- [x] 12. Implement Rate Limiting and Security
  - [x] 12.1 Implement rate limiting middleware
    - Use Redis to track request counts per IP/user
    - Enforce 60 req/min for anonymous users
    - Enforce 300 req/min for authenticated users
    - Enforce 10 req/min for auth endpoints
    - Enforce 30 req/min for trading endpoints
    - Enforce 5 req/min for M-Pesa endpoints
    - Return 429 Too Many Requests when limit exceeded
    - _Requirements: 21.1, 21.2, 21.3, 21.4, 21.5, 21.6_
  
  - [x] 12.2 Implement input validation middleware
    - Validate username length (3-30 characters)
    - Validate email format
    - Validate phone number format
    - Validate bet amounts (positive, within limits)
    - Validate poll options count (2-10)
    - Sanitize comment content to prevent XSS
    - Return 400 Bad Request for invalid inputs
    - _Requirements: 22.1, 22.2, 22.3, 22.4, 22.5, 22.6, 22.7_
  
  - [x] 12.3 Implement comprehensive error handling
    - Map database errors to 500 Internal Server Error
    - Map validation errors to 400 Bad Request
    - Map not found errors to 404 Not Found
    - Map authorization errors to 403 Forbidden
    - Map authentication errors to 401 Unauthorized
    - Map conflicts to 409 Conflict
    - Log all errors with request context
    - Sanitize error messages (no stack traces)
    - _Requirements: 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8_

- [x] 13. Implement Caching Layer
  - [x] 13.1 Implement session caching
    - Cache user sessions in Redis with 7-day TTL
    - Check cache before database query
    - Invalidate on logout
    - _Requirements: 25.1_
  
  - [x] 13.2 Implement poll and market caching
    - Cache poll details in Redis with 30-second TTL
    - Cache market prices in Redis with 5-second TTL
    - Invalidate cache on updates (bets, resolution)
    - Implement cache fallback to database
    - _Requirements: 25.2, 25.3, 25.4, 25.5_

- [ ] 14. Final Integration and Testing
  - [ ]* 14.1 Write property-based tests for LMSR
    - **Property 1: Price bounds** - All option prices must be between 0 and 1
    - **Validates: Requirements 2.7**
  
  - [ ]* 14.2 Write property-based tests for LMSR
    - **Property 2: Price sum** - Sum of all option prices must equal 1.0 ± ε
    - **Validates: Requirements 2.2**
  
  - [ ]* 14.3 Write property-based tests for LMSR
    - **Property 3: Round trip consistency** - Buying then selling same shares should return approximately same amount
    - **Validates: Requirements 2.4, 2.5**
  
  - [ ]* 14.4 Write property-based tests for wallet
    - **Property 4: Balance conservation** - Sum of all transaction amounts must equal current balance
    - **Validates: Requirements 14.5**
  
  - [ ]* 14.5 Write property-based tests for wallet
    - **Property 5: Non-negative balance** - Balance can never go negative
    - **Validates: Requirements 14.6**
  
  - [ ]* 14.6 Write property-based tests for challenges
    - **Property 6: Challenge pool distribution** - Total challenge pool (2x amount) must be distributed exactly once
    - **Validates: Requirements 10.8**
  
  - [ ]* 14.7 Write integration tests for complete poll lifecycle
    - Test: create poll → place bets → close → resolve → verify payouts
    - Test: poll expiration and auto-close
    - Test: suspension with refunds
    - Test: cancellation with refunds
    - _Requirements: 1.1-1.7, 3.1-3.8, 5.1-5.10, 6.1-6.7_

- [x] 15. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at key milestones
- Property tests validate universal correctness properties from the design
- Unit and integration tests validate specific examples and edge cases
- All database operations use transactions to ensure atomicity
- WebSocket integration is stubbed initially and completed in Phase 11
- Rate limiting and caching can be implemented incrementally
