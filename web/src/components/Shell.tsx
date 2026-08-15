"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import { api, type TopicView } from "@/lib/api";
import styles from "./Shell.module.css";

/**
 * The workbook frame: sheets on the left, the open sheet on the right.
 */
export default function Shell({
  activeTopicId,
  children,
}: {
  activeTopicId?: string;
  children: React.ReactNode;
}) {
  const [due, setDue] = useState<TopicView[] | null>(null);

  useEffect(() => {
    api.today().then(setDue).catch(() => setDue([]));
  }, [activeTopicId]);

  return (
    <div className={styles.shell}>
      <aside className={styles.side}>
        <Link href="/">
          <h1 className={styles.brand}>lacuna</h1>
        </Link>
        <p className={styles.tagline}>Fill in what is missing.</p>

        <div className={styles.group}>
          <p className={`label ${styles.groupTitle}`}>Due today</p>
          {due === null && <p className={styles.empty}>Loading</p>}
          {due?.length === 0 && <p className={styles.empty}>Nothing due. Rest.</p>}
          {due?.map((topic) => (
            <Link
              key={topic.id}
              href={`/sheet/${topic.id}`}
              className={`${styles.item} ${topic.id === activeTopicId ? styles.active : ""}`}
            >
              {topic.title}
              <small>
                {topic.cefr} &middot; {topic.is_new ? "new" : `${topic.reps} reviews`}
              </small>
            </Link>
          ))}
        </div>

        <div className={styles.group}>
          <p className={`label ${styles.groupTitle}`}>Workbook</p>
          <Link href="/" className={styles.item}>
            All topics
          </Link>
          <Link href="/stats" className={styles.item}>
            Statistics
          </Link>
        </div>
      </aside>

      <main className={styles.main}>{children}</main>
    </div>
  );
}
