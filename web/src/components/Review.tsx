"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";

import {
  api,
  type ClientItem,
  type ClientSheet,
  type FinishResponse,
  type ItemAttempt,
} from "@/lib/api";
import styles from "./Review.module.css";

/*
 * The red flash itself is half a second, defined by the `flash-wrong` keyframe
 * in Review.module.css. These two hold the sentence in place afterwards.
 */
/** How long a wrong answer and its correction stay readable before moving on. */
const WRONG_HOLD_MS = 1600;
/** A correct answer only needs a beat before the next sentence. */
const RIGHT_HOLD_MS = 420;

type Phase = "typing" | "right" | "wrong";

export default function Review() {
  const [sheet, setSheet] = useState<ClientSheet | null>(null);
  const [remaining, setRemaining] = useState(0);
  const [index, setIndex] = useState(0);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [phase, setPhase] = useState<Phase>("typing");
  const [correction, setCorrection] = useState<string | null>(null);
  const [done, setDone] = useState<FinishResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  /** When the current sentence appeared. The reader never sees this. */
  const shownAt = useRef<number>(0);
  const attempts = useRef<ItemAttempt[]>([]);
  const inputs = useRef<Record<string, HTMLInputElement | null>>({});
  const busy = useRef(false);

  const load = useCallback(() => {
    api
      .reviewNext()
      .then((queue) => {
        attempts.current = [];
        setSheet(queue.sheet);
        setRemaining(queue.remaining);
        setIndex(0);
        setAnswers({});
        setCorrection(null);
        setPhase("typing");
        setDone(null);
      })
      .catch((e: Error) => setError(e.message));
  }, []);

  useEffect(() => {
    let cancelled = false;
    api
      .reviewNext()
      .then((queue) => {
        if (cancelled) return;
        setSheet(queue.sheet);
        setRemaining(queue.remaining);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const item: ClientItem | undefined = sheet?.items[index];
  const blankIds =
    item?.segments.filter((s) => s.type === "blank").map((s) => s.id) ?? [];

  // The clock starts when the sentence is on screen, and the first blank takes
  // focus so the run never needs the mouse.
  useEffect(() => {
    if (!item || phase !== "typing") return;
    shownAt.current = performance.now();
    const first = blankIds[0];
    if (first) inputs.current[first]?.focus();
    // blankIds is derived from item, so item and phase are the real inputs here.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item, phase]);

  const submitItem = useCallback(async () => {
    if (!sheet || !item || busy.current) return;
    busy.current = true;
    const elapsed = performance.now() - shownAt.current;
    const given: Record<string, string> = {};
    blankIds.forEach((id) => {
      given[id] = answers[id] ?? "";
    });

    try {
      const verdict = await api.reviewItem(sheet.sheet_id, item.n, elapsed, given);
      attempts.current.push({ n: item.n, elapsed_ms: Math.round(elapsed), answers: given });

      if (verdict.correct) {
        setPhase("right");
      } else {
        const wrong = verdict.results.find((r) => r.verdict === "wrong");
        setCorrection(wrong && "expected" in wrong ? wrong.expected : null);
        setPhase("wrong");
      }

      const hold = verdict.correct ? RIGHT_HOLD_MS : WRONG_HOLD_MS;
      window.setTimeout(async () => {
        busy.current = false;
        setCorrection(null);
        if (index + 1 < sheet.items.length) {
          setPhase("typing");
          setIndex((i) => i + 1);
          return;
        }
        try {
          setDone(await api.reviewFinish(sheet.sheet_id, attempts.current));
        } catch (e) {
          setError((e as Error).message);
        }
      }, hold);
    } catch (e) {
      busy.current = false;
      setError((e as Error).message);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sheet, item, answers, index]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>, blankId: string) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    if (phase !== "typing") return;

    const position = blankIds.indexOf(blankId);
    const next = blankIds[position + 1];
    if (next) {
      inputs.current[next]?.focus();
      inputs.current[next]?.select();
      return;
    }
    void submitItem();
  };

  if (error) {
    return (
      <div className={styles.screen}>
        <div className={styles.stage}>
          <div>
            <p className={styles.error}>{error}</p>
            <Link href="/" className={styles.leave}>
              back to today
            </Link>
          </div>
        </div>
      </div>
    );
  }

  // Finished a topic.
  if (done) {
    return (
      <div className={styles.screen}>
        <div className={styles.stage}>
          <div className={styles.summary}>
            <p className="label">{done.topic_title}</p>
            <p className={`${styles.rating} ${styles[done.rating]}`}>{done.rating}</p>
            <p className={styles.line}>
              {done.correct} of {done.total} blanks right. Back in {done.interval_days}{" "}
              {done.interval_days === 1 ? "day" : "days"}, on {done.due}.
              {done.remaining > 0
                ? ` ${done.remaining} ${done.remaining === 1 ? "topic" : "topics"} still due.`
                : " Nothing else due today."}
            </p>
            <div className={styles.actions}>
              {done.remaining > 0 ? (
                <button type="button" className={styles.button} onClick={load}>
                  Next topic
                </button>
              ) : (
                <Link href="/" className={styles.button}>
                  Done for today
                </Link>
              )}
              <Link href="/" className={`${styles.button} ${styles.ghost}`}>
                Stop here
              </Link>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!sheet) {
    return (
      <div className={styles.screen}>
        <div className={styles.stage}>
          <div className={styles.empty}>
            <p className="label">Nothing due</p>
            <Link href="/" className={styles.leave}>
              back to today
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!item) return null;

  const progress = (index / sheet.items.length) * 100;

  return (
    <div className={`${styles.screen} ${phase === "wrong" ? styles.wrong : ""}`}>
      <div className={styles.top}>
        <span className={styles.topic}>{sheet.topic_title}</span>
        <div className={styles.progress}>
          <i style={{ width: `${progress}%` }} />
        </div>
        <span className={styles.topic}>
          {index + 1}/{sheet.items.length}
          {remaining > 0 && ` · ${remaining} left today`}
        </span>
        <Link href="/" className={styles.leave}>
          leave
        </Link>
      </div>

      <div className={styles.stage}>
        <div className={styles.card}>
          {/* The key restarts the arrival animation on every sentence. */}
          <p className={styles.sentence} key={item.n}>
            {item.segments.map((segment, position) =>
              segment.type === "text" ? (
                <span key={position}>{segment.text}</span>
              ) : (
                <input
                  key={segment.id}
                  ref={(element) => {
                    inputs.current[segment.id] = element;
                  }}
                  className={`${styles.blank} ${
                    phase === "right" ? styles.ok : phase === "wrong" ? styles.bad : ""
                  }`}
                  aria-label="missing word"
                  value={answers[segment.id] ?? ""}
                  disabled={phase !== "typing"}
                  autoComplete="off"
                  autoCorrect="off"
                  spellCheck={false}
                  onChange={(event) =>
                    setAnswers((a) => ({ ...a, [segment.id]: event.target.value }))
                  }
                  onKeyDown={(event) => onKeyDown(event, segment.id)}
                />
              ),
            )}
          </p>

          {item.hint && <span className={styles.hint}>{item.hint}</span>}

          {phase === "wrong" && correction && (
            <span className={styles.correction} role="status">
              correct form <b>{correction}</b>
            </span>
          )}

          {phase === "typing" && <span className={styles.keys}>enter to answer</span>}
        </div>
      </div>
    </div>
  );
}
