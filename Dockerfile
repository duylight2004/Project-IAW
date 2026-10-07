# ── build stage ── bắt buộc --release (Act 2 dựa vào overflow-checks=false)
FROM rust:slim AS build
WORKDIR /app

# cache deps (Cargo.lock để khóa phiên bản -> build tái lập được)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked || true
RUN rm -rf src

# nguồn thật
COPY src ./src
COPY templates ./templates
# include_str! cần templates lúc compile; static được copy ở runtime stage
RUN touch src/main.rs && cargo build --release --locked

# ── runtime stage ──
FROM debian:stable-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -u 1000 -m antiqua

WORKDIR /app
COPY --from=build /app/target/release/antiqua-library /app/antiqua-library
COPY static ./static
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh \
    && chown -R 1000:1000 /app

ENV DATABASE_URL=sqlite:///tmp/db/antiqua.db
ENV TMPDIR=/tmp
EXPOSE 8080
USER 1000
ENTRYPOINT ["/app/entrypoint.sh"]
