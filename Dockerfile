FROM rust:1.75

RUN apt-get update && DEBIAN_FRONTEND=nointeractive apt-get install --no-install-recommends --assume-yes protobuf-compiler

WORKDIR /usr/src/miku-notes-data
COPY . .

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres
RUN sqlx migrate run

RUN cargo install --path .

CMD ["miku-notes-data"]
