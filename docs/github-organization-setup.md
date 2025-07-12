# GitHub Organization Setup for CrowdControl

This guide covers advanced GitHub App setup patterns for organizations that want to share CrowdControl access across multiple repositories and users.

## Overview

CrowdControl supports organization-wide GitHub App sharing through credential templates, inspired by ArgoCD's repository credential templates. This allows:

- **Single GitHub App** for the entire organization
- **Shared credentials** across all team members
- **Repository-specific access control** through GitHub App permissions
- **GitHub Enterprise** support for corporate environments

## Organization Deployment Patterns

### Pattern 1: Single Organization App (Recommended)

One GitHub App installed organization-wide, shared by all CrowdControl users.

**Benefits:**
- Centralized management
- Consistent commit attribution
- Easy access control via GitHub
- Lower maintenance overhead

**Setup Process:**

1. **Organization Admin creates GitHub App** (see main setup guide)
2. **Install app organization-wide** or on specific repositories
3. **Share credentials** with team members
4. **Individual users configure** their local CrowdControl

### Pattern 2: Per-Repository Apps

Individual GitHub Apps for specific repositories or teams.

**Benefits:**
- Fine-grained access control
- Team-specific attribution
- Isolated credentials

**Use Cases:**
- Different teams with different security requirements
- External contractors needing limited access
- Compliance requirements for credential isolation

## Organization Configuration Methods

### Method 1: Environment Variables (Simplest)

All team members use the same environment variables:

```bash
# Shared by all team members
export GITHUB_APP_ID="123456"
export GITHUB_INSTALLATION_ID="789012"
export GITHUB_PRIVATE_KEY_PATH="/shared/secure/location/crowdcontrol-key.pem"

# For GitHub Enterprise
export GITHUB_BASE_URL="https://github.yourcompany.com"
```

### Method 2: Credential Templates (Future)

Configuration file-based credential sharing:

```toml
# ~/.config/crowdcontrol/github-templates.toml
[[templates]]
name = "acme-corp-main"
description = "Main ACME Corp GitHub App"
url_pattern = "https://github.com/acme-corp/*"

[templates.config]
app_id = "123456"
installation_id = "789012"
private_key_path = "/shared/crowdcontrol-acme.pem"

[[templates]]
name = "acme-labs"
description = "ACME Labs experimental projects"
url_pattern = "https://github.com/acme-labs/*"

[templates.config]
app_id = "654321"
installation_id = "210987"
private_key_path = "/shared/crowdcontrol-labs.pem"
```

## Security Considerations for Organizations

### Private Key Management

**Option A: Shared Network Location**
```bash
# Mount secure network share
export GITHUB_PRIVATE_KEY_PATH="/mnt/secure-share/crowdcontrol-key.pem"
```

**Option B: Individual Key Copies**
```bash
# Each user has their own copy
export GITHUB_PRIVATE_KEY_PATH="~/.config/crowdcontrol/github-key.pem"
```

**Option C: Secret Management System**
```bash
# Integration with corporate secret management
# (Future enhancement - could integrate with HashiCorp Vault, etc.)
```

### Access Control Best Practices

1. **GitHub App Permissions:**
   - Use minimal required permissions (Contents: Read+Write, PRs: Write)
   - Install only on repositories that need CrowdControl
   - Regular permission audits

2. **Private Key Security:**
   - Restrict file permissions (`chmod 600`)
   - Use secure storage locations
   - Regular key rotation (recommended: every 90 days)

3. **Network Security:**
   - GitHub Enterprise: Use internal networks where possible
   - Firewall rules for GitHub API access
   - Monitor API usage and anomalies

## GitHub Enterprise Setup

For organizations using GitHub Enterprise Server or Cloud:

### Environment Configuration
```bash
# GitHub Enterprise Cloud
export GITHUB_BASE_URL="https://github.yourcompany.com"

# GitHub Enterprise Server
export GITHUB_BASE_URL="https://github.enterprise.internal"

# Standard app credentials
export GITHUB_APP_ID="123456"
export GITHUB_INSTALLATION_ID="789012"
export GITHUB_PRIVATE_KEY_PATH="/path/to/key.pem"
```

### Network Requirements
- CrowdControl containers need HTTPS access to your GitHub Enterprise instance
- API endpoints: `$GITHUB_BASE_URL/api/v3/*`
- Git operations: `$GITHUB_BASE_URL/*`

## Team Onboarding Process

### For Organization Admins

1. **Create and configure GitHub App** (one-time setup)
2. **Distribute credentials** securely to team members
3. **Document organization-specific setup** (repo patterns, base URLs, etc.)
4. **Set up monitoring** for GitHub App usage
5. **Establish key rotation schedule**

### For Team Members

1. **Receive credentials** from organization admin
2. **Configure environment variables** or config files
3. **Test setup** with a test repository
4. **Verify commit attribution** shows as CrowdControl[bot]

### Onboarding Script Example

```bash
#!/bin/bash
# setup-crowdcontrol-acme.sh
# Organization-specific setup script

echo "Setting up CrowdControl for ACME Corp..."

# Organization-specific configuration
export GITHUB_BASE_URL="https://github.acme-corp.com"
export GITHUB_APP_ID="123456"
export GITHUB_INSTALLATION_ID="789012"

# Private key location (customize for your organization)
PRIVATE_KEY_PATH="$HOME/.config/crowdcontrol/acme-corp-key.pem"

if [ ! -f "$PRIVATE_KEY_PATH" ]; then
    echo "Error: Private key not found at $PRIVATE_KEY_PATH"
    echo "Please copy the key file from the secure location"
    exit 1
fi

export GITHUB_PRIVATE_KEY_PATH="$PRIVATE_KEY_PATH"

# Add to shell profile
echo "# CrowdControl ACME Corp Configuration" >> ~/.bashrc
echo "export GITHUB_BASE_URL=\"$GITHUB_BASE_URL\"" >> ~/.bashrc
echo "export GITHUB_APP_ID=\"$GITHUB_APP_ID\"" >> ~/.bashrc
echo "export GITHUB_INSTALLATION_ID=\"$GITHUB_INSTALLATION_ID\"" >> ~/.bashrc
echo "export GITHUB_PRIVATE_KEY_PATH=\"$PRIVATE_KEY_PATH\"" >> ~/.bashrc

echo "CrowdControl configured for ACME Corp!"
echo "Restart your terminal or run: source ~/.bashrc"
```

## Monitoring and Maintenance

### GitHub App Usage Monitoring

Monitor these metrics in your GitHub organization:
- API request usage and rate limits
- Repository access patterns
- Commit/PR attribution patterns
- Failed authentication attempts

### Maintenance Tasks

**Weekly:**
- Review GitHub App activity logs
- Check for failed authentication attempts
- Verify team member access

**Monthly:**
- Review and update repository access permissions
- Audit private key file locations and permissions
- Check for new team members needing access

**Quarterly:**
- Rotate GitHub App private keys
- Review overall security posture
- Update documentation and onboarding materials

## Troubleshooting Organization Issues

### "Authentication failed" for some repositories
- Verify GitHub App is installed on the specific repository
- Check repository permissions in GitHub App settings
- Ensure private key file is accessible to all team members

### Inconsistent commit attribution
- Verify all team members are using the same GitHub App
- Check environment variable configuration across team
- Ensure git user.name is set to "CrowdControl[bot]"

### GitHub Enterprise connectivity issues
- Verify `GITHUB_BASE_URL` is correctly set
- Test network connectivity to GitHub Enterprise instance
- Check corporate firewall rules for GitHub API access

## Migration from Individual to Organization Setup

If transitioning from individual GitHub Apps to organization-wide:

1. **Create organization GitHub App** following this guide
2. **Test with one repository** before rolling out widely
3. **Update team environment variables** to use new app
4. **Uninstall individual GitHub Apps** once migration is complete
5. **Update documentation** with new organization-specific instructions

## Advanced: CI/CD Integration

For organizations using CrowdControl in CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Setup CrowdControl GitHub Auth
  env:
    GITHUB_APP_ID: ${{ secrets.CROWDCONTROL_APP_ID }}
    GITHUB_INSTALLATION_ID: ${{ secrets.CROWDCONTROL_INSTALLATION_ID }}
    GITHUB_PRIVATE_KEY_PATH: /tmp/crowdcontrol-key.pem
  run: |
    echo "${{ secrets.CROWDCONTROL_PRIVATE_KEY }}" > /tmp/crowdcontrol-key.pem
    chmod 600 /tmp/crowdcontrol-key.pem
    crowdcontrol new ci-agent ${{ github.repository }}
```

## Summary

Organization-wide CrowdControl deployment provides:
- **Centralized credential management**
- **Consistent GitHub App attribution**
- **GitHub Enterprise support**
- **Scalable team onboarding**
- **Security best practices**

This approach scales from small teams to large enterprises while maintaining security and ease of use.