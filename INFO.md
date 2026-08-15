# lacuna

A local, open source platform for drilling grammar by filling in blanks.
German first, other languages later. The name is the mechanic: a lacuna is a gap in a text.

## Locked decisions (2026-08-15)

| Decision | Choice |
|---|---|
| Interface | Local website, Next.js + TypeScript |
| Backend | Rust (axum + tokio), one binary that also serves the frontend |
| Database | SQLite, one `lacuna.db` file |
| Scheduler | `fsrs-rs`, the same algorithm Anki uses |
| Exercise unit | A sheet: one topic, 20 items |
| UI language | English only |
| Design direction | "Papier": warm paper, Source Serif 4 for German text, Inter for chrome, blanks as ruled underlines |
| Anki role | Read only, vocabulary staleness source. No grammar deck in Anki |

## Why Rust for the backend

1. `fsrs-rs` is first party. Anki's own scheduler is Rust, so the spacing behaviour is the real thing, not a port.
2. Grading is exact string work with edge cases (`Straße` vs `strasse`, noun capitalisation, sentence-initial case). Enums and exhaustive matching make that safe to extend.
3. Generated sheets must be validated ruthlessly. A serde struct plus a validation pass makes a malformed sheet unrepresentable in the database.
4. The grammar domain is closed sets: Case, Gender, Number, ErrorTag. Language packs get verified at load time.
5. Distribution: one binary with the frontend embedded (`rust-embed`). No Node needed by anyone who clones it.
6. Background work (pre-generating the next sheet, refreshing stale words) is natural in tokio, no queue or cron.

Cost: two languages and one API contract. Mitigated by generating TypeScript types from the Rust structs with `ts-rs`.

## Model

### Two independent schedulers

| Track | Lives in | Unit | Signal |
|---|---|---|---|
| Grammar mastery | lacuna's SQLite | topic | how you scored on sheets |
| Vocabulary recall | Anki deck "Deutsch" | word | Anki's own interval, read only via AnkiConnect |

A sheet is generated from the due topic, and its sentences are seeded with words you know but have not seen in 30+ days. Grammar practice refreshes cold vocabulary at the same time.

### Grading

Scores map to FSRS ratings automatically, with a manual override:

- under 60% correct: Again
- 60 to 80: Hard
- 80 to 95: Good
- above 95: Easy

Every wrong blank is also tagged (`case:dative`, `trigger:preposition:mit`, `article:der-group`) so the app can later target a specific weakness instead of re-drilling a whole topic.

### Language packs

`packs/de/` holds the category and topic tree plus generation hints. Categories: Articles, Pronouns, Verbs present, Cases, Adverbs, Connectors, Verbs modal, Verbs past tenses, The negative, Discourse and sentence types, Adjectives, Verbs subjunctive. Adding a language is adding a folder.

### Generation

Claude Code generates a sheet behind an API route, the Rust side validates and stores it. Sheets are cached, so a sheet can be redone offline and shared later.

## Blank design rules

1. Every blank starts at one constant width, so it never leaks the answer length.
2. Tab moves to the next blank, Enter checks the sheet. 20 items with no mouse.
3. A wrong answer shows the correct form in place, not in a summary at the bottom.
4. "Also accept" permanently patches the stored sheet when your answer was valid too. The generator will be wrong sometimes, so store an accept list per blank, never a single string.

## Open

- Full topic list per category, still to be supplied.
- Whether the "recycled words" panel should be visible during a sheet or only in the review afterwards.
