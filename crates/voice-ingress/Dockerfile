# Build Stage
FROM ghcr.io/revoltchat/base:latest AS builder

# Bundle Stage
FROM debian:bullseye-slim
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean
COPY --from=builder /home/rust/src/target/release/revolt-voice-ingress ./
EXPOSE 9000
CMD ["./revolt-voice-ingress"]
