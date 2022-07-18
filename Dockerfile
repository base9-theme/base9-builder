FROM rust:latest
WORKDIR /app
RUN cargo install wasm-pack

COPY Cargo.toml .

# Faster build
RUN mkdir src
RUN touch src/lib.rs
RUN cargo build
RUN cargo build --release
RUN wasm-pack build --target web
RUN rm -r src

COPY . .
RUN scripts/deploy.sh
