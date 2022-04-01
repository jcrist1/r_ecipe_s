![ferris the rustacean chef](frontend/static/ferris-chef.svg)
# RecipeS


You'll need to configure the `r_ecipe_s_backend/config/config.toml` (see the `config.toml.dist`) file for an example.
If you use the docker compose to spin up a postgres instance, you can set the password used there

you'll need the [sqlx cli](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli)
you'll need the [perseus cli](https://docs.rs/perseus-cli/latest/perseus_cli/index.html)

Run migrations
```bash
cd r_ecipe_s_backend
sqlx run migrations
```
you'll need to run migrations to compile. Then run the backend:
```bash
cargo run
```
then run the frontend (you will need the perseus cli)
```
cd r_ecipe_s_frontend
perseus build
```
When perseus first runs it will create it's own `r_ecipe_s_frontend/.perseus/Cargo.toml` where it defines empty
workspaces. The backend depends on this workspace, but is in its own workspace. To fix things
just remove `[workspaces]` from the `r_ecipe_s_frontend/.perseus/Cargo.toml`

## Building docker image
for now it's a little clunky.  Need to compile the frontend, but now compile the backend with
```bash
cargo build --release --target=x86_64-unknown-linux-gnu
```
On MacOS you will need the `x86_64-unknown-linux-gnu` linker from `brew`

Then copy the executable from the root: `target/x86_64-unknown-linux-gnu/release/main` to `docker/`
Copy the directory `r_ecipe_s_frontend/.perseus/dist` to `docker/`.
Copy `r_ecipe_s_frontend/index.html` to `docker/`.
Copy `r_ecipe_s_frontend/static` to `docker/`.
We currently have to do this in order to have a small build context for `fly`

## Building Css
```sh
tailwindcss-to-rust --input frontend/css/tailwind.css --tailwind-config frontend/tailwind.config.js --output r_ecipe_s_style/src/generated.rs --rustfmt
```
Then you need to make everything `pub` in the generated file instead of `pub(crate)`. 
Now tailwind can scan a specific dependency without scanning the whole generated rust file

