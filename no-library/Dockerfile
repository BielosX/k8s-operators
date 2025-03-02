FROM rust:slim
COPY . /workspace
WORKDIR /workspace
RUN apt-get update && apt-get install -y pkg-config libssl-dev && cargo build --release

FROM debian:12-slim
RUN apt-get update && apt-get install -y openssl ca-certificates libssl-dev
COPY --from=0 /workspace/target/release/no-library /bin/no-library
CMD ["/bin/no-library"]