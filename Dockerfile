# syntax=docker/dockerfile:1
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
COPY . .
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
WORKDIR /app
COPY --from=builder /app/target/release/mybooks /app/mybooks
COPY --from=builder /app/migrations /app/migrations
RUN mkdir -p /app/static
COPY static/ /app/static/
ENV DATABASE_URL=sqlite:///data/mybooks.db
ENV PORT=3000
VOLUME ["/data"]
CMD ["/app/mybooks"]
