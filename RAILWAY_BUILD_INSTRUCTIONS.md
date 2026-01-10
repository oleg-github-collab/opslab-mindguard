# Railway Build Instructions

## Setup

Railway needs DATABASE_URL during build for sqlx query verification.

### Environment Variables (Set in Railway Dashboard)

**Build-time variables:**
- `DATABASE_URL` - PostgreSQL connection string (use internal URL for faster build)

**Runtime variables:**
- `DATABASE_URL` - Same PostgreSQL URL
- `APP_ENC_KEY` - Get from RAILWAY_ENV_VARS_PRIVATE.txt
- `SESSION_KEY` - Get from RAILWAY_ENV_VARS_PRIVATE.txt
- `TELEGRAM_BOT_TOKEN` - Get from RAILWAY_ENV_VARS_PRIVATE.txt
- `OPENAI_API_KEY` - Get from RAILWAY_ENV_VARS_PRIVATE.txt
- `RUST_LOG=info`
- `PORT=3000`

### Build Settings

Railway will automatically:
1. Detect Dockerfile
2. Run `docker build`
3. Use DATABASE_URL to verify SQL queries during build
4. Deploy the built image

### Troubleshooting

If build fails with "SQLX_OFFLINE=true but no cached data":
- Ensure DATABASE_URL is set in Railway environment variables
- Railway builder needs network access to database during build
- Or generate sqlx-data.json locally and commit it

### Local Development

Generate Cargo.lock and optionally sqlx-data.json:

```bash
# Generate Cargo.lock
cargo generate-lockfile

# Optional: Generate sqlx-data.json (requires DATABASE_URL)
export DATABASE_URL="your-external-db-url"
cargo sqlx prepare --merged

# Then you can build offline
export SQLX_OFFLINE=true
cargo build --release
```

## Current Status

⚠️ **Note**: Code has compilation errors due to breaking changes in dependencies (async-openai 0.21, teloxide 0.12).
Railway build will use DATABASE_URL at build time to work around missing sqlx-data.json.

To fix compilation errors, update:
1. ChatCompletionRequestMessage API (add `role` field)
2. Teloxide Message API changes
3. Type mismatches in various handlers
