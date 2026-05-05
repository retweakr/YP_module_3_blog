# Блог-платформа (модуль 3)

Rust workspace из четырёх крейтов:

| Крейт | Назначение |
|-------|------------|
| **blog-server** | HTTP (Actix) + gRPC (Tonic), PostgreSQL (sqlx), JWT и Argon2 |
| **blog-client** | Общая библиотека: транспорты HTTP (`reqwest`) и gRPC (`tonic`) |
| **blog-cli** | CLI на базе `blog-client`, флаг `--grpc`, токен в файле `.blog_token` |
| **blog-wasm** | WASM-интерфейс в браузере (`gloo-net`, `wasm-bindgen`), только HTTP |

На сервере используется **чистая архитектура**: `domain` → `application` → `data` / `infrastructure` → `presentation` (HTTP-обработчики, gRPC, JWT middleware).

## Структура репозитория

```
.
├── Cargo.toml              # workspace
├── README.md
├── index.html              # страница для WASM (после сборки pkg)
├── blog-server/            # бэкенд: миграции, proto, исходники
├── blog-client/            # библиотека клиента + копия blog.proto
├── blog-cli/               # консольный клиент
└── blog-wasm/              # WASM-библиотека (cdylib)
```

## Требования

- **Rust** (stable), [`rustup`](https://rustup.rs/).
- **PostgreSQL** 14+ (или совместимая версия).
- Для сборки WASM: `rustup target add wasm32-unknown-unknown`.
- Опционально: [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) для удобной упаковки WASM.

## Переменные окружения (сервер)

Создайте файл `blog-server/.env` по образцу `blog-server/.env.example`:

| Переменная | Описание |
|------------|----------|
| `DATABASE_URL` | Строка подключения к PostgreSQL, например `postgres://USER:PASSWORD@localhost/blog_db` |
| `JWT_SECRET` | Секрет для подписи JWT (**не короче 32 символов**, храните в секрете) |

Сервер при старте подгружает `blog-server/.env` (через путь к манифесту крейта), затем подхватывает переменные из окружения процесса.

Порты по умолчанию:

- **HTTP:** `0.0.0.0:8080`, префикс API: `/api/...`
- **gRPC:** `0.0.0.0:50051`, сервис `BlogService`

Файл `.env` не коммитьте (в `.gitignore`).

## База данных

Создайте пустую БД; при первом запуске сервера **миграции применятся автоматически** (`blog-server/migrations/`).

```bash
createdb blog_db   # или через psql / GUI
```

## Проверка сборки (перед сдачей)

Из корня репозитория:

```bash
cargo build --workspace
```

Ожидается успешная сборка всех четырёх крейтов без ошибок.

## Сборка

```bash
cargo build --workspace
cargo build --workspace --release   # опционально, релизная сборка
```

## Запуск API-сервера

Из корня репозитория (при использовании `.env` в `blog-server/` переменные можно не экспортировать):

```bash
cargo run -p blog-server --bin blog-server
```

Либо явно:

```bash
export DATABASE_URL=postgres://...
export JWT_SECRET=не_менее_32_символов_секрета
cargo run -p blog-server --bin blog-server
```

В логах должны появиться сообщения о прослушивании HTTP `:8080` и gRPC `:50051`.

### Примеры HTTP (curl)

Регистрация:

```bash
curl -s -X POST http://localhost:8080/api/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"secret123"}'
```

Вход:

```bash
curl -s -X POST http://localhost:8080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"secret123"}'
```

Создание поста (подставьте `TOKEN` из ответа регистрации/входа):

```bash
curl -s -X POST http://localhost:8080/api/posts \
  -H "Authorization: Bearer TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Привет","content":"Текст поста"}'
```

Список постов:

```bash
curl -s 'http://localhost:8080/api/posts?limit=10&offset=0'
```

Пост по id:

```bash
curl -s http://localhost:8080/api/posts/1
```

## CLI (`blog-cli`)

По умолчанию: HTTP `http://localhost:8080`, gRPC `http://localhost:50051`.

```bash
cargo run -p blog-cli --bin blog-cli -- register \
  --username alice --email alice@example.com --password secret123

cargo run -p blog-cli --bin blog-cli -- login --username alice --password secret123

cargo run -p blog-cli --bin blog-cli -- create --title "Заголовок" --content "Текст"

cargo run -p blog-cli --bin blog-cli -- list --limit 10 --offset 0

cargo run -p blog-cli --bin blog-cli -- get --id 1

# Тот же сценарий через gRPC:
cargo run -p blog-cli --bin blog-cli -- --grpc create --title "Через gRPC" --content "..."
```

После `register` / `login` JWT сохраняется в файл **`.blog_token`** в текущей рабочей директории.

## WASM-фронтенд

1. Соберите пакет одним из способов:

   ```bash
   wasm-pack build blog-wasm --target web
   ```

   По умолчанию появится каталог **`blog-wasm/pkg/`** — такой путь указан в корневом `index.html` (`./blog-wasm/pkg/blog_wasm.js`).

   Чтобы положить `pkg` в корень репозитория:

   ```bash
   wasm-pack build blog-wasm --target web --out-dir pkg
   ```

   Тогда в `index.html` замените импорт на `./pkg/blog_wasm.js`.

   Альтернатива без wasm-pack (артефакт в `target/`):

   ```bash
   cargo build -p blog-wasm --target wasm32-unknown-unknown --release
   ```

2. Поднимите статический сервер из каталога, где лежат `index.html` и каталог `pkg`:

   ```bash
   python3 -m http.server 8000
   ```

3. Откройте в браузере `http://localhost:8000`. Если API не на `http://localhost:8080`, измените константу `API_BASE` в `index.html`.

В режиме разработки на сервере включён широкий **CORS**; для продакшена ограничьте список разрешённых источников в коде сервера.

## Protocol Buffers

Источник схемы: `blog-server/proto/blog.proto`; копия для клиента: `blog-client/proto/blog.proto`. Генерация кода — в `build.rs` каждого крейта, пересборка при изменении proto: `cargo:rerun-if-changed=proto/blog.proto`.

## Устранение неполадок

- **Подключение к БД:** проверьте `DATABASE_URL`, что PostgreSQL запущен и база создана.
- **JWT:** слабый или пустой `JWT_SECRET` может привести к ошибкам при старте или проверке токенов.
- **Браузер и CORS:** фронт и API должны быть согласованы по origin или CORS-настройкам сервера.

## English summary

Same workspace: **blog-server** (HTTP :8080, gRPC :50051), **blog-client**, **blog-cli**, **blog-wasm**. Configure `blog-server/.env`, run `cargo build --workspace`, then `cargo run -p blog-server --bin blog-server`. Use `wasm-pack build blog-wasm --target web --out-dir pkg` and a static file server for the bundled `index.html`.
