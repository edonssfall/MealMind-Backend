# MealMind Backend

## API Endpoints

### Authentication

#### Register

`http://localhost:8080/auth/register`

`{"email":"user@example.com","password":"password123"}`

Response:
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": "uuid",
    "email": "user@example.com"
  }
}
```

#### Login

`http://localhost:8080/auth/login`

`{"email":"user@example.com","password":"password123"}`

#### Refresh Token

`http://localhost:8080/auth/refresh`

`{"refresh_token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."}`

### Protected Endpoints

#### Get Current User

`http://localhost:8080/me`

`"Authorization: Bearer YOUR_ACCESS_TOKEN"`

Response:
```json
{
  "id": "uuid",
  "email": "user@example.com"
}
```

---

Rust backend with Axum, PostgreSQL, JWT authentication, and refresh tokens.

## Quick Start

```bash
# Start services
docker compose up -d

# Check logs
docker compose logs -f app
```

## Configuration

Environment variables:
- `JWT_SECRET`: Secret for signing tokens
- `JWT_TTL_MINUTES`: Access token expiry (default: 60)
- `JWT_REFRESH_TTL_MINUTES`: Refresh token expiry (default: 20160 = 14 days)
- `DATABASE_URL`: PostgreSQL connection string
- `LOG_FORMAT=json`: Enable JSON logging

## Development

```bash
# Build and run
cargo run

# With custom env
RUST_LOG=debug cargo run
```