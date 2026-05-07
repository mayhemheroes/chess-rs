FROM rustlang/rust:nightly AS builder

RUN cargo install cargo-fuzz

ADD . /chess-rs
WORKDIR /chess-rs/chrs-lib/fuzz
RUN cargo +nightly fuzz build 

FROM debian:bookworm
COPY --from=builder /chess-rs/chrs-lib/fuzz/target/x86_64-unknown-linux-gnu/release/chrs-lib-fuzz /
