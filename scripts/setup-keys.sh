#!/bin/bash
# Quick setup script for OPENAKTA API keys
# This script helps you configure API keys for the first time

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SECRETS_DIR="$SCRIPT_DIR/.openakta/secrets"

echo "🔑 OPENAKTA API Key Setup"
echo "========================"
echo ""

# Create secrets directory
mkdir -p "$SECRETS_DIR"
echo "✓ Created secrets directory: $SECRETS_DIR"
echo ""

# Check if Alibaba key exists
ALIBABA_KEY="$SECRETS_DIR/alibaba-coding.key"
if [ ! -f "$ALIBABA_KEY" ]; then
    echo "⚠️  Alibaba Cloud API key not found"
    echo ""
    echo "To get your API key:"
    echo "  1. Visit: https://dashscope.console.aliyun.com/"
    echo "  2. Sign in or create an account"
    echo "  3. Go to API Key Management"
    echo "  4. Create a new key or copy existing one"
    echo ""
    read -p "Enter your Alibaba Cloud API key (or press Enter to skip): " api_key
    
    if [ -n "$api_key" ]; then
        echo "$api_key" > "$ALIBABA_KEY"
        chmod 600 "$ALIBABA_KEY"
        echo "✓ API key saved to: $ALIBABA_KEY"
        echo ""
        echo "✅ Setup complete! You can now run:"
        echo "   cargo run -p openakta-cli -- do \"your mission\""
    else
        echo "⊘ Skipped. You can add the key later by running:"
        echo "   echo 'your-api-key' > $ALIBABA_KEY"
    fi
else
    echo "✓ Alibaba Cloud API key found"
    echo ""
    echo "✅ Configuration complete! You can now run:"
    echo "   cargo run -p openakta-cli -- do \"your mission\""
fi

echo ""
echo "📚 For more information, see: $SECRETS_DIR/README.md"
