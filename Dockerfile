FROM rust as builder
WORKDIR /usr/src/sevengg
COPY . .
RUN cargo prisma db push
RUN cargo install --path .

CMD ["sevengg"]
