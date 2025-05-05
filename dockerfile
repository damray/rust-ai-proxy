# ------------ Builder Stage ------------
    FROM rust:1.86-bookworm AS builder

    WORKDIR /app
    
    # Copie tous les fichiers projet
    COPY . .
    
    # Palo Alto / proxy SSL: désactive les vérifications de certificats
    RUN git config --global http.sslVerify false
    ENV CARGO_HTTP_CHECK_REVOKE=false
    ENV GIT_SSL_NO_VERIFY=true

    # Build en release
    RUN cargo build --release
    
    # ------------ Runtime Stage ------------
    FROM debian:bookworm-slim
    
    WORKDIR /app
    
    # Install certificats SSL pour les requêtes HTTPs (reqwest/AIRS)
    RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
     && rm -rf /var/lib/apt/lists/*
    
    # Copie du binaire depuis le builder
    COPY --from=builder /app/target/release/rust_ai_proxy .
    
    # Ajoute un utilisateur non-root (optionnel mais conseillé)
    RUN useradd -m rustuser
    USER rustuser
    
    # Expose port API
    EXPOSE 3000
    
    # Lance le proxy
    ENTRYPOINT ["./rust_ai_proxy"]
    