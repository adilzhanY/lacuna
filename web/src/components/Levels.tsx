"use client";

import { ArrowUpRight } from "lucide-react";
import Link from "next/link";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/Button";
import { Centered, Chip } from "@/components/ui/Section";
import { api, type Level, type TopicView } from "@/lib/api";
import { LEVELS, levelShort, levelSlug } from "@/lib/levels";
import { slideTo, useReveal } from "@/lib/motion";
import { setCached, useData } from "@/lib/store";

/** Where the underline sat on the last level page, so the next one slides from it. */
let lastUnderline: { x: number; width: number } | null = null;
import styles from "./Levels.module.css";

/**
 * One level of the roadmap: a row of tabs for all twelve levels on top, the
 * chosen level's topics below. Browsing a level never changes your level, only
 * the explicit button does.
 */
export default function Levels({ level }: { level: Level }) {
  const { data: topics, error: topicsError } = useData("topics");
  const { data: plan, error: planError } = useData("plan");
  const [saveError, setSaveError] = useState<string | null>(null);
  const error = topicsError ?? planError ?? saveError;
  const mine = plan?.level ?? null;
  const next = plan?.new_topics[0]?.id ?? null;
  const root = useRef<HTMLDivElement>(null);
  const tabs = useRef<HTMLElement>(null);
  const underline = useRef<HTMLSpanElement>(null);
  useReveal(root, Boolean(topics), level);

  useEffect(() => {
    const active = tabs.current?.querySelector<HTMLElement>('[aria-current="page"]') ?? null;
    if (!active) return;
    active.scrollIntoView({ block: "nearest", inline: "nearest" });
    slideTo(underline.current, active, lastUnderline);
    lastUnderline = { x: active.offsetLeft, width: active.offsetWidth };
  }, [level]);

  const makeMine = async () => {
    try {
      setCached("plan", await api.setLevel(level));
    } catch (e) {
      setSaveError((e as Error).message);
    }
  };

  if (error) return <Centered title="Could not load" body={error} error />;

  const inLevel = topics?.filter((t) => t.cefr === level) ?? [];
  const started = inLevel.filter((t) => t.reps > 0).length;

  return (
    <div ref={root}>
      <nav aria-label="Levels" className={styles.tabs} ref={tabs}>
        <span className={styles.underline} ref={underline} aria-hidden="true" />
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
              {levelShort(tab)}
              <span className={styles.tabBar}>
                <span style={{ width: `${share}%` }} />
              </span>
              {tab === mine && <span className={styles.mineDot} aria-label="your level" />}
            </Link>
          );
        })}
      </nav>

      <header className={styles.head} data-reveal>
        <div>
          <h1 className={styles.title}>{level}</h1>
          <p className={styles.intro}>
            {topics
              ? `${inLevel.length} topics · ${started} started`
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
          <div
            key={topic.id}
            data-reveal
            className={`${styles.row} ${topic.id === next ? styles.next : ""}`}
          >
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
                      <ArrowUpRight size={12} />
                    </a>
                  )}
                  {topic.flexible && (
                    <span className={styles.flexible}>any order</span>
                  )}
                </span>
              )}
            </span>
            <State topic={topic} next={topic.id === next} />
          </div>
        ))}
      </div>

      <p className={styles.source} data-reveal>
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
    </div>
  );
}

function hostOf(url: string): string {
  return new URL(url).hostname.replace(/^www\./, "");
}

function State({ topic, next }: { topic: TopicView; next: boolean }) {
  if (next) return <Chip>next</Chip>;
  if (topic.reps === 0) return <Chip tone="neutral">new</Chip>;
  if (topic.is_due) return <Chip>due</Chip>;
  return <span className={styles.date}>{topic.due ?? ""}</span>;
}
