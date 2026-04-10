# ── Build stage ──
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev curl \
    && rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos

WORKDIR /app
COPY Cargo.toml Cargo.lock* rust-toolchain.toml ./
COPY src/ src/
COPY style/ style/
COPY public/ public/

RUN cargo leptos build --release

# ── Runtime stage — Kali Linux with tools ──
FROM kalilinux/kali-rolling

ENV DEBIAN_FRONTEND=noninteractive

# Security tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    nmap nikto sqlmap python3-pip python3-venv \
    curl wget git ca-certificates unzip \
    default-jre-headless \
    xfonts-75dpi xfonts-base fontconfig libxrender1 libxext6 \
    && rm -rf /var/lib/apt/lists/*

# wkhtmltopdf (not in Kali repos — install from upstream .deb)
RUN curl -sSLo /tmp/wkhtmltopdf.deb https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-3/wkhtmltox_0.12.6.1-3.bookworm_amd64.deb \
    && dpkg -i /tmp/wkhtmltopdf.deb || apt-get install -f -y \
    && rm /tmp/wkhtmltopdf.deb

# Bandit
RUN pip3 install --break-system-packages bandit

# Trivy
RUN curl -sfL https://github.com/aquasecurity/trivy/releases/download/v0.69.3/trivy_0.69.3_Linux-64bit.deb -o /tmp/trivy.deb \
    && dpkg -i /tmp/trivy.deb && rm /tmp/trivy.deb

# sonar-scanner CLI
RUN curl -sSLo /tmp/sonar-scanner.zip https://binaries.sonarsource.com/Distribution/sonar-scanner-cli/sonar-scanner-cli-6.2.1.4610-linux-x64.zip \
    && unzip /tmp/sonar-scanner.zip -d /opt \
    && ln -s /opt/sonar-scanner-6.2.1.4610-linux-x64/bin/sonar-scanner /usr/local/bin/sonar-scanner \
    && rm /tmp/sonar-scanner.zip

# OWASP Dependency-Check
RUN curl -sSLo /tmp/dc.zip https://github.com/jeremylong/DependencyCheck/releases/download/v12.1.0/dependency-check-12.1.0-release.zip \
    && unzip /tmp/dc.zip -d /opt \
    && chmod +x /opt/dependency-check/bin/dependency-check.sh \
    && rm /tmp/dc.zip

WORKDIR /app

# Copy built artifacts from builder
COPY --from=builder /app/target/release/watchtower /app/watchtower
COPY --from=builder /app/target/site /app/target/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml

# Create data dirs
RUN mkdir -p /app/data /app/reports /tmp/empty-sonar

EXPOSE 67

ENV LEPTOS_OUTPUT_NAME="watchtower"
ENV LEPTOS_SITE_ROOT="target/site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:67"
ENV LEPTOS_RELOAD_PORT="3001"
ENV DATABASE_URL="sqlite:///app/data/watchtower.db?mode=rwc"

CMD ["/app/watchtower"]
