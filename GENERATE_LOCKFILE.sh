#!/bin/bash
# Script to generate Cargo.lock and SQLx offline data
# Run this BEFORE deploying to Railway

set -e

echo "========================================="
echo "Production Build Preparation"
echo "========================================="
echo ""

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found!"
    echo "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if in project root
if [ ! -f "Cargo.toml" ]; then
    echo "ERROR: Cargo.toml not found. Are you in the project root?"
    exit 1
fi

echo "✓ Cargo found: $(cargo --version)"
echo ""

# Step 1: Generate Cargo.lock
echo "Step 1/4: Generating Cargo.lock..."
if [ -f "Cargo.lock" ]; then
    echo "  Cargo.lock already exists, updating..."
    cargo update
else
    cargo generate-lockfile
fi
echo "✓ Cargo.lock generated"
ls -lh Cargo.lock
echo ""

# Step 2: Check database connection
echo "Step 2/4: Checking database connection..."
if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL not set!"
    echo "Set it with: export DATABASE_URL='postgresql://user:password@localhost/mindguard'"
    exit 1
fi
echo "✓ DATABASE_URL set"
echo ""

# Step 3: Run migrations
echo "Step 3/4: Running migrations..."
if ! command -v sqlx &> /dev/null; then
    echo "  Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

sqlx database create 2>/dev/null || echo "  Database already exists"
sqlx migrate run
echo "✓ Migrations applied"
echo ""

# Step 4: Generate SQLx offline data
echo "Step 4/4: Generating SQLx offline query data..."
cargo sqlx prepare
echo "✓ SQLx metadata generated"
ls -lh .sqlx
echo ""

# Verify offline build works
echo "========================================="
echo "Verification: Testing offline build..."
echo "========================================="
export SQLX_OFFLINE=true
cargo check
echo "✓ Offline build works!"
echo ""

# Summary
echo "========================================="
echo "SUCCESS! Ready for production deploy"
echo "========================================="
echo ""
echo "Files generated:"
echo "  ✓ Cargo.lock - $(ls -lh Cargo.lock | awk '{print $5}')"
echo "  ✓ .sqlx - $(ls -ld .sqlx | awk '{print $5}')"
echo ""
echo "Next steps:"
echo "  1. git add Cargo.lock .sqlx"
echo "  2. git commit -m 'Add build artifacts for production'"
echo "  3. git push origin main"
echo ""
echo "Railway will now build deterministically!"
