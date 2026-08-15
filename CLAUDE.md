# lacuna

A local, open source platform for drilling grammar by filling in blanks. German first,
other languages later. The name is the mechanic: a lacuna is a gap in a text.

Read `INFO.md` for the decisions and why they were made. Read `TODO.md` for the work.
Do not re-open a settled decision without saying so explicitly.

## Stack

| Part | Choice |
|---|---|
| Frontend | Next.js + TypeScript, in `web/` |
| Backend | Rust, axum + tokio, in `server/` |
| Database | SQLite, one `lacuna.db`, `sqlx` migrations |
| Scheduling | `fsrs-rs`, the same algorithm Anki uses |
| Type bridge | `ts-rs`, Rust structs export TypeScript types into `web/` |
| Packaging | `rust-embed`, one binary that also serves the frontend |

## Layout

```
server/          Rust backend
web/             Next.js frontend
packs/de/        German language pack (topics.toml and generation hints)
INFO.md          decisions and rationale
TODO.md          the work, in build order
```

## Invariants

These are not preferences. Breaking one costs a migration or a rewrite later.

1. **A blank stores an accept list, never a single string.** The generator will
   sometimes mark a valid answer wrong, and "also accept" must be able to patch a
   stored sheet permanently.
2. **Error tags are produced at generation time**, while the model still knows what
   it was testing. Never infer them afterwards from a wrong answer.
3. **Anki is read only.** It is a source of stale vocabulary through AnkiConnect and
   nothing else. Never write to a deck. Never create a grammar deck in Anki, because
   a grammar topic is a generator of prompts and error tags, not a single card.
4. **Two independent schedulers.** Grammar mastery lives in lacuna's SQLite, keyed by
   topic. Vocabulary recall stays in the Anki "Deutsch" deck. They never merge.
5. **A sheet is one topic and 20 items.** Not a mixed drill.
6. **A generated sheet is validated before it is stored**, and an invalid one is
   regenerated, never repaired by hand at read time.
7. **Anki not running must never block a session.** Degrade to no recycled words.

## UI direction: "Papier"

A printed German workbook, not a SaaS dashboard.

- Source Serif 4 for German text, Inter for interface chrome
- paper `#F2ECE1`, sheet `#FDFBF6`, ink `#1E1B16`, hint blue `#27456F`,
  wrong `#A33529`, right `#3D6B49`, due gold `#C8912F`
- Blanks are ruled underlines, never boxes
- Every blank is one constant width, so its size never leaks the answer length
- Tab moves to the next blank, Enter checks the sheet, 20 items with no mouse
- A wrong answer shows the correct form in place, not in a summary at the bottom

## Language and content

- The interface is English only. No Russian, no German labels in the chrome.
- German appears only inside exercise content.
- Sentences describe ordinary situations in Germany: bakery, Bürgeramt, train ticket,
  doctor, flat viewing, phone call. Not textbook abstractions.

## The topic tree

`packs/de/topics.toml` holds 43 topics across A1, A2 and B1. Each has an `id`, its
source `cefr` label, and a `stage` giving the real teaching order, which differs from
the source order in four documented places. If you change an order, update the notes
at the bottom of that file in the same edit.

## Conventions

- No em dashes or en dashes anywhere, including code comments and commit messages.
- Commit messages are one short line, no body, no attribution or co-author trailers.
- **Commit and push after every finished feature.** A feature is done when it builds,
  its tests pass, and it is committed and pushed. Do not batch several features into
  one commit, and do not leave finished work sitting uncommitted.
- Grading and normalisation logic gets unit tests. That is where the bugs will be.
- Prefer quality, simplicity and long term maintainability over development speed.
