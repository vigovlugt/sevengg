FROM rust as builder
WORKDIR /usr/src/sevengg
COPY . .
RUN cargo install --path .

CMD ["sevengg"]
