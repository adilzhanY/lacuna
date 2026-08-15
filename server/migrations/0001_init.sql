-- lacuna schema, version 1.
--
-- Dates are ISO 8601 text, which SQLite compares correctly as strings.
-- Sheets are stored as validated JSON: they are generated documents, and
-- shredding them into rows would buy nothing and cost the ability to hand a
-- sheet back exactly as it was answered.

create table topic (
    id       text primary key,
    cefr     text    not null,
    stage    integer not null,
    category text    not null,
    title    text    not null,
    goal     text    not null,
    status   text    not null
);

create table topic_state (
    topic_id    text primary key references topic (id) on delete cascade,
    stability   real,
    difficulty  real,
    due         text,
    last_review text,
    reps        integer not null default 0,
    lapses      integer not null default 0
);

create table sheet (
    id         integer primary key autoincrement,
    topic_id   text not null references topic (id),
    language   text not null,
    body       text not null,
    created_at text not null
);

create index sheet_topic_idx on sheet (topic_id);

create table review (
    id          integer primary key autoincrement,
    sheet_id    integer not null references sheet (id) on delete cascade,
    topic_id    text    not null,
    reviewed_at text    not null,
    correct     integer not null,
    total       integer not null,
    score       real    not null,
    rating      text    not null
);

create index review_topic_idx on review (topic_id);

create table answer (
    id        integer primary key autoincrement,
    review_id integer not null references review (id) on delete cascade,
    blank_id  text    not null,
    given     text    not null,
    expected  text    not null,
    correct   integer not null,
    -- Space separated error tags, empty when the answer was right.
    tags      text    not null default ''
);

create index answer_review_idx on answer (review_id);

create table settings (
    key   text primary key,
    value text not null
);
