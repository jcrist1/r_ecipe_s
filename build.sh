cd server
export  TARGET_CC=x86_64-unknown-linux-gnu-gcc
cargo build --release --target=x86_64-unknown-linux-gnu
cd ../frontend_ls
nvm use 19
trunk build --release
cd ..
cp -r frontend_ls/static docker/
cp -r frontend_ls/dist docker/
cp -r target/x86_64-unknown-linux-gnu/release/r_ecipe_s_server docker/
cd docker
docker build -t r-ecipe-s:latest .
