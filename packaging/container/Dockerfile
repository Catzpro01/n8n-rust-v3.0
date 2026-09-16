# SPDX-License-Identifier: AGPL-3.0-or-later
# Ticket 20: Minimal hardened multi-arch OCI container for Canopy Workbench (n8n-rust)

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -g 10001 workflowd \
    && useradd -u 10001 -g 10001 -s /sbin/nologin -d /var/lib/workflow-rust workflowd \
    && mkdir -p /var/lib/workflow-rust /tmp \
    && chown -R 10001:10001 /var/lib/workflow-rust /tmp

# The verified signed workflowd binary is copied from the build artifact
COPY workflowd /usr/bin/workflowd
RUN chmod 0755 /usr/bin/workflowd

USER 10001:10001
WORKDIR /var/lib/workflow-rust
VOLUME ["/var/lib/workflow-rust"]
EXPOSE 8000

ENV WORKFLOWD_STATE_DIR=/var/lib/workflow-rust \
    WORKFLOWD_LISTEN_ADDR=0.0.0.0:8000

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:8000/health/ready || exit 1

ENTRYPOINT ["/usr/bin/workflowd"]
CMD ["serve"]
