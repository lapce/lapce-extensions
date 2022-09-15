FROM gitpod/workspace-full
RUN npm install --global nodemon
RUN cargo install diesel_cli --no-default-features -F postgres
