# ── build stage ── bắt buộc --release (Act 2 dựa vào overflow-checks=false)
FROM rust:slim AS build
WORKDIR /app

# cache deps
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release || true
RUN rm -rf src

# nguồn thật
COPY src ./src
COPY templates ./templates
# include_str! cần templates lúc compile; static được copy ở runtime stage
RUN touch src/main.rs && cargo build --release

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
RUN chmod +x /app/entrypoint.sh

ENV DATABASE_URL=sqlite:///tmp/db/antiqua.db
EXPOSE 8080
USER 1000
ENTRYPOINT ["/app/entrypoint.sh"]
