FROM rust:1.90-bookworm AS build
WORKDIR /app

# Copia todo el proyecto (respeta .dockerignore)
COPY . .

# Compila tu binario real
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# OJO: si tu binario se llama distinto, cambia "rust-service"
COPY --from=build /app/target/release/rust-service /app/rust-service

EXPOSE 8080
ENV PORT=8080
CMD ["/app/rust-service"]

