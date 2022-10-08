FROM alpine:edge
COPY . /
WORKDIR /
RUN apk add --no-cache rustup build-base libpq-dev nodejs npm
RUN rustup-init -q -y --default-toolchain nightly --profile minimal
RUN source $HOME/.cargo/env && \
	cargo install diesel_cli --no-default-features -F postgres && \
	cargo build --release
WORKDIR /marketplace
RUN npm install
RUN node_modules/.bin/vite build
WORKDIR /
CMD diesel migration run && /target/release/server
