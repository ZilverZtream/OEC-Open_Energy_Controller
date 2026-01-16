# syntax=docker/dockerfile:1
FROM rust:1.78-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --features "swagger metrics"

FROM debian:bookworm-slim
WORKDIR /app
RUN useradd -m appuser
COPY --from=builder /app/target/release/open-energy-controller /app/oec
COPY config /app/config
USER appuser
EXPOSE 8080
ENV RUST_LOG=info
CMD ["/app/oec"]
