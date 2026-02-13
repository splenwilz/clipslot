# Build stage
FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN groupadd app && useradd -r -g app -d /nonexistent -s /sbin/nologin app
COPY --from=builder /app/target/release/clipslot-server /usr/local/bin/
RUN chown app:app /usr/local/bin/clipslot-server
USER app
EXPOSE 3000
CMD ["clipslot-server"]
