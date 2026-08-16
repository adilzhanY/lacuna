"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import Shell from "@/components/Shell";
import { api, type Level, type TopicView } from "@/lib/api";
import styles from "./page.module.css";

const LEVELS: Level[] = ["A1", "A2", "B1", "B2", "C1", "C2"];

export default function Curriculum() {
  const [topics, setTopics] = useState<TopicView[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .topics()
      .then((loaded) => {
        if (!cancelled) setTopics(loaded);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Shell>
      <h1 className={styles.title}>Curriculum</h1>
      <p className={styles.intro}>
        Every topic in the German pack, in the order it should be taught. Numbers on the
        left are the teaching stage inside the level.
      </p>

      {error && <p className={styles.error}>{error}</p>}
      {!topics && !error && <p className="label">Loading topics</p>}

      <div className={styles.levels}>
        {LEVELS.map((level) => {
          const inLevel = topics?.filter((t) => t.cefr === level) ?? [];
          if (inLevel.length === 0) return null;
          return (
            <section key={level}>
              <div className={styles.levelHead}>
                <h2 className={styles.levelName}>{level}</h2>
                <span className="label">{inLevel.length} topics</span>
              </div>
              {inLevel.map((topic) => (
                <div key={topic.id} className={styles.row}>
                  <span className={styles.stage}>{String(topic.stage).padStart(2, "0")}</span>
                  <span className={styles.rowTitle}>
                    {topic.has_sheet ? (
                      <Link href={`/sheet/${topic.id}`} className={styles.ready}>
                        {topic.title}
                      </Link>
                    ) : (
                      topic.title
                    )}
                    <small>{topic.goal}</small>
                  </span>
                  <span
                    className={`${styles.state} ${
                      topic.is_due && topic.has_sheet ? styles.due : ""
                    }`}
                  >
                    {topic.reps === 0
                      ? "new"
                      : topic.is_due
                        ? "due"
                        : `${topic.due ?? ""}`}
                  </span>
                </div>
              ))}
            </section>
          );
        })}
      </div>
    </Shell>
  );
}
