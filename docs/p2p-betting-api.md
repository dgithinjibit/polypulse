# P2P Betting API Reference

All endpoints are prefixed with `/api/v1`. Authenticated endpoints require `Authorization: Bearer <jwt>`.

---

## Endpoints

### Create a bet

```
POST /api/v1/p2p-bets
Authorization: Bearer <jwt>
```

**Request**

```json
{
  "question": "Will it rain in Nairobi tomorrow?",
  "stake_amount": 1000000,
  "end_time": "2024-12-31T18:00:00Z"
}
```

| Field | Type | Description |
|---|---|---|
| `question` | string | Bet question (10–200 chars, must end with `?`) |
| `stake_amount` | integer | Creator stake in stroops (1 XLM = 10,000,000 stroops) |
| `end_time` | ISO 8601 datetime | Must be in the future |

**Response `201`**

```json
{
  "bet_id": 42,
  "shareable_url": "will-it-rain-in-nairobi-tomorrow-creator-alice.polypulse.co.ke/bet/enc_abc123"
}
```

---

### List bets

```
GET /api/v1/p2p-bets
```

**Query parameters**

| Param | Values | Default | Description |
|---|---|---|---|
| `status` | `All`, `Active`, `Ending Soon`, `Ended` | — | Filter by status |
| `search` | string | — | Full-text search on question |
| `sort` | `newest`, `volume`, `liquidity`, `ending_soon` | `newest` | Sort order |
| `page` | integer | `1` | Page number |
| `limit` | integer (1–100) | `20` | Results per page |

**Response `200`**

```json
[
  {
    "id": 42,
    "creator_id": 1,
    "question": "Will it rain in Nairobi tomorrow?",
    "stake_amount": 1000000,
    "end_time": "2024-12-31T18:00:00Z",
    "state": "Active",
    "created_at": "2024-12-01T10:00:00Z",
    "shareable_url": "will-it-rain...",
    "verified_outcome": null,
    "disputed": false
  }
]
```

---

### Get bet details

```
GET /api/v1/p2p-bets/:id
```

**Response `200`** — same shape as a single item from the list response.

**Response `404`**

```json
{ "error": "Bet not found" }
```

---

### Resolve shareable URL

```
GET /api/v1/p2p-bets/share/:encrypted_id
```

Decrypts the encrypted bet ID from a shareable URL and returns the bet details. Same response shape as `GET /api/v1/p2p-bets/:id`.

---

### Join a bet

```
POST /api/v1/p2p-bets/:id/join
Authorization: Bearer <jwt>
```

**Request**

```json
{
  "position": true,
  "stake": 500000
}
```

| Field | Type | Description |
|---|---|---|
| `position` | boolean | `true` = Yes, `false` = No |
| `stake` | integer | Stake in stroops |

**Response `200`** — empty body on success.

**Errors**

| Status | Reason |
|---|---|
| `400` | Bet not accepting participants, already ended, or already joined |
| `404` | Bet not found |

---

### Cancel a bet

```
POST /api/v1/p2p-bets/:id/cancel
Authorization: Bearer <jwt>
```

Only the creator can cancel, and only if no participants have joined yet.

**Response `200`** — empty body on success.

---

### Report outcome

```
POST /api/v1/p2p-bets/:id/report-outcome
Authorization: Bearer <jwt>
```

Called by the first participant to report the real-world result after the bet ends.

**Request**

```json
{ "outcome": true }
```

`true` = Yes outcome, `false` = No outcome.

**Response `200`** — empty body on success.

---

### Confirm outcome

```
POST /api/v1/p2p-bets/:id/confirm-outcome
Authorization: Bearer <jwt>
```

Called by remaining participants to agree or disagree with the reported outcome.

- If all participants agree → bet moves to `Verified` and payout is executed automatically.
- If any participant disagrees → bet moves to `Disputed` and funds are locked.

**Request**

```json
{ "outcome": true }
```

**Response `200`** — empty body on success.

---

### Get outcome status

```
GET /api/v1/p2p-bets/:id/outcome-status
Authorization: Bearer <jwt>
```

**Response `200`**

```json
[
  {
    "reporter_id": 2,
    "outcome": true,
    "reported_at": "2024-12-31T19:00:00Z"
  },
  {
    "reporter_id": 3,
    "outcome": true,
    "reported_at": "2024-12-31T19:05:00Z"
  }
]
```

---

### Generate shareable URL

```
POST /api/v1/p2p-bets/:id/generate-url
Authorization: Bearer <jwt>
```

Regenerates the shareable URL for a bet. Useful if the original URL was lost.

**Response `200`**

```json
{ "shareable_url": "will-it-rain-creator-alice.polypulse.co.ke/bet/enc_abc123" }
```

---

### My positions

```
GET /api/v1/p2p-bets/my-positions
Authorization: Bearer <jwt>
```

Returns all bets the authenticated user has joined as a participant.

**Response `200`**

```json
[
  {
    "bet_id": 42,
    "question": "Will it rain in Nairobi tomorrow?",
    "position": true,
    "stake": 500000,
    "state": "Active",
    "end_time": "2024-12-31T18:00:00Z",
    "joined_at": "2024-12-01T11:00:00Z"
  }
]
```

---

### My bets

```
GET /api/v1/p2p-bets/my-bets
Authorization: Bearer <jwt>
```

Returns all bets created by the authenticated user.

**Response `200`** — array of bet objects (same shape as list response).

---

## Bet states

| State | Description |
|---|---|
| `Created` | Bet created, no participants yet |
| `Active` | At least one participant has joined |
| `Ended` | End time passed, outcome reporting in progress |
| `Verified` | All participants agreed on outcome, payout executed |
| `Disputed` | Participants disagreed; funds locked pending admin resolution |
| `Paid` | Payout complete |
| `Cancelled` | Creator cancelled before any participants joined |

---

## WebSocket events

Connect at `wss://api.polypulse.co.ke/ws/p2p-bets/:bet_id?token=<jwt>`.

| Event | Payload | Description |
|---|---|---|
| `participant_joined` | `{ user_id, position, stake }` | New participant joined |
| `outcome_reported` | `{ user_id, outcome }` | First outcome report submitted |
| `outcome_verified` | `{ outcome }` | All participants agreed |
| `disputed` | `{}` | Participants disagreed |
| `paid` | `{ winners: [user_id] }` | Payout executed |
| `cancelled` | `{}` | Bet cancelled by creator |

---

## Error responses

All errors follow this shape:

```json
{ "error": "Human-readable message" }
```

| Status | Meaning |
|---|---|
| `400` | Bad request / validation error |
| `401` | Missing or invalid JWT |
| `403` | Forbidden (e.g. not the creator) |
| `404` | Resource not found |
| `500` | Internal server error |
