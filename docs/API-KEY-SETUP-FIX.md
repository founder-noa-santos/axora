# API Key Configuration Fix

## Problem

After fixing the SQLite corruption issue, users encountered a new error:

```
Error: failed to read api key file /Users/noasantos/Documents/openakta/aktacode/.openakta/secrets/alibaba-coding.key

Caused by:
    No such file or directory (os error 2)
```

## Root Cause

The default configuration in `openakta.toml` uses Alibaba Cloud's Coding Plan API, which requires an API key stored in a file. The file didn't exist because:

1. The `.openakta/` directory was recreated after the SQLite corruption fix
2. API key files are intentionally excluded from version control (security)
3. No setup instructions were visible to new users

## Solution

### 1. Improved Error Message

Updated `crates/openakta-core/src/config_resolve.rs` to provide actionable error messages:

**Before:**
```
Error: failed to read api key file /path/to/file
Caused by: No such file or directory (os error 2)
```

**After:**
```
Error: API key file not found at: /path/to/file

To fix this:
1. Create the file: mkdir -p /path/to/secrets
2. Add your API key: echo 'your-api-key-here' > /path/to/file

Or configure the API key directly in openakta.toml using api_key instead of api_key_file.
```

### 2. Documentation

Created `.openakta/secrets/README.md` with:
- Setup instructions for Alibaba Cloud and other providers
- Security best practices
- Troubleshooting guide
- Alternative configuration methods

### 3. Setup Script

Created `scripts/setup-keys.sh` for interactive first-time setup:

```bash
./scripts/setup-keys.sh
```

This script:
- Creates the secrets directory
- Prompts for API key with helpful instructions
- Sets proper file permissions (600)
- Provides skip option for later configuration

## Quick Start for Users

### Option 1: Interactive Setup (Recommended)

```bash
cd aktacode
./scripts/setup-keys.sh
```

### Option 2: Manual Setup

```bash
# Create directory
mkdir -p .openakta/secrets

# Add your Alibaba Cloud API key
echo 'sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' > .openakta/secrets/alibaba-coding.key

# Set secure permissions
chmod 600 .openakta/secrets/alibaba-coding.key

# Test configuration
cargo run -p openakta-cli -- do "test"
```

### Option 3: Use Different Provider

Edit `openakta.toml` to use a provider you already have configured:

```toml
[providers]
default_cloud_instance = "openai"  # or "deepseek", etc.

[providers.instances.openai.secret]
api_key_file = ".openakta/secrets/openai.key"
```

Then create the corresponding key file.

### Option 4: Environment Variable (Development Only)

For quick testing, you can temporarily embed the key in `openakta.toml`:

```toml
[providers.instances.alibaba-coding.secret]
api_key = "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
```

**Warning:** Never commit this to version control!

## Files Changed

1. **`crates/openakta-core/src/config_resolve.rs`**
   - Enhanced error handling in `resolve_secret_ref()`
   - Added helpful setup instructions in error message

2. **`.openakta/secrets/README.md`** (new)
   - Comprehensive setup guide
   - Security best practices
   - Multi-provider instructions

3. **`scripts/setup-keys.sh`** (new)
   - Interactive setup script
   - First-time user experience

## Testing

Verify the fix works:

```bash
# Should show improved error message
cargo run -p openakta-cli -- do "test"

# Run setup script
./scripts/setup-keys.sh

# Should work with valid key
cargo run -p openakta-cli -- do "Response apenas: Ok"
```

## Related Issues

This fix complements the SQLite corruption recovery fix in `docs/SQLITE-CORRUPTION-RECOVERY.md`. Both improvements make the runtime more resilient and user-friendly.

## Next Steps

Consider adding:
- [ ] Support for environment variable fallbacks (e.g., `DASHSCOPE_API_KEY`)
- [ ] Wizard-style interactive setup for multiple providers
- [ ] Key validation before saving (check format, test API call)
- [ ] Integration with system keychain (macOS Keychain, Windows Credential Manager)
