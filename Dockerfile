FROM rust:latest as builder

WORKDIR /app

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/backup_upload_server /usr/local/bin/

WORKDIR /app

# Create a non-privileged user
ARG UID=10001
RUN adduser --disabled-password --gecos "" --shell "/sbin/nologin" --uid "${UID}" appuser

RUN mkdir -p /app/uploads && chown -R appuser:appuser /app/uploads

# Switch to non-privileged user
USER appuser

# Expose port
EXPOSE 8080

CMD ["backup_upload_server"]
