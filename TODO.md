# lacuna TODO

Everything decided so far, turned into work. Decisions and rationale live in `INFO.md`.
Ticked items are done and committed.

## 0. Repo scaffold

- [x] `cargo new` the backend as `server/`, axum + tokio
- [x] `create-next-app` the frontend as `web/`, TypeScript
- [x] Wire `ts-rs` so Rust structs export TypeScript types into `web/`
- [x] Dev setup: Next dev server proxies `/api` to the Rust server
- [ ] Release setup: `rust-embed` bundles the Next static export into the binary
- [x] `.gitignore`, `README.md`, MIT licence

## 1. Data model and database

- [x] SQLite schema in `sqlx` migrations, single `lacuna.db`
- [x] `topic` table, loaded from the pack, keyed by the ids in `packs/de/topics.toml`
- [x] `sheet` table: topic, generated JSON, created date, which words were recycled
- [x] `item` and `blank`: **`accept` is a list from day one, never a single string**
- [x] `answer` log: what you typed, whether it was right, the error tags
- [x] `topic_state`: FSRS stability, difficulty, due date, review count, lapses
- [x] `settings` table, since "lots of customisation" should not mean editing code
- [x] Rust enums for `Level`, `Rating`, `ErrorKind` and `ErrorTag`, validated at pack load
      (`Case`, `Gender` and `Number` live inside error tag details for now, promote them to
      their own enums when generation needs to reason about them)

## 2. Language pack

- [x] `packs/de/topics.toml`, 43 topics across A1, A2, B1, with teaching order
- [x] Pack loader that fails to start the server on a malformed or unknown topic id
- [ ] Per topic: a short rule summary shown next to the sheet
- [ ] Per topic: generation hints, the everyday German situations that fit it
      (bakery, Bürgeramt, train ticket, doctor, flat viewing, phone call)
- [ ] Per topic: prerequisites, so a topic cannot be introduced before its base
- [ ] Draft the missing B2, C1 and C2 topics: Genitiv, Relativsätze, Passiv,
      Konjunktiv II past, Futur I and II, general Präteritum, indirect speech,
      Partizipialkonstruktionen, Nominalstil

## 3. Generation

- [ ] API route that asks Claude for a 20 item sheet on one topic
- [ ] Prompt includes: the topic rule, the situation hints, and the recycled words
- [ ] **Validation pass, reject and retry if any of these fail:**
  - [x] exactly 20 items
  - [x] every blank has at least one accepted answer
  - [x] the answer never appears in the visible part of the sentence
  - [ ] the hint words match the blanks
  - [ ] the sentence actually exercises the topic
- [ ] Error tags produced **at generation time**, not inferred later from a wrong answer
- [ ] Cache generated sheets so a sheet can be redone offline
- [ ] Background pre-generation of the next due sheet while you work on the current one

## 4. Anki integration (read only)

- [ ] AnkiConnect client on `localhost:8765`
- [ ] Pull the "Deutsch" deck: word, interval, last review
- [ ] Stale word query: known, but not reviewed in 30+ days (threshold configurable)
- [ ] Feed those words into generation as required vocabulary
- [ ] Degrade gracefully when Anki is not running, never block a session
- [ ] Never write to Anki

## 5. Grading

- [x] Answer normalisation: `ß` and `ss`, `ä` and `ae`, whitespace, trailing punctuation
- [x] Case sensitivity: noun capitalisation counts as wrong, sentence-initial position does not
- [x] Unit tests for every normalisation rule, this is where the bugs will live
- [x] Score to FSRS rating: under 60 Again, 60 to 80 Hard, 80 to 95 Good, above 95 Easy
- [x] Manual override of the computed rating
- [x] `fsrs-rs` scheduling on `topic_state`
- [x] "Also accept" patches the stored sheet permanently and regrades the answer
- [x] Error tag aggregation, so the app can say "dative fails only after two-way prepositions"
- [ ] Leech detection: a topic that keeps lapsing gets flagged, not just rescheduled

## 6. Scheduler behaviour

- [ ] Cap on new topics introduced per day, default 1 or 2
      (A1 alone is 27 topics and 540 sentences, without a cap the due queue drowns you by week two)
- [ ] Respect topic prerequisites when choosing what to introduce
- [ ] Targeted sheets: generate from a weak error tag instead of a whole topic
- [ ] Today view: what is due, what is new, current streak

## 7. UI, "Papier" direction

- [x] Design tokens: paper `#F2ECE1`, sheet `#FDFBF6`, ink `#1E1B16`, hint blue `#27456F`,
      wrong `#A33529`, right `#3D6B49`, due gold `#C8912F`
- [x] Source Serif 4 for German text, Inter for interface chrome
- [x] Sheet view: numbered items, dotted rules between them, hints in italic blue
- [ ] Blank component:
  - [x] ruled underline, no box
  - [x] one constant width for every blank, so length never leaks the answer
  - [x] Tab moves to the next blank, Enter checks the sheet, no mouse needed
  - [x] the correct form appears under the blank on a wrong answer, not in a summary
  - [x] "Also accept" button next to a wrong blank
- [ ] Sidebar: due sheets, recycled words panel
- [ ] Watch the line height cost of 17px serif over 20 items, shrink the type scale if it scrolls too much
- [ ] Settings page driven by the `settings` table
- [x] Review screen after a sheet: score, computed rating, override buttons

## 8. Open questions

- [ ] Recycled words panel: visible during the sheet, or only after?
      (leaning towards after, since seeing the target words makes some items too easy)
- [ ] Should a wrong answer immediately reveal, or only after the whole sheet is checked?
- [ ] One sheet per session, or keep serving sheets until the due queue is empty?

## 9. Before open sourcing

- [x] README with a one command start
- [ ] Ship a prebuilt binary per platform, no Node needed by the user
- [ ] Document how to write a language pack, so `packs/es/` is a pull request and not a fork
- [ ] Make the Claude API key configurable, never committed
