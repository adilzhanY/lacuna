"use client";

import { ArrowRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import Shell from "@/components/Shell";
import { LinkButton } from "@/components/ui/Button";
import { Centered, ReadOnly, Section } from "@/components/ui/Section";
import { api, type DayPoint, type Stats } from "@/lib/api";
import styles from "./page.module.css";

const WEEKDAYS = ["M", "", "W", "", "F", "", "S"];

export default function Today() {
  const [stats, setStats] = useState<Stats | null>(null);
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

  const weeks = useMemo(() => chunk(stats?.year ?? [], 7), [stats]);

  if (error) {
    return (
      <Shell>
        <Centered title="Could not load" body={error} error />
      </Shell>
    );
  }
  if (!stats) {
    return (
      <Shell>
        <p className="label">Loading</p>
      </Shell>
    );
  }

  const due = stats.topics_due;
  const minutes = stats.today_ms / 60000;
  const perBlank =
    stats.today_blanks > 0 ? stats.today_ms / stats.today_blanks / 1000 : 0;

  return (
    <Shell>
      <div className={styles.page}>
        <section className={styles.hero}>
          <div className={styles.heroText}>
            <h1 className={styles.title}>
              {due > 0
                ? `${due} ${due === 1 ? "topic" : "topics"} to review`
                : "Nothing due today"}
            </h1>
            <p className={styles.sub}>
              {due > 0
                ? "One sentence at a time. Type the missing word and press Enter."
                : "Everything is scheduled ahead. Come back tomorrow, or open the curriculum to start a topic early."}
            </p>
          </div>
          {due > 0 ? (
            <LinkButton
              href="/review"
              variant="primary"
              large
              icon={<ArrowRight size={16} />}
            >
              Start review
            </LinkButton>
          ) : (
            <LinkButton href="/curriculum" large>
              Open the curriculum
            </LinkButton>
          )}
        </section>

        <div className={styles.grid}>
          <div className={styles.card}>
            <Section title="This year" action="sheets per day" first>
              <p className={styles.todayLine}>
                {stats.today_blanks > 0 ? (
                  <>
                    Studied <b>{stats.today_blanks}</b> blanks in{" "}
                    {minutes.toFixed(1)} minutes today{" "}
                    <span className={styles.quiet}>
                      ({perBlank.toFixed(1)}s each)
                    </span>
                  </>
                ) : (
                  <span className={styles.quiet}>
                    Nothing studied yet today
                  </span>
                )}
              </p>

              <div className={styles.heatWrap}>
                <div className={styles.days}>
                  {WEEKDAYS.map((day, index) => (
                    <span key={index}>{day}</span>
                  ))}
                </div>
                <div>
                  <div className={styles.weeks}>
                    {weeks.map((week, index) => (
                      <div key={index} className={styles.week}>
                        {week.map((day) => (
                          <span
                            key={day.date}
                            className={`${styles.cell} ${levelClass(day.count)} ${
                              day.date === stats.today ? styles.today : ""
                            }`}
                            title={`${day.date}: ${day.count} ${
                              day.count === 1 ? "sheet" : "sheets"
                            }`}
                          />
                        ))}
                      </div>
                    ))}
                  </div>
                  <div className={styles.months}>
                    {monthLabels(weeks).map((label, index) => (
                      <span key={index} className={styles.month}>
                        {label}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            </Section>
          </div>

          <div className={styles.card}>
            <Section title="Showing up" first>
              <div className={styles.rows}>
                <ReadOnly
                  label="Days learned"
                  value={`${Math.round(stats.days_learned * 100)}%`}
                />
                <ReadOnly
                  label="Current streak"
                  value={`${stats.streak_days} days`}
                />
                <ReadOnly
                  label="Longest streak"
                  value={`${stats.longest_streak} days`}
                />
                <ReadOnly
                  label="Accuracy"
                  value={`${Math.round(stats.accuracy * 100)}%`}
                />
                <ReadOnly
                  label="Topics started"
                  value={`${stats.topics_studied} of ${stats.topics_total}`}
                />
              </div>
            </Section>
          </div>
        </div>
      </div>
    </Shell>
  );
}

function chunk(days: DayPoint[], size: number): DayPoint[][] {
  const out: DayPoint[][] = [];
  for (let i = 0; i < days.length; i += size) {
    out.push(days.slice(i, i + size));
  }
  return out;
}

/** Four steps, so a heavy day is visibly heavier than a single sheet. */
function levelClass(count: number) {
  if (count === 0) return "";
  if (count === 1) return styles.l1;
  if (count <= 3) return styles.l2;
  if (count <= 6) return styles.l3;
  return styles.l4;
}

/** A month name over the first week that starts in it, blank everywhere else. */
function monthLabels(weeks: DayPoint[][]) {
  const names = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  let previous = -1;
  return weeks.map((week) => {
    const month = new Date(`${week[0].date}T00:00:00`).getMonth();
    if (month !== previous) {
      previous = month;
      return names[month];
    }
    return "";
  });
}
