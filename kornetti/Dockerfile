# Build stage
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build dependencies (for caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --bin kornetti 2>/dev/null || true
RUN rm -rf src

# Build the actual binary
COPY . .
RUN cargo build --release --bin kornetti

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app

COPY --from=builder /app/target/release/kornetti /app/kornetti

# Create non-root user
RUN addgroup -g 1000 kornetti && \
    adduser -u 1000 -G kornetti -s /bin/sh -D kornetti

USER kornetti

EXPOSE 8080

ENTRYPOINT ["/app/kornetti"]
CMD ["server"]
