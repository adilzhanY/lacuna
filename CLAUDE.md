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

## UI direction: "Slot"

One system fused from three sibling projects on 2026-10-12, replacing the Bauwerk copy:
Torq gave the dark set, one loud accent, content on the bare page and big numbers with
a small accent unit; Bauwerk gave mono labels, hairline spec rows, underline tabs and
key hints; Portfolio gave Open Runde, the floating glass pill and the warm light paper.
The blank you type into is the shape it is built on, starting with the logo.

All tokens live in `web/src/app/globals.css`. Never hardcode a colour in a component.

- **Three fonts, one job each.** Space Grotesk speaks (titles, big numbers, the main
  button). Open Runde reads (body and every German sentence, local in
  `web/src/app/fonts/`, OFL). Geist Mono labels (section labels, values, keys, links).
- light: paper `#F2F1EC`, panel `#FBFAF7`, ink `#141417`, muted `#66666C`,
  line `#DCDBD3`, accent `#2545D3`, right `#2F7A47`, wrong `#C2431F`
- dark: paper `#0E0F0E`, panel `#151714`, ink `#F2F4EE`, muted `#9AA294`,
  accent `#8EA4FF`, right `#5AC8A0`, wrong `#E06A5A`
- **One accent.** Cobalt marks focus, progress, your level and the main action, which
  is the only accent fill on a screen. Nothing else is coloured except a right or
  wrong answer.
- **Corners**: 8px for slots and tags, 10px for anything you press, 16px for a card,
  pills only for status (chips, the level badge, the navbar).
- **Cardless pages.** Content sits on the paper with hairlines between rows. A card
  is reserved for the one thing to act on, like the session card on Today.
- **The logo** is the Slot mark (`web/src/components/Logo.tsx`): an ink tile with two
  lines of text and one accent blank. `web/src/app/icon.svg` is the same mark.
- **Motion is GSAP, through `web/src/lib/motion.ts` only.** Pages opt in with data
  attributes (`data-reveal`, `data-grow`, `data-rise`, `data-count`, `data-wave`) and
  `useReveal`; feedback uses `pop`, `shake` and `slideTo`. One ease, short durations,
  and nothing moves when the system asks for reduced motion.
- **Navigation never waits.** The shell lives in `web/src/app/(app)/layout.tsx` so it
  mounts once, and pages read `useData` from `web/src/lib/store.ts`, which hands back
  the last value at once and refreshes it in the background.
- **Every size is in rem, never px**, apart from hairlines under 4px (borders, small
  radii) which stay crisp in px. The root font size in `globals.css` is the zoom dial
  for the whole app: 17px, stepping up to 18, 19 and 21 on wider screens. Media query
  widths stay in px, since rem inside a media query means the browser default.
- **One theme at a time.** Light by default, dark from `prefers-color-scheme`. No
  section inverts.
- Blanks are slots with a heavier bottom edge. Empty ones share one minimum width, so
  their size never leaks the answer length, and grow only to fit what is typed.
- The sheet splits into two columns above 1180px so twenty items fit one screen.
- Tab and Enter both move to the next blank, so 20 items need no mouse. Enter on the
  last blank arms the check and says so, and a second Enter runs it. Enter never
  checks the sheet from the middle, because that ends a run by accident.
- A wrong answer shows the correct form under the blank, not in a summary at the bottom.

## Review mode

The default way to study, at `/review`. One sentence at a time, Enter answers it,
the next arrives on its own. Sheet mode at `/sheet/<topic>` still exists for working
through twenty items at once.

- The clock runs from the sentence appearing to the answer being sent, and is **never
  shown**. Timing feeds the rating only. Showing it would turn practice into a race.
- Each item earns a rating: a mistake is always Again, whatever the clock says. A
  correct answer is Easy, Good or Hard depending on how it compares to a budget worked
  out from that item, not a flat number. See `server/src/review.rs`.
- The topic rating is the mean of the item ratings, mapped through the same thresholds
  a checked sheet uses, with one rule on top: **a perfect rating needs a perfect run**,
  so one mistake caps the session at Good.
- A wrong answer flashes the background red for half a second, shows the correct form,
  and moves on. No retries, no going back.
- Grading happens on the server twice: once per item for immediate feedback, once at
  the end over the same raw answers and timings. The end pass is what gets recorded, so
  the server stays the authority.

## Routes

| Route | What it is |
|---|---|
| `/` | Today: one card with today's reviews, the next new topic, your level's progress and the button into review mode, then the year heatmap |
| `/review` | Review mode, one sentence at a time |
| `/levels/<level>` | One level of the roadmap, with a tab row for all twelve. `/levels` opens your level, `/curriculum` redirects there |
| `/stats` | Dashboard |
| `/sheet/<topic>` | Sheet mode, twenty items at once |

## Due and new

A topic never studied is **new**, not due. Today's session is every due review,
from any level, then at most one new topic a day from your current level (the
`current_level` setting, chosen with "Make this my level" on a level page). When
that level has nothing new left, new topics come from the levels after it.

## Language and content

- The interface is English only. No Russian, no German labels in the chrome.
- German appears only inside exercise content.
- Sentences describe ordinary situations in Germany: bakery, Bürgeramt, train ticket,
  doctor, flat viewing, phone call. Not textbook abstractions.
- **Hints never give the grammar away.** A hint names the word to use (the infinitive,
  the adjective, the pronoun), never its gender, number or case. Stored hints may still
  carry markers like `Frau, f` or `klein, f dat`; `Item::learner_hint` in
  `server/src/sheet.rs` strips them before a sheet leaves the server, and drops a hint
  that only repeats a word the sentence already shows.

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
