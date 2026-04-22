# Requirements Document: Rust Backend Migration for PolyPulse

## Introduction

This document specifies the functional requirements for migrating the PolyPulse prediction market platform from Python Django to Rust with Axum. PolyPulse enables users to create prediction markets, place bets using LMSR pricing, manage wallets with M-Pesa integration, participate in challenges, and interact through comments and notifications. The migration must preserve all existing functionality while improving performance, type safety, and maintainability.

## Glossary

- **System**: The PolyPulse Rust backend application
- **User**: An authenticated user of the platform
- **Poll**: A prediction market with multiple options
- **Market**: The LMSR-based pricing mechanism for a poll
- **Bet**: A user's purchase of shares in a poll option
- **Position**: A user's holdings of shares across poll options
- **Challenge**: A direct or open wager between users
- **LMSR**: Logarithmic Market Scoring Rule pricing algorithm
- **M-Pesa**: Mobile money payment service by Safaricom
- **STK_Push**: M-Pesa prompt sent to user's phone for payment
- **JWT**: JSON Web Token for authentication
- **Nonce**: Single-use random value for wallet authentication
- **WebSocket**: Bidirectional real-time communication protocol
- **Admin**: A user with elevated privileges (is_staff = true)
- **Balance**: User's available funds in the platform
- **Transaction**: A record of balance change (deposit, bet, win, refund)
- **Notification**: A message sent to a user about platform events
- **Comment**: User-generated text content on a poll
- **Mention**: Reference to another user using @username syntax

## Requirements

### Requirement 1: Poll Management

**User Story:** As a user, I want to create and manage prediction markets, so that I can pose questions and enable betting on outcomes.

#### Acceptance Criteria

1. WHEN a user creates a poll with valid parameters, THE System SHALL create the poll with associated options and market
2. WHEN a user creates a poll, THE System SHALL enforce the daily creation limit (5 polls per day)
3. WHEN a user queries polls, THE System SHALL return polls filtered by category, status, and creator
4. WHEN a user requests poll details, THE System SHALL return the poll with options, current prices, and bet statistics
5. WHEN a poll's closes_at time is reached, THE System SHALL automatically update the status to closed
6. THE System SHALL require polls to have at least 2 options and at most 10 options
7. THE System SHALL validate that poll closes_at is in the future at creation time

### Requirement 2: Market Pricing

**User Story:** As a user, I want accurate market prices based on LMSR, so that I can make informed betting decisions.

#### Acceptance Criteria

1. WHEN a market is created, THE System SHALL initialize liquidity_b and shares_outstanding for all options
2. WHEN calculating option prices, THE System SHALL ensure the sum of all option prices equals 1.0 within tolerance ε
3. WHEN a user queries market prices, THE System SHALL return current prices for all options calculated via LMSR
4. WHEN a bet is placed, THE System SHALL calculate shares using the LMSR cost function
5. WHEN shares are sold, THE System SHALL calculate refund amount using the LMSR cost function
6. THE System SHALL maintain shares_outstanding equal to the sum of all user positions for each option
7. THE System SHALL ensure market prices remain between 0 and 1 for all options

### Requirement 3: Bet Placement

**User Story:** As a user, I want to place bets on poll options, so that I can participate in prediction markets.

#### Acceptance Criteria

1. WHEN a user places a bet, THE System SHALL verify the user has sufficient balance
2. WHEN a user places a bet, THE System SHALL verify the poll status is open
3. WHEN a bet is placed, THE System SHALL atomically deduct balance, record bet, update position, and update market shares
4. WHEN a bet transaction fails, THE System SHALL rollback all state changes
5. WHEN a bet is placed, THE System SHALL record a wallet transaction with type "bet"
6. WHEN a bet is placed, THE System SHALL broadcast a price update event via WebSocket
7. THE System SHALL enforce minimum bet amount of 1.0
8. THE System SHALL prevent bets that exceed user balance

### Requirement 4: Share Selling

**User Story:** As a user, I want to sell my shares before market resolution, so that I can realize gains or cut losses.

#### Acceptance Criteria

1. WHEN a user sells shares, THE System SHALL verify the user owns sufficient shares in that option
2. WHEN a user sells shares, THE System SHALL calculate refund amount using LMSR
3. WHEN shares are sold, THE System SHALL atomically credit balance, update position, and update market shares
4. WHEN a sell transaction fails, THE System SHALL rollback all state changes
5. WHEN shares are sold, THE System SHALL record a wallet transaction with type "refund"
6. WHEN shares are sold, THE System SHALL broadcast a price update event via WebSocket

### Requirement 5: Market Resolution

**User Story:** As an admin, I want to resolve markets and distribute payouts, so that winners receive their earnings.

#### Acceptance Criteria

1. WHEN an admin resolves a poll, THE System SHALL verify the poll status is closed
2. WHEN an admin resolves a poll, THE System SHALL verify closes_at has passed
3. WHEN a poll is resolved, THE System SHALL set the winning_option_id
4. WHEN a poll is resolved, THE System SHALL calculate payouts as shares * 1.0 for winning positions
5. WHEN a poll is resolved, THE System SHALL credit each winner's balance and record wallet transactions
6. WHEN a poll is resolved, THE System SHALL create notifications for all winners
7. WHEN a poll is resolved, THE System SHALL update user statistics (streaks, accuracy)
8. WHEN a poll is resolved, THE System SHALL update status to resolved
9. WHEN a poll is resolved, THE System SHALL broadcast a resolution event via WebSocket
10. THE System SHALL ensure total payouts equal the sum of winning shares

### Requirement 6: Market Suspension and Cancellation

**User Story:** As an admin, I want to suspend or cancel markets, so that I can handle policy violations or errors.

#### Acceptance Criteria

1. WHEN an admin suspends a poll, THE System SHALL update status to suspended
2. WHEN an admin suspends a poll, THE System SHALL prevent new bets
3. WHEN an admin cancels a poll, THE System SHALL refund all bets at original bet amounts
4. WHEN a poll is cancelled, THE System SHALL update all positions to zero shares
5. WHEN a poll is cancelled, THE System SHALL record wallet transactions with type "refund"
6. WHEN a poll is cancelled, THE System SHALL create notifications for all participants
7. WHEN a poll is cancelled, THE System SHALL update status to cancelled

### Requirement 7: User Positions and Portfolio

**User Story:** As a user, I want to view my positions and portfolio, so that I can track my investments.

#### Acceptance Criteria

1. WHEN a user queries positions, THE System SHALL return all active positions with share counts
2. WHEN a user queries positions, THE System SHALL calculate current value using current market prices
3. WHEN a user queries positions, THE System SHALL calculate profit/loss as current_value - amount_spent
4. WHEN a user queries positions, THE System SHALL include poll details and status
5. THE System SHALL maintain position records with option_shares and option_spent as JSON objects

### Requirement 8: Challenge Creation

**User Story:** As a user, I want to create challenges, so that I can wager directly with other users.

#### Acceptance Criteria

1. WHEN a user creates a direct challenge, THE System SHALL require an opponent_id
2. WHEN a user creates an open challenge, THE System SHALL set is_open to true
3. WHEN a challenge is created, THE System SHALL verify the creator has sufficient balance
4. WHEN a challenge is created, THE System SHALL set status to pending
5. WHEN a challenge is created, THE System SHALL set expires_at based on expiration period
6. WHEN a direct challenge is created, THE System SHALL create a notification for the opponent
7. THE System SHALL enforce minimum challenge amount of 1.0
8. WHERE a challenge is linked to a poll, THE System SHALL verify the poll exists and is open

### Requirement 9: Challenge Acceptance

**User Story:** As a user, I want to accept challenges, so that I can participate in wagers.

#### Acceptance Criteria

1. WHEN a user accepts a challenge, THE System SHALL verify the challenge status is pending
2. WHEN a user accepts a challenge, THE System SHALL verify the challenge has not expired
3. WHEN a user accepts a challenge, THE System SHALL verify both users have sufficient balance
4. WHEN a challenge is accepted, THE System SHALL atomically deduct amount from both users
5. WHEN a challenge is accepted, THE System SHALL record wallet transactions for both users
6. WHEN a challenge is accepted, THE System SHALL update status to accepted
7. WHEN a challenge is accepted, THE System SHALL create a notification for the creator
8. WHEN an acceptance transaction fails, THE System SHALL rollback all state changes

### Requirement 10: Challenge Resolution

**User Story:** As a user, I want challenges to be resolved, so that winners receive payouts.

#### Acceptance Criteria

1. WHEN a challenge is resolved, THE System SHALL verify the challenge status is accepted
2. WHEN a poll-linked challenge is resolved, THE System SHALL use the poll's winning option to determine the winner
3. WHEN a manual challenge is resolved, THE System SHALL accept the winner_id from the request
4. WHEN a challenge is resolved, THE System SHALL credit the winner with 2 * amount
5. WHEN a challenge is resolved, THE System SHALL record a wallet transaction for the winner
6. WHEN a challenge is resolved, THE System SHALL update status to resolved
7. WHEN a challenge is resolved, THE System SHALL create notifications for both participants
8. THE System SHALL ensure the winner receives exactly 2 * challenge amount

### Requirement 11: Challenge Cancellation

**User Story:** As a user, I want to cancel pending challenges, so that I can withdraw unaccepted wagers.

#### Acceptance Criteria

1. WHEN a user cancels a challenge, THE System SHALL verify the challenge status is pending
2. WHEN a user cancels a challenge, THE System SHALL verify the user is the creator
3. WHEN a challenge is cancelled, THE System SHALL update status to cancelled
4. WHEN a challenge is cancelled, THE System SHALL create a notification for the opponent (if direct)
5. WHEN a challenge expires, THE System SHALL automatically update status to expired

### Requirement 12: Comments and Replies

**User Story:** As a user, I want to comment on polls, so that I can discuss predictions with others.

#### Acceptance Criteria

1. WHEN a user creates a comment, THE System SHALL associate it with the specified poll
2. WHEN a user creates a reply, THE System SHALL set parent_id to the parent comment
3. WHEN comments are queried, THE System SHALL return a tree structure with nested replies
4. WHEN a comment contains @mentions, THE System SHALL parse and extract mentioned usernames
5. WHEN a comment with @mentions is created, THE System SHALL create notifications for mentioned users
6. THE System SHALL validate comment content is not empty

### Requirement 13: Comment Likes

**User Story:** As a user, I want to like comments, so that I can show appreciation for insightful contributions.

#### Acceptance Criteria

1. WHEN a user likes a comment, THE System SHALL create a like record if none exists
2. WHEN a user unlikes a comment, THE System SHALL delete the like record if it exists
3. WHEN comments are queried, THE System SHALL include like counts for each comment
4. WHEN comments are queried, THE System SHALL indicate which comments the requesting user has liked

### Requirement 14: Wallet Balance and Transactions

**User Story:** As a user, I want to view my balance and transaction history, so that I can track my funds.

#### Acceptance Criteria

1. WHEN a user queries balance, THE System SHALL return the current balance
2. WHEN a user queries transactions, THE System SHALL return transactions ordered by created_at descending
3. WHEN a user queries transactions, THE System SHALL support filtering by transaction_type
4. WHEN a user queries transactions, THE System SHALL support pagination
5. THE System SHALL ensure balance equals initial_balance plus sum of all transaction amounts
6. THE System SHALL enforce balance >= 0 at all times

### Requirement 15: M-Pesa Deposits

**User Story:** As a user, I want to deposit funds via M-Pesa, so that I can participate in markets.

#### Acceptance Criteria

1. WHEN a user initiates a deposit, THE System SHALL validate the phone number format
2. WHEN a user initiates a deposit, THE System SHALL validate the amount is positive
3. WHEN a deposit is initiated, THE System SHALL create a pending M-Pesa transaction record
4. WHEN a deposit is initiated, THE System SHALL call the Daraja API STK Push endpoint
5. WHEN the STK Push succeeds, THE System SHALL store the CheckoutRequestID
6. WHEN the STK Push succeeds, THE System SHALL return the CheckoutRequestID to the user
7. WHEN the M-Pesa callback is received, THE System SHALL verify the callback signature
8. WHEN the M-Pesa callback indicates success, THE System SHALL credit the user balance
9. WHEN the M-Pesa callback indicates success, THE System SHALL record a wallet transaction with type "deposit"
10. WHEN the M-Pesa callback indicates success, THE System SHALL update the M-Pesa transaction status to completed
11. WHEN the M-Pesa callback indicates failure, THE System SHALL update the M-Pesa transaction status to failed
12. WHEN a user queries deposit status, THE System SHALL return the current status by CheckoutRequestID

### Requirement 16: Notifications

**User Story:** As a user, I want to receive notifications, so that I stay informed about platform events.

#### Acceptance Criteria

1. WHEN a user queries notifications, THE System SHALL return notifications ordered by created_at descending
2. WHEN a user queries notifications, THE System SHALL support pagination
3. WHEN a user marks a notification as read, THE System SHALL update is_read to true
4. WHEN a user marks all notifications as read, THE System SHALL update all unread notifications to read
5. WHEN a user queries unread count, THE System SHALL return the count of unread notifications
6. WHEN a notification is created, THE System SHALL send it via WebSocket if the user is connected
7. THE System SHALL create notifications for: bet wins, challenge invites, challenge acceptances, challenge resolutions, @mentions, poll resolutions

### Requirement 17: Authentication

**User Story:** As a user, I want secure authentication, so that my account and funds are protected.

#### Acceptance Criteria

1. WHEN a user logs in with valid credentials, THE System SHALL generate a JWT access token with 30-minute expiry
2. WHEN a user logs in with valid credentials, THE System SHALL generate a refresh token with 7-day expiry
3. WHEN a user accesses a protected endpoint, THE System SHALL validate the JWT signature and expiry
4. WHEN a JWT is expired, THE System SHALL return 401 Unauthorized
5. WHEN a user refreshes tokens, THE System SHALL validate the refresh token and issue new tokens
6. WHEN a user authenticates with a wallet, THE System SHALL verify the ed25519 signature
7. WHEN a user authenticates with a wallet, THE System SHALL verify the nonce is unused and not expired
8. WHEN a nonce is used, THE System SHALL mark it as used to prevent replay attacks
9. THE System SHALL enforce nonce expiry of 5 minutes
10. THE System SHALL store refresh tokens in Redis with 7-day TTL

### Requirement 18: Authorization

**User Story:** As a system, I want to enforce access control, so that users can only perform authorized actions.

#### Acceptance Criteria

1. WHEN a user accesses an admin endpoint, THE System SHALL verify is_staff is true
2. WHEN a user accesses their own resources, THE System SHALL verify the user_id matches the JWT claims
3. WHEN a user attempts unauthorized access, THE System SHALL return 403 Forbidden
4. WHEN a user is inactive (is_active = false), THE System SHALL deny access to protected endpoints

### Requirement 19: WebSocket Connections

**User Story:** As a user, I want real-time updates, so that I see market changes immediately.

#### Acceptance Criteria

1. WHEN a user connects via WebSocket, THE System SHALL authenticate using the JWT query parameter
2. WHEN a WebSocket connection is established, THE System SHALL register the connection in Redis
3. WHEN a user disconnects, THE System SHALL remove the connection from Redis
4. WHEN a connection is idle for 60 seconds, THE System SHALL send a ping message
5. WHEN a connection fails to respond to ping, THE System SHALL close the connection
6. THE System SHALL limit each user to 5 concurrent WebSocket connections
7. THE System SHALL require WebSocket authentication within 30 seconds of connection

### Requirement 20: WebSocket Subscriptions and Broadcasting

**User Story:** As a user, I want to subscribe to specific polls, so that I receive relevant updates.

#### Acceptance Criteria

1. WHEN a user subscribes to a poll, THE System SHALL add the connection to the poll's subscriber list
2. WHEN a bet is placed on a poll, THE System SHALL broadcast a price update to all poll subscribers
3. WHEN a poll is resolved, THE System SHALL broadcast a resolution event to all poll subscribers
4. WHEN a comment is added to a poll, THE System SHALL broadcast a comment event to all poll subscribers
5. WHEN a notification is created for a user, THE System SHALL send it to all of the user's active connections
6. WHEN a challenge is created for a user, THE System SHALL send a notification to the user's connections
7. THE System SHALL use Redis pub/sub for broadcasting across multiple server instances

### Requirement 21: Rate Limiting

**User Story:** As a system, I want to rate limit requests, so that I prevent abuse and ensure fair usage.

#### Acceptance Criteria

1. WHEN an anonymous user makes requests, THE System SHALL enforce a limit of 60 requests per minute
2. WHEN an authenticated user makes requests, THE System SHALL enforce a limit of 300 requests per minute
3. WHEN a user exceeds the rate limit, THE System SHALL return 429 Too Many Requests
4. WHEN a user accesses auth endpoints, THE System SHALL enforce a limit of 10 requests per minute
5. WHEN a user accesses trading endpoints, THE System SHALL enforce a limit of 30 requests per minute
6. WHEN a user initiates M-Pesa deposits, THE System SHALL enforce a limit of 5 requests per minute

### Requirement 22: Input Validation

**User Story:** As a system, I want to validate all inputs, so that I prevent invalid data and security vulnerabilities.

#### Acceptance Criteria

1. WHEN a user provides a username, THE System SHALL validate length is between 3 and 30 characters
2. WHEN a user provides an email, THE System SHALL validate it matches email format
3. WHEN a user provides a phone number, THE System SHALL validate it matches the expected format
4. WHEN a user provides a bet amount, THE System SHALL validate it is positive and within limits
5. WHEN a user provides poll options, THE System SHALL validate count is between 2 and 10
6. WHEN a user provides comment content, THE System SHALL sanitize it to prevent XSS attacks
7. THE System SHALL reject requests with invalid or missing required fields

### Requirement 23: Error Handling

**User Story:** As a system, I want to handle errors gracefully, so that users receive helpful feedback.

#### Acceptance Criteria

1. WHEN a database error occurs, THE System SHALL return 500 Internal Server Error
2. WHEN a user provides invalid input, THE System SHALL return 400 Bad Request with error details
3. WHEN a resource is not found, THE System SHALL return 404 Not Found
4. WHEN a user lacks authorization, THE System SHALL return 403 Forbidden
5. WHEN authentication fails, THE System SHALL return 401 Unauthorized
6. WHEN a conflict occurs (e.g., duplicate), THE System SHALL return 409 Conflict
7. THE System SHALL log all errors with request context for debugging
8. THE System SHALL not expose stack traces or sensitive information in error responses

### Requirement 24: Database Transactions

**User Story:** As a system, I want atomic database operations, so that data remains consistent.

#### Acceptance Criteria

1. WHEN a bet is placed, THE System SHALL execute all database operations within a transaction
2. WHEN a challenge is accepted, THE System SHALL execute all database operations within a transaction
3. WHEN a poll is resolved, THE System SHALL execute all database operations within a transaction
4. WHEN any operation in a transaction fails, THE System SHALL rollback all changes
5. WHEN a transaction succeeds, THE System SHALL commit all changes atomically
6. THE System SHALL use SELECT FOR UPDATE for critical sections (balance updates, market updates)

### Requirement 25: Caching

**User Story:** As a system, I want to cache frequently accessed data, so that I improve performance.

#### Acceptance Criteria

1. WHEN a user session is created, THE System SHALL cache it in Redis with 7-day TTL
2. WHEN poll details are queried, THE System SHALL cache them in Redis with 30-second TTL
3. WHEN market prices are queried, THE System SHALL cache them in Redis with 5-second TTL
4. WHEN cached data is updated, THE System SHALL invalidate the cache
5. WHEN cache is unavailable, THE System SHALL fall back to database queries

### Requirement 26: Logging and Observability

**User Story:** As a developer, I want structured logging, so that I can debug issues and monitor performance.

#### Acceptance Criteria

1. WHEN a request is received, THE System SHALL log the request ID, method, path, and user ID
2. WHEN a request completes, THE System SHALL log the response status and duration
3. WHEN an error occurs, THE System SHALL log the error message, stack trace, and request context
4. WHEN a security event occurs, THE System SHALL log it with severity level
5. THE System SHALL use structured logging with JSON format
6. THE System SHALL include trace IDs for distributed tracing

### Requirement 27: Configuration Management

**User Story:** As a developer, I want externalized configuration, so that I can deploy to different environments.

#### Acceptance Criteria

1. THE System SHALL load configuration from environment variables
2. THE System SHALL support .env files for local development
3. THE System SHALL validate required configuration at startup
4. THE System SHALL fail fast if required configuration is missing
5. THE System SHALL not log or expose sensitive configuration values

### Requirement 28: Database Connection Pooling

**User Story:** As a system, I want connection pooling, so that I efficiently manage database connections.

#### Acceptance Criteria

1. THE System SHALL maintain a connection pool with maximum 20 connections
2. THE System SHALL reuse connections from the pool for database queries
3. WHEN the pool is exhausted, THE System SHALL queue requests until a connection is available
4. WHEN a connection is idle for 10 minutes, THE System SHALL close it
5. THE System SHALL validate connections before use

### Requirement 29: Password Security

**User Story:** As a system, I want secure password handling, so that user credentials are protected.

#### Acceptance Criteria

1. WHEN a user registers, THE System SHALL hash the password using Argon2
2. WHEN a user logs in, THE System SHALL verify the password against the Argon2 hash
3. THE System SHALL never store passwords in plaintext
4. THE System SHALL never log or expose password hashes
5. THE System SHALL enforce minimum password length of 8 characters

### Requirement 30: CORS Configuration

**User Story:** As a system, I want CORS support, so that the frontend can make cross-origin requests.

#### Acceptance Criteria

1. THE System SHALL allow cross-origin requests from configured frontend origins
2. THE System SHALL include appropriate CORS headers in responses
3. THE System SHALL handle preflight OPTIONS requests
4. THE System SHALL restrict CORS origins in production
5. THE System SHALL allow credentials in CORS requests
