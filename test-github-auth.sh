#!/bin/bash
# Script to demonstrate GitHub authentication testing

echo "🧪 crowdcontrol GitHub Authentication Test Runner"
echo "=================================================="
echo ""

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

echo "✅ Docker is running"

# Check if the crowdcontrol:latest image exists
if ! docker image inspect crowdcontrol:latest >/dev/null 2>&1; then
    echo "❌ crowdcontrol:latest image not found. Please build it first:"
    echo "   docker build -t crowdcontrol:latest ./container/"
    exit 1
fi

echo "✅ crowdcontrol:latest image found"
echo ""

# Quick authentication status check
echo "🔍 Running quick GitHub authentication check..."
cargo test --test github_end_to_end_test test_quick_github_auth_check -- --nocapture --ignored

echo ""
echo "📋 Available GitHub Authentication Tests:"
echo ""
echo "1. Basic End-to-End Test (shows failure without GitHub auth):"
echo "   cargo test --test github_end_to_end_test test_github_auth_end_to_end_workflow -- --nocapture --ignored"
echo ""
echo "2. Success Test (requires GitHub credentials):"
echo "   cargo test --test github_end_to_end_test test_github_auth_success_workflow -- --nocapture --ignored"
echo ""
echo "3. Real Repository Test (requires GitHub credentials and test repo):"
echo "   export GITHUB_TEST_REPO=https://github.com/yourusername/test-repo.git"
echo "   cargo test --test github_end_to_end_test test_real_github_operations -- --nocapture --ignored"
echo ""

# Check if GitHub credentials are configured
if [ -n "$GITHUB_INSTALLATION_TOKEN" ] || [ -n "$GITHUB_APP_ID" ]; then
    echo "✅ GitHub credentials detected!"
    echo "🚀 You can run the success tests:"
    echo ""
    echo "   # Test with existing credentials:"
    echo "   cargo test --test github_end_to_end_test test_github_auth_success_workflow -- --nocapture --ignored"
    echo ""
    if [ -n "$GITHUB_TEST_REPO" ]; then
        echo "   # Test with real repository operations:"
        echo "   cargo test --test github_end_to_end_test test_real_github_operations -- --nocapture --ignored"
    else
        echo "   # To test real repository operations, set:"
        echo "   export GITHUB_TEST_REPO=https://github.com/yourusername/test-repo.git"
        echo "   cargo test --test github_end_to_end_test test_real_github_operations -- --nocapture --ignored"
    fi
else
    echo "ℹ️  No GitHub credentials configured (this is normal for testing the failure case)"
    echo "🧪 Running the end-to-end workflow test to demonstrate the failure case..."
    echo ""

    # Run the main test that shows what happens without GitHub auth
    cargo test --test github_end_to_end_test test_github_auth_end_to_end_workflow -- --nocapture --ignored
fi

echo ""
echo "📖 For setup instructions, see:"
echo "   docs/github-app-setup.md - Individual setup"
echo "   docs/github-organization-setup.md - Organization setup"
