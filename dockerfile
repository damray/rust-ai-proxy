FROM rust:latest as builder

WORKDIR /app

# Copie ton manifest et télécharge les dépendances
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/proxy_openwebui*

# Copie le code source
COPY . .

# Compile en release
RUN cargo build --release

# Image finale
FROM debian:bullseye-slim

WORKDIR /app

# Installe les libs nécessaires
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copie juste le binaire
COPY --from=builder /app/target/release/proxy_openwebui .

EXPOSE 3000

CMD ["./proxy_openwebui"]