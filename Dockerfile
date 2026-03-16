# Strike Security Scanner - Production Dockerfile
# Multi-stage build for optimized image size with all security tools pre-installed

# Stage 1: Rust builder
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY examples ./examples
COPY tests ./tests

# Build release binary
RUN cargo build --release --locked

# Stage 2: Security tools base
FROM debian:bookworm-slim AS tools

# Install system dependencies and security tools
RUN apt-get update && apt-get install -y \
    # Core utilities
    curl \
    wget \
    git \
    unzip \
    ca-certificates \
    gnupg \
    lsb-release \
    # Network tools
    nmap \
    masscan \
    netcat-openbsd \
    dnsutils \
    whois \
    # SSL/TLS tools
    openssl \
    # Python for tools
    python3 \
    python3-pip \
    python3-venv \
    # Build tools for Go-based tools
    golang-go \
    # Ruby for tools
    ruby \
    ruby-dev \
    # Additional dependencies
    libpcap-dev \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash striker && \
    mkdir -p /home/striker/.strike /home/striker/tools && \
    chown -R striker:striker /home/striker

# Switch to striker user for tool installations
USER striker
WORKDIR /home/striker/tools

# Install Go-based tools
ENV GOPATH=/home/striker/go
ENV PATH=$PATH:$GOPATH/bin

# Nuclei - Vulnerability scanner
RUN go install -v github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest

# Subfinder - Subdomain discovery
RUN go install -v github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest

# httpx - HTTP toolkit
RUN go install -v github.com/projectdiscovery/httpx/cmd/httpx@latest

# Gobuster - Directory/DNS bruteforcer
RUN go install -v github.com/OJ/gobuster/v3@latest

# FFuf - Web fuzzer
RUN go install -v github.com/ffuf/ffuf/v2@latest

# Amass - Network mapping
RUN go install -v github.com/owasp-amass/amass/v4/...@master

# Katana - Web crawler
RUN go install -v github.com/projectdiscovery/katana/cmd/katana@latest

# dnsx - DNS toolkit
RUN go install -v github.com/projectdiscovery/dnsx/cmd/dnsx@latest

# Create Python virtual environment
RUN python3 -m venv /home/striker/venv
ENV PATH="/home/striker/venv/bin:$PATH"

# Install Python-based tools
RUN pip install --no-cache-dir \
    # SQLMap - SQL injection
    sqlmap \
    # Arjun - HTTP parameter discovery
    arjun \
    # XSStrike - XSS detection
    xsstrike \
    # CMSeeK - CMS detection
    cmseek \
    # WhatWeb alternative
    builtwith \
    # Additional utilities
    requests \
    beautifulsoup4 \
    lxml

# Install Nikto (Perl-based)
USER root
RUN cd /opt && \
    git clone https://github.com/sullo/nikto.git && \
    cd nikto/program && \
    chmod +x nikto.pl && \
    ln -s /opt/nikto/program/nikto.pl /usr/local/bin/nikto

# Install WPScan (Ruby-based)
RUN gem install wpscan

# Install testssl.sh
RUN cd /opt && \
    git clone --depth 1 https://github.com/drwetter/testssl.sh.git && \
    chmod +x /opt/testssl.sh/testssl.sh && \
    ln -s /opt/testssl.sh/testssl.sh /usr/local/bin/testssl

# Install wafw00f (WAF detection)
RUN pip3 install --no-cache-dir wafw00f

# Download Nuclei templates
USER striker
RUN nuclei -update-templates

# Stage 3: Final runtime image
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    nmap \
    masscan \
    openssl \
    python3 \
    python3-pip \
    ruby \
    libpcap0.8 \
    netcat-openbsd \
    dnsutils \
    whois \
    git \
    perl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash striker && \
    mkdir -p /home/striker/.strike /home/striker/reports /home/striker/workflows && \
    chown -R striker:striker /home/striker

# Copy Go binaries from tools stage
COPY --from=tools --chown=striker:striker /home/striker/go/bin/* /usr/local/bin/

# Copy Python venv from tools stage
COPY --from=tools --chown=striker:striker /home/striker/venv /home/striker/venv

# Copy installed tools from tools stage
COPY --from=tools /opt/nikto /opt/nikto
COPY --from=tools /opt/testssl.sh /opt/testssl.sh
COPY --from=tools /usr/local/bin/nikto /usr/local/bin/nikto
COPY --from=tools /usr/local/bin/testssl /usr/local/bin/testssl

# Copy Ruby gems
COPY --from=tools /var/lib/gems /var/lib/gems
COPY --from=tools /usr/local/bundle /usr/local/bundle

# Copy Nuclei templates
COPY --from=tools --chown=striker:striker /home/striker/nuclei-templates /home/striker/nuclei-templates

# Copy Strike binary from builder
COPY --from=builder /build/target/release/strike /usr/local/bin/strike

# Set up Python path
ENV PATH="/home/striker/venv/bin:$PATH"
ENV PYTHONPATH="/home/striker/venv/lib/python3.11/site-packages"

# Switch to non-root user
USER striker
WORKDIR /home/striker

# Create default configuration
RUN mkdir -p /home/striker/.strike/workflows /home/striker/.strike/templates

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD strike --version || exit 1

# Default command
ENTRYPOINT ["strike"]
CMD ["--help"]

# Labels
LABEL maintainer="Strike Security Team"
LABEL description="Strike - Autonomous AI-Powered Security Scanner"
LABEL version="1.0.0"
LABEL org.opencontainers.image.source="https://github.com/yourusername/strike"
LABEL org.opencontainers.image.documentation="https://github.com/yourusername/strike/blob/main/README.md"

# Expose default ports (if needed for web UI in future)
# EXPOSE 8080

# Volume for persistent data
VOLUME ["/home/striker/.strike", "/home/striker/reports", "/home/striker/workflows"]
