#!/bin/bash
# OpsLab Mindguard - Railway Environment Setup Script

echo "🚀 Setting up Railway environment variables..."

# Get Railway DATABASE_URL (this is auto-provided by Railway Postgres)
echo "📊 DATABASE_URL will be automatically provided by Railway Postgres service"

# Set encryption keys (GENERATED SECURELY)
echo "🔐 Setting encryption keys..."
railway variables --set APP_ENC_KEY="QSCi5HDSFq691xbRmGYQpqJupG4kRJf9s8968tAbDvQ="
railway variables --set SESSION_KEY="8TwaOtZBTGGxUlsy+v0+5JvWTIkOaLUtZpH4MaFfhkM="

# Set Telegram configuration (PLACEHOLDERS - YOU MUST REPLACE THESE)
echo "🤖 Setting Telegram bot configuration..."
echo "⚠️  WARNING: You MUST update these with real values:"
echo "   - TELEGRAM_BOT_TOKEN: Get from @BotFather"
echo "   - BOT_USERNAME: Your bot's username (without @)"
echo "   - ADMIN_TELEGRAM_ID: Your Telegram user ID"
echo "   - JANE_TELEGRAM_ID: Jane's Telegram user ID"
echo ""
railway variables --set TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN_FROM_BOTFATHER"
railway variables --set BOT_USERNAME="YOUR_BOT_USERNAME"
railway variables --set ADMIN_TELEGRAM_ID="YOUR_TELEGRAM_ID"
railway variables --set JANE_TELEGRAM_ID="JANE_TELEGRAM_ID"

# Set OpenAI API key (PLACEHOLDER - YOU MUST REPLACE THIS)
echo "🧠 Setting OpenAI API key..."
echo "⚠️  WARNING: You MUST update OPENAI_API_KEY with your real API key"
echo ""
railway variables --set OPENAI_API_KEY="YOUR_OPENAI_API_KEY"

# Set server configuration
echo "⚙️  Setting server configuration..."
railway variables --set BIND_ADDR="0.0.0.0:3000"
railway variables --set RUST_LOG="info"
railway variables --set SQLX_OFFLINE="true"
railway variables --set PRODUCTION="true"

echo ""
echo "✅ Railway variables configured!"
echo ""
echo "⚠️  NEXT STEPS:"
echo "1. Update the following variables with real values using Railway dashboard or CLI:"
echo "   railway variables --set TELEGRAM_BOT_TOKEN=\"your-real-token\""
echo "   railway variables --set BOT_USERNAME=\"your_bot_username\""
echo "   railway variables --set ADMIN_TELEGRAM_ID=\"123456789\""
echo "   railway variables --set JANE_TELEGRAM_ID=\"987654321\""
echo "   railway variables --set OPENAI_API_KEY=\"sk-your-real-key\""
echo ""
echo "2. Verify DATABASE_URL is set (should be automatic from Railway Postgres):"
echo "   railway variables"
echo ""
echo "3. Deploy:"
echo "   railway up"
echo ""
