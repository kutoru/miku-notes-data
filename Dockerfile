FROM rust:1.75

RUN apt-get update && DEBIAN_FRONTEND=nointeractive apt-get install --no-install-recommends --assume-yes protobuf-compiler

WORKDIR /usr/src/miku-notes-data
COPY . .

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres
RUN cargo install --path .

CMD /bin/sh -c "sqlx migrate run && miku-notes-data"
