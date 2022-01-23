# RecipeS

You'll need to configure the `r_ecipe_s_backend/config/config.toml` (see the `config.toml.dist`) file for an example.
If you use the docker compose to spin up a postgres instance, you can set the password used there

you'll need the [sqlx cli](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli)

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
