# syntax=docker/dockerfile:1

FROM rust:1.83-alpine AS builder
WORKDIR /app
RUN apk add --no-cache musl-dev openssl-dev pkgconfig
COPY . .
RUN cargo build --release

FROM alpine:3.20
WORKDIR /app
RUN apk add --no-cache ca-certificates tzdata
COPY --from=builder /app/target/release/mealmind /usr/local/bin/mealmind
COPY migrations ./migrations
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
EXPOSE 8080
CMD ["/usr/local/bin/mealmind"]
