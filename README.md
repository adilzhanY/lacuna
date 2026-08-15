# lacuna

Grammar practice by filling in the gaps. German first, other languages later.

A sheet is one grammar topic and twenty sentences with blanks in them. You fill them in,
lacuna grades them, and the topic comes back on an FSRS schedule, the same algorithm Anki
uses. Everything runs locally against one SQLite file.

Read `INFO.md` for the decisions behind it and `TODO.md` for what is still missing.

## Status

Early. The full loop works end to end: open a topic, answer twenty blanks, get graded,
watch the topic get rescheduled. Sheets are still hand written fixtures, since generation
is not wired up yet.

## Running it

Two processes in development, one binary later.

```sh
# backend, on http://127.0.0.1:4000
cd server
cargo run

# frontend, on http://localhost:3000
cd web
npm install
npm run dev
```

The Next dev server proxies `/api` to the backend, so the browser only sees one origin.

Environment variables the backend understands:

| Variable | Default | Meaning |
|---|---|---|
| `LACUNA_DB` | `sqlite://lacuna.db` | Database file |
| `LACUNA_PACKS` | `../packs` | Where language packs live |
| `LACUNA_LANGUAGE` | `de` | Which pack to load |
| `LACUNA_PORT` | `4000` | Backend port |

## Tests

```sh
cd server && cargo test     # includes a check that every shipped sheet is valid
cd web && npm run lint && npx tsc --noEmit
```

## Layout

```
server/          Rust backend: axum, sqlx, fsrs
web/             Next.js frontend
packs/de/        German pack: the topic tree and seed sheets
```

TypeScript types are generated from the Rust structs by `cargo test`, into
`web/src/lib/types/`. Do not edit those by hand.
