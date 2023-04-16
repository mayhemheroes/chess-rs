FROM ghcr.io/evanrichter/cargo-fuzz as builder

ADD . /chess-rs
WORKDIR /chess-rs/chrs-lib/fuzz
RUN cargo +nightly fuzz build 

FROM debian:bookworm
COPY --from=builder /chess-rs/chrs-lib/fuzz/target/x86_64-unknown-linux-gnu/release/chrs-lib-fuzz /