"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import Shell from "@/components/Shell";
import { Centered, Chip, Section } from "@/components/ui/Section";
import { api, type Level, type TopicView } from "@/lib/api";
import styles from "./page.module.css";

const LEVELS: Level[] = [
  "A1",
  "A1+",
  "A2",
  "A2+",
  "B1",
  "B1+",
  "B2",
  "B2+",
  "C1",
  "C1+",
  "C2",
  "Muttersprache",
];

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

  if (error) {
    return (
      <Shell>
        <Centered title="Could not load" body={error} error />
      </Shell>
    );
  }

  return (
    <Shell>
      <h1 className={styles.title}>Curriculum</h1>
      <p className={styles.intro}>
        Levels and topics follow ROADMAP:deutsch by the{" "}
        <a
          href="https://t.me/sheimande"
          target="_blank"
          rel="noopener noreferrer"
          className={styles.source}
        >
          t.me/sheimande
        </a>{" "}
        channel. Numbers on the left are the teaching stage inside the level.
        Topics without a sheet yet are listed but cannot be drilled.
      </p>

      {!topics && <p className="label">Loading topics</p>}

      <div className={styles.levels}>
        {LEVELS.map((level) => {
          const inLevel = topics?.filter((t) => t.cefr === level) ?? [];
          if (inLevel.length === 0) return null;
          return (
            <Section
              key={level}
              title={level}
              action={`${inLevel.length} topics`}
              first
            >
              <div className={styles.rows}>
                {inLevel.map((topic) => (
                  <div key={topic.id} className={styles.row}>
                    <span className={styles.stage}>
                      {String(topic.stage).padStart(2, "0")}
                    </span>
                    <span className={styles.rowTitle}>
                      {topic.has_sheet ? (
                        <Link
                          href={`/sheet/${topic.id}`}
                          className={styles.ready}
                        >
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
            </Section>
          );
        })}
      </div>
    </Shell>
  );
}

function hostOf(url: string): string {
  return new URL(url).hostname.replace(/^www\./, "");
}

function State({ topic }: { topic: TopicView }) {
  if (topic.reps === 0) return <Chip tone="neutral">new</Chip>;
  if (topic.is_due && topic.has_sheet) return <Chip>due</Chip>;
  return <span className={styles.date}>{topic.due ?? ""}</span>;
}
