"use client";

import { Flame } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";

import Shell from "@/components/Shell";
import { LinkButton } from "@/components/ui/Button";
import { Centered, ReadOnly } from "@/components/ui/Section";
import { api, type DayPoint, type Stats, type TodayPlan } from "@/lib/api";
import { levelSlug } from "@/lib/levels";
import styles from "./page.module.css";

const WEEKDAYS = ["M", "", "W", "", "F", "", "S"];
const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTH_NAMES = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/** Every sheet is twenty sentences, so a topic in the queue is twenty to answer. */
const SENTENCES_PER_TOPIC = 20;

export default function Today() {
  const router = useRouter();
  const [stats, setStats] = useState<Stats | null>(null);
  const [plan, setPlan] = useState<TodayPlan | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([api.stats(), api.plan()])
      .then(([loadedStats, loadedPlan]) => {
        if (cancelled) return;
        setStats(loadedStats);
        setPlan(loadedPlan);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const queue = useMemo(
    () => (plan ? [...plan.reviews, ...plan.new_topics] : []),
    [plan],
  );

  // Enter starts the session from anywhere on the page, as the key hint says.
  useEffect(() => {
    if (queue.length === 0) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Enter" || event.defaultPrevented) return;
      const target = event.target as HTMLElement | null;
      if (target?.closest("a, button, input, textarea")) return;
      router.push("/review");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [queue, router]);

  const weeks = useMemo(() => chunk(stats?.year ?? [], 7), [stats]);

  if (error) {
    return (
      <Shell>
        <Centered title="Could not load" body={error} error />
      </Shell>
    );
  }
  if (!stats || !plan) {
    return (
      <Shell>
        <p className="label">Loading</p>
      </Shell>
    );
  }

  const reviews = plan.reviews.length;
  const fresh = plan.new_topics[0];
  const levelHref = `/levels/${levelSlug(plan.level)}`;
  const date = new Date(`${stats.today}T00:00:00`);
  const lastWeek = stats.year.slice(-7);
  const activeDays = lastWeek.filter((day) => day.count > 0).length;
  const categories = [...new Set(queue.map((topic) => topic.category))].slice(0, 3);
  const minutes = stats.today_ms / 60000;
  const perBlank =
    stats.today_blanks > 0 ? stats.today_ms / stats.today_blanks / 1000 : 0;

  return (
    <Shell>
      <header className={styles.dayHead}>
        <h1 className={styles.day}>
          {DAY_NAMES[date.getDay()]}
          {stats.streak_days > 0 && (
            <span className={styles.streak} title="Days in a row">
              <Flame size={20} />
              {stats.streak_days}
            </span>
          )}
        </h1>
        <p className={styles.date}>
          {date.getDate()} {MONTH_NAMES[date.getMonth()]}
        </p>
      </header>

      <div className={styles.grid}>
        <div className={styles.main}>
          <section className={styles.hero}>
            <span className="label">
              Today · <span className={styles.accent}>Level {plan.level}</span>
            </span>
            <h2 className={styles.title}>{headline(reviews, fresh?.title)}</h2>
            <p className={styles.meta}>
              {queue.length > 0
                ? `${queue.length} ${queue.length === 1 ? "topic" : "topics"} · ${
                    queue.length * SENTENCES_PER_TOPIC
                  } sentences · one at a time`
                : "Everything is scheduled ahead. Come back tomorrow, or browse your level and start a topic early."}
            </p>
            {categories.length > 0 && (
              <div className={styles.tags}>
                {categories.map((category) => (
                  <span key={category} className={styles.tag}>
                    {category}
                  </span>
                ))}
              </div>
            )}
            {queue.length > 0 ? (
              <LinkButton
                href="/review"
                variant="primary"
                large
                className={styles.start}
              >
                Start session
                <span className={`kbd ${styles.keyOnAccent}`}>Enter</span>
              </LinkButton>
            ) : (
              <LinkButton href={levelHref} large className={styles.start}>
                Browse {plan.level}
              </LinkButton>
            )}
          </section>

          <section className={styles.block}>
            <span className="label">
              This week ·{" "}
              <span className={styles.accent}>
                {activeDays} of 7 days
              </span>
            </span>
            <div className={styles.week}>
              {lastWeek.map((day) => {
                const when = new Date(`${day.date}T00:00:00`);
                const isToday = day.date === stats.today;
                return (
                  <div
                    key={day.date}
                    className={`${styles.weekDay} ${day.count > 0 ? styles.did : ""} ${
                      isToday ? styles.now : ""
                    }`}
                    title={`${day.date}: ${day.count} ${day.count === 1 ? "sheet" : "sheets"}`}
                  >
                    <small>{DAY_NAMES[when.getDay()][0]}</small>
                    <b>{when.getDate()}</b>
                    <small>{isToday ? "today" : day.count > 0 ? day.count : "-"}</small>
                  </div>
                );
              })}
            </div>
          </section>
        </div>

        <aside className={styles.side}>
          <span className="label">Where you stand</span>
          <div className={styles.nums}>
            <a href={levelHref} className={styles.num}>
              <b>
                {plan.level_started}
                <em>/{plan.level_total}</em>
              </b>
              <span>{plan.level} topics</span>
              <Bar share={plan.level_total ? plan.level_started / plan.level_total : 0} />
            </a>
            <div className={styles.num}>
              <b>
                {Math.round(stats.accuracy * 100)}
                <em>%</em>
              </b>
              <span>accuracy</span>
              <Bar share={stats.accuracy} />
            </div>
            <div className={styles.num}>
              <b>
                {stats.streak_days}
                <em>d</em>
              </b>
              <span>streak, best {stats.longest_streak}</span>
              <Bar
                share={stats.longest_streak ? stats.streak_days / stats.longest_streak : 0}
              />
            </div>
          </div>

          <div className={styles.block}>
            <span className="label">Today so far</span>
            <div className={styles.rows}>
              <ReadOnly label="Blanks answered" value={String(stats.today_blanks)} />
              <ReadOnly label="Time" value={`${minutes.toFixed(1)} min`} />
              <ReadOnly
                label="Per blank"
                value={perBlank > 0 ? `${perBlank.toFixed(1)} s` : "-"}
              />
              <ReadOnly
                label="Topics started"
                value={`${stats.topics_studied} of ${stats.topics_total}`}
              />
            </div>
          </div>
        </aside>
      </div>

      <section className={styles.block}>
        <span className="label">
          This year · {Math.round(stats.days_learned * 100)}% of days
        </span>
        <div className={styles.heatWrap}>
          <div className={styles.days}>
            {WEEKDAYS.map((day, index) => (
              <span key={index}>{day}</span>
            ))}
          </div>
          <div>
            <div className={styles.weeks}>
              {weeks.map((week, index) => (
                <div key={index} className={styles.weekCol}>
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
      </section>
    </Shell>
  );
}

/** A thin progress line under a big number. */
function Bar({ share }: { share: number }) {
  return (
    <span className={styles.bar}>
      <span style={{ width: `${Math.min(1, Math.max(0, share)) * 100}%` }} />
    </span>
  );
}

/** What the session holds, said the way a person would say it. */
function headline(reviews: number, fresh: string | undefined): string {
  const r = `${reviews} ${reviews === 1 ? "review" : "reviews"}`;
  if (reviews > 0 && fresh) return `${r}, then ${fresh}`;
  if (reviews > 0) return `${r} today`;
  if (fresh) return `New today: ${fresh}`;
  return "Nothing due today";
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
