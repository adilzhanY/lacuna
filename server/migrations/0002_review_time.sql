-- How long a whole review took, in milliseconds.
--
-- Sheet mode does not measure time, so its reviews keep the default of 0 and are
-- excluded from the "minutes studied" figure rather than counted as instant.
alter table review add column elapsed_ms integer not null default 0;
