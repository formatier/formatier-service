FROM rust:alpine as builder

WORKDIR /app
COPY . .

RUN cargo build -p auth-service --release --bin auth-service

FROM gcr.io/distroless/cc-debian13

WORKDIR /bin
COPY --from=builder /app/target/release/auth-service ./

CMD ["/bin/auth-service"]
