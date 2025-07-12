FROM debian:12-slim

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install core dependencies
RUN apt-get update && apt-get install -y \
    docker.io \
    git ssh-client curl wget \
    jq unzip build-essential \
    sudo vim nano \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Install docker-compose as standalone binary
RUN curl -L "https://github.com/docker/compose/releases/download/v2.24.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose \
    && chmod +x /usr/local/bin/docker-compose

# Install Node.js 22 (needed for Claude Code)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs

# Install Claude Code
RUN npm install -g @anthropic-ai/claude-code

# Create developer user with configurable UID/GID
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd -g ${GROUP_ID} developer && \
    useradd -u ${USER_ID} -g ${GROUP_ID} -m -s /bin/bash developer && \
    usermod -aG sudo,docker developer && \
    echo 'developer ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

# Copy and configure entrypoint and scripts
COPY entrypoint.sh /entrypoint.sh
COPY refresh-claude-auth.sh /usr/local/bin/refresh-claude-auth.sh
COPY test-refresh-claude-auth.sh /usr/local/bin/test-refresh-claude-auth.sh
RUN chmod +x /entrypoint.sh /usr/local/bin/refresh-claude-auth.sh /usr/local/bin/test-refresh-claude-auth.sh

# Run tests during build to ensure scripts work
RUN /usr/local/bin/test-refresh-claude-auth.sh

# Set working directory
WORKDIR /workspace

# Use generic entrypoint
ENTRYPOINT ["/entrypoint.sh"]