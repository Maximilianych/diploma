FROM rust:slim-bookworm AS builder
WORKDIR /usr/src/backend
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/backend* target/release/deps/backend*

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update -y && apt-get install -y ca-certificates
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/backend/target/release/backend /usr/local/bin/backend
RUN mkdir -p /app/db
COPY data.db /app/db/data.db

# Вообще так делать неправильно, но пусть пока так будет
COPY .env /app/.env 

WORKDIR /app
EXPOSE 8080
ENV RUST_BACKTRACE=1
CMD ["/usr/local/bin/backend"]