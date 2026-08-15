"use client";

import { useEffect, useState } from "react";

import { api, type DayPoint, type Stats as StatsData } from "@/lib/api";
import styles from "./Stats.module.css";

export default function Stats() {
  const [stats, setStats] = useState<StatsData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .stats()
      .then((loaded) => {
        if (!cancelled) setStats(loaded);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) return <p className={styles.error}>{error}</p>;
  if (!stats) return <p className="label">Loading statistics</p>;

  const nothingYet = stats.reviews_total === 0;

  return (
    <div className={styles.page}>
      <h2 className={styles.title}>Statistics</h2>
      <p className={styles.intro}>
        Everything here comes from sheets you have actually answered. Accuracy counts
        blanks, not sheets, so one bad sheet of twenty moves it by the right amount.
      </p>

      <div className={styles.tiles}>
        <Tile
          label="Accuracy"
          value={nothingYet ? "-" : `${Math.round(stats.accuracy * 100)}%`}
          note={
            nothingYet
              ? "no answers yet"
              : `${stats.blanks_correct} of ${stats.blanks_total} blanks`
          }
        />
        <Tile
          label="Streak"
          value={`${stats.streak_days}`}
          note={stats.streak_days === 1 ? "day in a row" : "days in a row"}
        />
        <Tile
          label="Topics started"
          value={`${stats.topics_studied}`}
          note={`of ${stats.topics_total} in the pack`}
        />
        <Tile label="Sheets checked" value={`${stats.reviews_total}`} note="all time" />
        <Tile
          label="Due now"
          value={`${stats.topics_due}`}
          note={stats.topics_due === 1 ? "topic waiting" : "topics waiting"}
        />
      </div>

      <div className={styles.grid2}>
        <Section title="Last 30 days" note="sheets checked per day">
          <Columns points={stats.activity} kind="past" />
        </Section>

        <Section title="Next 14 days" note="topics falling due">
          <Columns points={stats.forecast} kind="future" />
        </Section>
      </div>

      <Section title="Progress by level" note="topics started">
        <div className={styles.rows}>
          {stats.by_level.map((level) => (
            <div key={level.level} className={styles.row}>
              <span className={styles.rowLabel}>
                {level.level} <small>&middot; {level.total} topics</small>
              </span>
              <div
                className={styles.track}
                role="img"
                aria-label={`${level.studied} of ${level.total} topics started at ${level.level}`}
              >
                <div
                  className={styles.fill}
                  style={{ width: `${(level.studied / level.total) * 100}%` }}
                />
              </div>
              <span className={styles.rowValue}>
                {level.studied} / {level.total}
              </span>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Weakest points" note="wrong answers by what they were testing">
        {stats.weakest.length === 0 ? (
          <p className={styles.empty}>Nothing wrong yet, or nothing answered yet.</p>
        ) : (
          <div className={styles.rows}>
            {stats.weakest.map((weak) => (
              <div key={weak.tag} className={styles.row}>
                <span className={styles.rowLabel}>{prettyTag(weak.tag)}</span>
                <div
                  className={styles.track}
                  role="img"
                  aria-label={`${weak.tag}, ${weak.count} wrong`}
                >
                  <div
                    className={`${styles.fill} ${styles.weak}`}
                    style={{
                      width: `${(weak.count / stats.weakest[0].count) * 100}%`,
                    }}
                  />
                </div>
                <span className={styles.rowValue}>{weak.count}</span>
              </div>
            ))}
          </div>
        )}
      </Section>

      <Section title="Hardest topics" note="lowest accuracy first">
        {stats.hardest.length === 0 ? (
          <p className={styles.empty}>No topic has been reviewed yet.</p>
        ) : (
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Topic</th>
                <th>Level</th>
                <th className={styles.numeric}>Accuracy</th>
                <th className={styles.numeric}>Sheets</th>
                <th className={styles.numeric}>Lapses</th>
              </tr>
            </thead>
            <tbody>
              {stats.hardest.map((topic) => (
                <tr key={topic.id}>
                  <td>{topic.title}</td>
                  <td>{topic.cefr}</td>
                  <td
                    className={`${styles.numeric} ${topic.accuracy < 0.6 ? styles.bad : styles.good}`}
                  >
                    {Math.round(topic.accuracy * 100)}%
                  </td>
                  <td className={styles.numeric}>{topic.reviews}</td>
                  <td className={styles.numeric}>{topic.lapses}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Section>
    </div>
  );
}

function Tile({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className={styles.tile}>
      <p className="label">{label}</p>
      <p className={styles.tileValue}>{value}</p>
      <p className={styles.tileNote}>{note}</p>
    </div>
  );
}

function Section({
  title,
  note,
  children,
}: {
  title: string;
  note: string;
  children: React.ReactNode;
}) {
  return (
    <section className={styles.section}>
      <div className={styles.sectionHead}>
        <h3 className={styles.sectionTitle}>{title}</h3>
        <span className="label">{note}</span>
      </div>
      {children}
    </section>
  );
}

/**
 * One bar per day. The scale is shared across the whole series, and an empty day
 * keeps its column as a hairline so gaps stay visible.
 */
function Columns({ points, kind }: { points: DayPoint[]; kind: "past" | "future" }) {
  const max = Math.max(1, ...points.map((p) => p.count));
  const first = points[0];
  const last = points[points.length - 1];

  return (
    <>
      <div className={styles.columns}>
        {points.map((point) => (
          <div key={point.date} className={styles.column}>
            <span className={styles.tip}>
              {formatDay(point.date)}: {point.count}
              {point.accuracy !== null && ` · ${Math.round(point.accuracy * 100)}%`}
            </span>
            <div
              className={`${styles.bar} ${kind === "future" ? styles.future : ""} ${
                point.count === 0 ? styles.zero : ""
              }`}
              style={{ height: `${(point.count / max) * 100}%` }}
              role="img"
              aria-label={`${point.date}: ${point.count}`}
            />
          </div>
        ))}
      </div>
      <div className={styles.axis}>
        <span>{formatDay(first.date)}</span>
        <span>peak {max}</span>
        <span>{formatDay(last.date)}</span>
      </div>
    </>
  );
}

function formatDay(iso: string) {
  const [, month, day] = iso.split("-");
  return `${day}.${month}`;
}

/** `trigger:preposition_mit` reads as `trigger: preposition mit`. */
function prettyTag(tag: string) {
  const [kind, detail] = tag.split(":");
  return `${kind}: ${detail.replace(/_/g, " ")}`;
}
