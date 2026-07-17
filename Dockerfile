# Dockerfile — runtime mínimo do tep-plant.
#
# Este Dockerfile NÃO compila o projeto Rust.
# Ele assume que o binário já foi gerado antes, pelo VSCode/Cargo/ambiente local,
# em:
#
#   target/release/tep-plant
#
# Ou seja, o fluxo esperado é:
#
#   1. Compilar o projeto fora do Docker:
#      cargo build --release --bin tep-plant
#
#   2. Construir a imagem:
#      docker build -t tep-plant .
#
#   3. Rodar o container:
#      docker run --rm -p 4840:4840 tep-plant
#
# O container apenas copia e executa o binário pronto.

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/tep-plant ./tep-plant

EXPOSE 4840

ENTRYPOINT ["./tep-plant"]