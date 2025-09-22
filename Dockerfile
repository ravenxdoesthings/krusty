FROM rust:bullseye AS builder

WORKDIR /src/krusty
COPY . .

RUN apt-get update && apt-get install -y libssl-dev pkg-config ca-certificates
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /src/krusty/target/release/krusty /usr/local/bin/krusty

ENTRYPOINT ["krusty"]
