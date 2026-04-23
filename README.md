# Lettro

A backend API for sending email newsletters to confirmed subscribers, built in Rust.

## What it does

Blog authors can publish newsletter issues via a single API call. The API delivers them to every subscriber who confirmed their email address. Unconfirmed subscribers are ignored.

Subscribers sign up via a form, receive a confirmation email, and click a link to confirm. Only then do they appear in the delivery list.

## Stack

- **Axum** — HTTP server
- **sqlx + PostgreSQL** — database and migrations
- **Postmark** — email delivery
- **Argon2** — password hashing
- **tracing + tracing-subscriber** — structured logging

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health_check` | Check if the server is running |
| POST | `/subscriptions` | Subscribe with name and email |
| GET | `/subscriptions/confirm` | Confirm a subscription via token |
| POST | `/newsletters` | Publish a newsletter issue (requires Basic auth) |

## Running locally

You need Rust, Docker, and the sqlx CLI installed.

Start the database:

```bash
./scripts/init_db.sh
```

Run the app:

```bash
cargo run
```

Run tests:

```bash
cargo test
```

## Configuration

Settings live in `configuration.yaml`. The app reads from environment variables with the `APP__` prefix, so you can override any value at runtime:

```bash
APP_APPLICATION__PORT=5001 cargo run
APP_DATABASE__PASSWORD=secret cargo run
```

## Authentication

`POST /newsletters` uses HTTP Basic authentication. Pass a username and password registered in the `users` table. Credentials must be sent over HTTPS.

```bash
curl -u username:password \
  -H "Content-Type: application/json" \
  -d '{"title":"Hello","content":{"text":"Hi","html":"<p>Hi</p>"}}' \
  https://your-domain.com/newsletters
```

## Database migrations

```bash
sqlx migrate run
```

To regenerate the offline query cache after changing queries:

```bash
cargo sqlx prepare -- --lib
```
