FROM rust:1.87 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/mengxuegushi-rust /usr/local/bin/mengxuegushi-rust
ENV PORT=8080
EXPOSE 8080
CMD ["mengxuegushi-rust"]
