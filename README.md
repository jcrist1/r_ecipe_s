# r_ecipe_s

You'll need to configure the `r_ecipe_s_backend/config/config.toml` (see the `config.toml.dist`) file for an example.
If you use the docker compose to spin up a postgres instance, you can set the password used there

you'll need to run the backend:
```bash
cd r_ecipe_s_backend
cargo run
```
then run the frontend (you will need the perseus cli)
```
cd r_ecipe_s_frontend
perseus serve
```
