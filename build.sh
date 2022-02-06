cd r_ecipe_s_frontend
perseus build
cd ../server
cargo build --release --target=x86_64-unknown-linux-gnu
cd ..
cp -r r_ecipe_s_frontend/.perseus/dist docker/
cp -r r_ecipe_s_frontend/static docker/
cp -r r_ecipe_s_frontend/index.html docker/
cp -r target/x86_64-unknown-linux-gnu/release/main docker/
cd docker
docker build -t r-ecipe-s:latest .
