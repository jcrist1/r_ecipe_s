cd server
export  TARGET_CC=x86_64-unknown-linux-gnu-gcc
cargo build --release --target=x86_64-unknown-linux-gnu
cd ../frontend
trunk build --release
cd ..
cp -r frontend/static docker/
cp -r frontend/dist docker/
cp -r target/x86_64-unknown-linux-gnu/release/r_ecipe_s_server docker/
cd docker
docker build -t r-ecipe-s:latest .
