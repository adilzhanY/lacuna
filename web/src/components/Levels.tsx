"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Centered, Chip } from "@/components/ui/Section";
import { api, type Level, type TopicView } from "@/lib/api";
import { LEVELS, levelShort, levelSlug } from "@/lib/levels";
import styles from "./Levels.module.css";

/**
 * One level of the roadmap: a row of tabs for all twelve levels on top, the
 * chosen level's topics below. Browsing a level never changes your level, only
 * the explicit button does.
 */
export default function Levels({ level }: { level: Level }) {
  const [topics, setTopics] = useState<TopicView[] | null>(null);
  const [mine, setMine] = useState<Level | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([api.topics(), api.plan()])
      .then(([loaded, plan]) => {
        if (cancelled) return;
        setTopics(loaded);
        setMine(plan.level);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const makeMine = async () => {
    try {
      const plan = await api.setLevel(level);
      setMine(plan.level);
    } catch (e) {
      setError((e as Error).message);
    }
  };

  if (error) return <Centered title="Could not load" body={error} error />;

  const inLevel = topics?.filter((t) => t.cefr === level) ?? [];
  const started = inLevel.filter((t) => t.reps > 0).length;

  return (
    <>
      <nav aria-label="Levels" className={styles.tabs}>
        {LEVELS.map((tab) => {
          const all = topics?.filter((t) => t.cefr === tab) ?? [];
          const done = all.filter((t) => t.reps > 0).length;
          const share = all.length ? (done / all.length) * 100 : 0;
          return (
            <Link
              key={tab}
              href={`/levels/${levelSlug(tab)}`}
              title={tab}
              aria-current={tab === level ? "page" : undefined}
              className={`${styles.tab} ${tab === level ? styles.active : ""}`}
            >
              <span>{levelShort(tab)}</span>
              <span className={styles.tabBar}>
                <span style={{ width: `${share}%` }} />
              </span>
              {tab === mine && <span className={styles.mineDot} aria-label="your level" />}
            </Link>
          );
        })}
      </nav>

      <header className={styles.head}>
        <div>
          <h1 className={styles.title}>{level}</h1>
          <p className={styles.intro}>
            {topics
              ? `${inLevel.length} topics, ${started} started.`
              : "Loading topics"}
          </p>
        </div>
        {mine === level ? (
          <Chip>Your level</Chip>
        ) : (
          mine && <Button onClick={makeMine}>Make this my level</Button>
        )}
      </header>

      <div className={styles.rows}>
        {inLevel.map((topic) => (
          <div key={topic.id} className={styles.row}>
            <span className={styles.stage}>
              {String(topic.stage).padStart(2, "0")}
            </span>
            <span className={styles.rowTitle}>
              {topic.has_sheet ? (
                <Link href={`/sheet/${topic.id}`} className={styles.ready}>
                  {topic.title}
                </Link>
              ) : (
                topic.title
              )}
              <small>{topic.goal}</small>
              {(topic.link || topic.flexible) && (
                <span className={styles.meta}>
                  {topic.link && (
                    <a
                      href={topic.link}
                      target="_blank"
                      rel="noopener noreferrer"
                      className={styles.link}
                    >
                      {hostOf(topic.link)}
                    </a>
                  )}
                  {topic.flexible && (
                    <span className={styles.flexible}>
                      can come before or after this level
                    </span>
                  )}
                </span>
              )}
            </span>
            <State topic={topic} />
          </div>
        ))}
      </div>

      <p className={styles.source}>
        Levels and topics follow ROADMAP:deutsch by the{" "}
        <a
          href="https://t.me/sheimande"
          target="_blank"
          rel="noopener noreferrer"
          className={styles.link}
        >
          t.me/sheimande
        </a>{" "}
        channel.
      </p>
    </>
  );
}

function hostOf(url: string): string {
  return new URL(url).hostname.replace(/^www\./, "");
}

function State({ topic }: { topic: TopicView }) {
  if (topic.reps === 0) return <Chip tone="neutral">new</Chip>;
  if (topic.is_due) return <Chip>due</Chip>;
  return <span className={styles.date}>{topic.due ?? ""}</span>;
}
