# GitHub App Setup for CrowdControl

This guide will help you set up a GitHub App to enable CrowdControl to authenticate with GitHub and appear as a distinct app in commit history and pull requests.

## Why Use a GitHub App?

When CrowdControl uses a GitHub App for authentication, all commits and pull requests will show as being created by "CrowdControl[bot]" instead of your personal GitHub account. This provides clear attribution and separation between your manual work and CrowdControl's automated changes.

## Prerequisites

- A GitHub account
- Access to create GitHub Apps (personal account or organization owner/admin permissions)
- CrowdControl installed locally

## Step 1: Create a GitHub App

1. **Navigate to GitHub App settings:**
   - For personal account: Go to GitHub → Settings → Developer settings → GitHub Apps
   - For organization: Go to your organization → Settings → Developer settings → GitHub Apps

2. **Click "New GitHub App"**

3. **Fill out the basic information:**
   - **App name**: `CrowdControl` (or `CrowdControl-YourName` if the name is taken)
   - **Description**: `AI-powered development environment manager`
   - **Homepage URL**: `https://github.com/wadefletch/crowdcontrol`
   - **Webhook URL**: Leave blank (not needed for our use case)
   - **Webhook secret**: Leave blank

4. **Configure permissions:**
   Under "Repository permissions", set the following:
   - **Contents**: `Read and write` (required for cloning repos, creating branches, pushing code)
   - **Pull requests**: `Write` (required for creating pull requests)
   - **Metadata**: `Read` (automatically selected, required for basic repository access)

5. **Where can this GitHub App be installed?**
   - Select "Only on this account" (recommended for personal use)
   - Or "Any account" if you want to share with others

6. **Click "Create GitHub App"**

## Step 2: Generate a Private Key

1. After creating the app, scroll down to the "Private keys" section
2. Click "Generate a private key"
3. A `.pem` file will be downloaded to your computer
4. **Important**: Store this file securely - you cannot regenerate the same key

## Step 3: Install the App

1. On your GitHub App's page, click "Install App" in the left sidebar
2. Choose the account/organization where you want to install it
3. Select either:
   - **All repositories** (gives CrowdControl access to all your repos)
   - **Selected repositories** (choose specific repos - recommended for testing)
4. Click "Install"

## Step 4: Get Installation Information

After installation, you'll be redirected to a URL that looks like:
```
https://github.com/settings/installations/12345678
```

The number at the end (`12345678`) is your **Installation ID** - save this number.

You can also find this by:
1. Going to GitHub → Settings → Applications → Installed GitHub Apps
2. Click "Configure" next to your CrowdControl app
3. The Installation ID is in the URL

## Step 5: Get Your App ID

1. Go back to your GitHub App's settings page
2. The **App ID** is displayed near the top of the page (it's a shorter number like `123456`)

## Step 6: Configure CrowdControl

You now have three pieces of information needed for CrowdControl:

1. **App ID** (from Step 5)
2. **Installation ID** (from Step 4)  
3. **Private Key file** (from Step 2)

### Option A: Environment Variables (Recommended)

Set these environment variables in your shell:

```bash
export GITHUB_APP_ID="123456"
export GITHUB_INSTALLATION_ID="12345678"
export GITHUB_PRIVATE_KEY_PATH="/path/to/your/crowdcontrol-private-key.pem"
```

Add these to your shell profile (`.bashrc`, `.zshrc`, etc.) to make them permanent.

### Option B: Installation Token (Simpler, but less secure)

Instead of App ID + private key, you can generate a direct installation token:

1. Install the GitHub CLI: `gh auth login`
2. Generate an installation token:
   ```bash
   gh api /app/installations/YOUR_INSTALLATION_ID/access_tokens -X POST
   ```
3. Use the token directly:
   ```bash
   export GITHUB_INSTALLATION_TOKEN="ghs_your_generated_token"
   ```

**Note**: Installation tokens expire after 1 hour, so this method requires manual renewal.

## Step 7: Test the Setup

1. Create a new CrowdControl agent:
   ```bash
   crowdcontrol new test-github-auth https://github.com/yourusername/test-repo.git
   ```

2. Connect to the agent and make a test commit:
   ```bash
   crowdcontrol connect test-github-auth
   # Inside the container:
   echo "Test commit from CrowdControl" >> README.md
   git add README.md
   git commit -m "test: verify GitHub App authentication"
   git push origin main
   ```

3. Check GitHub - the commit should show as authored by "CrowdControl[bot]"

## Troubleshooting

### "No GitHub configuration found" error
- Verify your environment variables are set correctly
- Run `env | grep GITHUB` to check current values
- Restart your terminal after setting environment variables

### "Authentication failed" error
- Check that your Installation ID is correct
- Verify the private key file path and permissions (`chmod 600` for security)
- Ensure the GitHub App is installed on the repository you're trying to access

### App not appearing in commit history
- Verify that the git user configuration inside the container is set to "CrowdControl[bot]"
- Check that the installation token is being used correctly

### Permission denied errors
- Review the GitHub App permissions - ensure "Contents: Write" is enabled
- Check if the repository has branch protection rules that might block the app
- Verify the app is installed on the specific repository

## Security Best Practices

1. **Protect your private key**: Store it with restricted permissions (`chmod 600`)
2. **Use minimal permissions**: Only grant the permissions your app actually needs
3. **Monitor app usage**: Regularly review the app's activity in GitHub
4. **Rotate keys regularly**: Generate new private keys periodically
5. **Use environment variables**: Don't commit credentials to version control

## Advanced: Organization-wide Setup

For organizations wanting to deploy CrowdControl across multiple repositories:

1. Create the GitHub App under your organization account
2. Set permissions for organization-wide installation
3. Install on all repositories or use a specific pattern
4. Share the App ID and Installation ID with team members
5. Use a secure secret management system for the private key

## Next Steps

Once GitHub authentication is working:
- CrowdControl commits will appear as "CrowdControl[bot]"
- Pull requests created by CrowdControl will show the app as the author
- You can track all CrowdControl activity separately from human commits
- Organization admins can manage CrowdControl access centrally

For more information, see the [GitHub Apps documentation](https://docs.github.com/en/developers/apps/getting-started-with-apps).