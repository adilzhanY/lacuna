"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import Shell from "@/components/Shell";
import { api, type Level, type TopicView } from "@/lib/api";
import styles from "./page.module.css";

const LEVELS: Level[] = ["A1", "A2", "B1", "B2", "C1", "C2"];

export default function Home() {
  const [topics, setTopics] = useState<TopicView[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.topics().then(setTopics).catch((e: Error) => setError(e.message));
  }, []);

  return (
    <Shell>
      <h2 className={styles.title}>The curriculum</h2>
      <p className={styles.intro}>
        Every topic in the German pack, in the order it should be taught. A topic with a
        sheet can be opened now. The rest are waiting on generation.
      </p>

      {error && <p className={styles.error}>{error}</p>}
      {!topics && !error && <p className="label">Loading topics</p>}

      {LEVELS.map((level) => {
        const inLevel = topics?.filter((t) => t.cefr === level) ?? [];
        if (inLevel.length === 0) return null;
        return (
          <section key={level} className={styles.level}>
            <div className={styles.levelHead}>
              <h3 className={styles.levelName}>{level}</h3>
              <span className="label">{inLevel.length} topics</span>
            </div>
            {inLevel.map((topic) => (
              <div key={topic.id} className={styles.row}>
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
                {topic.is_due && topic.has_sheet && <span className={styles.badge}>due</span>}
                <span className={styles.state}>
                  {topic.reps === 0
                    ? "not studied"
                    : `${topic.reps} reviews${topic.due ? `, due ${topic.due}` : ""}`}
                </span>
              </div>
            ))}
          </section>
        );
      })}
    </Shell>
  );
}
