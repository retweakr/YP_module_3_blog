# Blog platform (module 3)

Rust workspace with four crates:

| Crate | Role |
|-------|------|
| **blog-server** | HTTP (Actix) + gRPC (Tonic), PostgreSQL (sqlx), JWT + Argon2 |
| **blog-client** | Shared library: HTTP (`reqwest`) and gRPC (`tonic`) transports |
| **blog-cli** | CLI using `blog-client`, optional `--grpc`, token file `.blog_token` |
| **blog-wasm** | Browser WASM UI via `gloo-net` + `wasm-bindgen` (HTTP only) |

Architecture on the server follows **clean architecture**: `domain` → `application` → `data` / `infrastructure` → `presentation` (`http_handlers`, `grpc_service`, JWT middleware).

## Prerequisites

- Rust stable (`rustup`), PostgreSQL 14+ (or compatible).
- Optional: [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) for ergonomic WASM bundles.
- For WASM builds: `rustup target add wasm32-unknown-unknown`.

## Configuration (server)

Create `blog-server/.env` (see `blog-server/.env.example`):

```env
DATABASE_URL=postgres://USER:PASSWORD@localhost/blog_db
JWT_SECRET=at_least_32_characters_for_hmac_secret_key
```

The server loads `blog-server/.env` automatically (via `CARGO_MANIFEST_DIR`), then falls back to the process environment.

- **HTTP** listens on **`0.0.0.0:8080`** (`/api/...`).
- **gRPC** listens on **`0.0.0.0:50051`** (`BlogService`).

## Database

Create an empty database, then start the server — **migrations run on startup** (`blog-server/migrations/`).

```bash
createdb blog_db   # or use psql / GUI
```

## Build everything

```bash
cargo build --workspace
```

## Run the API server

From the repository root:

```bash
export DATABASE_URL=postgres://...
export JWT_SECRET=your_32_plus_char_secret   # if not using .env
cargo run -p blog-server --bin blog-server
```

You should see logs for HTTP `:8080` and gRPC `:50051`.

### Quick HTTP checks

Register:

```bash
curl -s -X POST http://localhost:8080/api/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"secret123"}'
```

Create a post (replace `TOKEN`):

```bash
curl -s -X POST http://localhost:8080/api/posts \
  -H "Authorization: Bearer TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Hello","content":"World"}'
```

List posts:

```bash
curl -s 'http://localhost:8080/api/posts?limit=10&offset=0'
```

## CLI (`blog-cli`)

Defaults: HTTP `http://localhost:8080`, gRPC `http://localhost:50051`.

```bash
cargo run -p blog-cli --bin blog-cli -- register \
  --username alice --email alice@example.com --password secret123

cargo run -p blog-cli --bin blog-cli -- login --username alice --password secret123

cargo run -p blog-cli --bin blog-cli -- create --title "Hi" --content "Body"

cargo run -p blog-cli --bin blog-cli -- list --limit 10 --offset 0

cargo run -p blog-cli --bin blog-cli -- --grpc create --title "Via gRPC" --content "..."
```

JWT is stored in **`.blog_token`** in the current working directory after `register` / `login`.

## WASM frontend

1. Build the WASM package (from repo root):

   ```bash
   wasm-pack build blog-wasm --target web --out-dir pkg
   ```

   **Or** without wasm-pack (artifact under `target/` only):

   ```bash
   cargo build -p blog-wasm --target wasm32-unknown-unknown --release
   ```

2. If you used wasm-pack with `--out-dir pkg` at the workspace root, update `index.html` import to `./pkg/blog_wasm.js` (the sample `index.html` assumes `./blog-wasm/pkg/blog_wasm.js` when building inside `blog-wasm`).

3. Serve static files (example):

   ```bash
   python3 -m http.server 8000
   ```

4. Open `http://localhost:8000` — set `API_BASE` in `index.html` if the API is not on `http://localhost:8080`.

**CORS** is permissive in development (`allow_any_origin`). For production, restrict origins in `blog-server` CORS settings.

## Protocol buffers

`blog-server/proto/blog.proto` is the source of truth; an identical copy lives in `blog-client/proto/`. Regeneration is handled by each crate’s `build.rs` with `cargo:rerun-if-changed=proto/blog.proto`.

## Troubleshooting

- **`sqlx` / DB connection**: ensure `DATABASE_URL` matches a running PostgreSQL instance and the database exists.
- **JWT errors**: `JWT_SECRET` must be strong enough for HMAC; empty or short secrets may fail at runtime.
- **WASM + API host**: browsers require CORS; keep the static site origin compatible with server CORS rules (dev uses wide-open settings).
