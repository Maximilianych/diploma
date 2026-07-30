FROM rust:bookworm AS builder
WORKDIR /usr/src/frontend
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall trunk
RUN rustup target add wasm32-unknown-unknown
RUN apt update && apt install nodejs npm -y
RUN npm install @tailwindcss/cli
RUN npm install @tailwindcss/oxide-linux-x64-gnu
RUN npm install @parcel/watcher
RUN npm install @parcel/watcher-linux-x64-glibc
COPY . .
RUN npm install
RUN trunk build --release

FROM nginx:alpine
COPY --from=builder /usr/src/frontend/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf