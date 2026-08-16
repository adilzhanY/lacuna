"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  api,
  type BlankResult,
  type CheckResponse,
  type ClientSheet,
  type Rating,
  type TopicView,
} from "@/lib/api";
import styles from "./Sheet.module.css";

const RATINGS: Rating[] = ["again", "hard", "good", "easy"];

export default function Sheet({ topicId }: { topicId: string }) {
  const [sheet, setSheet] = useState<ClientSheet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [checked, setChecked] = useState<CheckResponse | null>(null);
  const [patched, setPatched] = useState<Record<string, true>>({});
  const [due, setDue] = useState<TopicView[]>([]);
  /** True once Enter has been pressed on the last blank, so the next one checks. */
  const [armed, setArmed] = useState(false);
  const inputs = useRef<Record<string, HTMLInputElement | null>>({});

  // The page remounts this component per topic (see the `key` on <Sheet>), so
  // the effect only has to fetch. Nothing here resets state by hand.
  useEffect(() => {
    let cancelled = false;
    api
      .sheet(topicId)
      .then((loaded) => {
        if (!cancelled) setSheet(loaded);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    api
      .today()
      .then((topics) => {
        if (!cancelled) setDue(topics.filter((t) => t.id !== topicId));
      })
      .catch(() => {
        if (!cancelled) setDue([]);
      });
    return () => {
      cancelled = true;
    };
  }, [topicId]);

  const byBlank = useMemo(() => {
    const map = new Map<string, BlankResult>();
    checked?.graded.results.forEach((r) => map.set(r.blank_id, r));
    return map;
  }, [checked]);

  /** Every blank id in reading order, which is the order Enter walks. */
  const order = useMemo(
    () =>
      sheet?.items.flatMap((item) =>
        item.segments.filter((s) => s.type === "blank").map((s) => s.id),
      ) ?? [],
    [sheet],
  );

  const filled = Object.values(answers).filter((a) => a.trim() !== "").length;
  const totalBlanks = order.length;

  const check = useCallback(
    async (ratingOverride?: Rating) => {
      if (!sheet) return;
      try {
        setChecked(await api.check(sheet.sheet_id, answers, ratingOverride));
      } catch (e) {
        setError((e as Error).message);
      }
    },
    [sheet, answers],
  );

  const onSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!checked) void check();
  };

  /**
   * Enter walks to the next blank rather than checking the sheet, so a run of
   * twenty items never needs the mouse and never ends early by accident. On the
   * last blank the first Enter only arms the check, and the second one runs it.
   */
  const onBlankKeyDown = (event: React.KeyboardEvent<HTMLInputElement>, blankId: string) => {
    if (event.key !== "Enter" || checked) return;
    event.preventDefault();

    const index = order.indexOf(blankId);
    const next = order[index + 1];

    if (next) {
      setArmed(false);
      const field = inputs.current[next];
      field?.focus();
      field?.select();
      return;
    }

    if (armed) {
      void check();
    } else {
      setArmed(true);
    }
  };

  const acceptAlso = async (blankId: string) => {
    if (!sheet) return;
    try {
      await api.acceptAlso(sheet.sheet_id, blankId, answers[blankId] ?? "");
      setPatched((p) => ({ ...p, [blankId]: true }));
    } catch (e) {
      setError((e as Error).message);
    }
  };

  if (error) return <p className={styles.error}>{error}</p>;
  if (!sheet) return <p className="label">Loading sheet</p>;

  const percent = checked ? Math.round(checked.graded.score * 100) : null;

  return (
    <form onSubmit={onSubmit} className={styles.layout}>
      <div>
        <div className={styles.head}>
          <h1 className={styles.title}>{sheet.topic_title}</h1>
        </div>
        <p className={styles.meta}>
          {sheet.topic_id} / {sheet.cefr} / {totalBlanks} blanks
        </p>

        <div className={styles.items}>
          {sheet.items.map((item) => (
            <div key={item.n} className={styles.item}>
              <span className={styles.n}>{String(item.n).padStart(2, "0")}</span>
              <span className={styles.sentence}>
                {item.segments.map((segment, index) =>
                  segment.type === "text" ? (
                    <span key={index}>{segment.text}</span>
                  ) : (
                    <Blank
                      key={segment.id}
                      id={segment.id}
                      value={answers[segment.id] ?? ""}
                      result={byBlank.get(segment.id)}
                      patched={patched[segment.id] === true}
                      locked={checked !== null}
                      onChange={(value) => {
                        // Typing again means the answer is not settled, so the
                        // armed check goes away.
                        setArmed(false);
                        setAnswers((a) => ({ ...a, [segment.id]: value }));
                      }}
                      onKeyDown={(event) => onBlankKeyDown(event, segment.id)}
                      register={(element) => {
                        inputs.current[segment.id] = element;
                      }}
                      onAccept={() => acceptAlso(segment.id)}
                    />
                  ),
                )}
                {item.hint && <span className={styles.hint}>{item.hint}</span>}
              </span>
            </div>
          ))}
        </div>

        {!checked && (
          <div className={styles.actions}>
            <button type="submit" className={styles.button}>
              Check sheet
            </button>
            <span className={armed ? styles.armed : styles.keys} aria-live="polite">
              {armed
                ? "Press Enter once more to check the blanks"
                : "enter moves to the next blank"}
            </span>
          </div>
        )}
      </div>

      <aside className={styles.rail}>
        <div className={styles.progress}>
          <i style={{ width: `${totalBlanks ? (filled / totalBlanks) * 100 : 0}%` }} />
        </div>

        {checked ? (
          <>
            <p className="label">Score</p>
            <p
              className={`${styles.railValue} ${
                percent !== null && percent >= 80 ? styles.correct : styles.poor
              }`}
            >
              {percent}%
            </p>
            <p className={styles.railLine}>
              {checked.graded.correct} of {checked.graded.total} blanks. Back in{" "}
              {checked.interval_days} {checked.interval_days === 1 ? "day" : "days"}, on{" "}
              {checked.due}.
            </p>
            <p className="label">Rating</p>
            <div className={styles.ratings}>
              {RATINGS.map((rating) => (
                <button
                  key={rating}
                  type="button"
                  className={`${styles.rating} ${
                    rating === checked.rating ? styles.chosen : ""
                  }`}
                  onClick={() => check(rating)}
                >
                  {rating}
                </button>
              ))}
            </div>
          </>
        ) : (
          <>
            <p className="label">Filled</p>
            <p className={styles.railValue}>
              {filled}/{totalBlanks}
            </p>
          </>
        )}

        {due.length > 0 && (
          <>
            <p className="label">Also due</p>
            {due.slice(0, 5).map((topic) => (
              <Link key={topic.id} href={`/sheet/${topic.id}`} className={styles.nextLink}>
                {topic.title}
              </Link>
            ))}
          </>
        )}
      </aside>
    </form>
  );
}

function Blank({
  id,
  value,
  result,
  patched,
  locked,
  onChange,
  onKeyDown,
  register,
  onAccept,
}: {
  id: string;
  value: string;
  result?: BlankResult;
  patched: boolean;
  locked: boolean;
  onChange: (value: string) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  register: (element: HTMLInputElement | null) => void;
  onAccept: () => void;
}) {
  const wrong = result?.verdict === "wrong" && !patched;
  const note = result?.verdict === "correct_with_note";
  const ok = result !== undefined && !wrong;

  const className = [
    styles.blank,
    wrong ? styles.no : "",
    note ? styles.note : ok ? styles.ok : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <span className={styles.blankWrap}>
      <input
        ref={register}
        className={className}
        aria-label={`blank ${id}`}
        value={value}
        disabled={locked}
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
      {wrong && result.verdict === "wrong" && (
        <>
          <span className={styles.correction}>{result.expected}</span>
          {value.trim() !== "" && (
            <button type="button" className={styles.accept} onClick={onAccept}>
              also accept
            </button>
          )}
        </>
      )}
      {note && result.verdict === "correct_with_note" && (
        <span className={`${styles.correction} ${styles.noteText}`}>{result.expected}</span>
      )}
      {patched && <span className={`${styles.correction} ${styles.noteText}`}>accepted</span>}
    </span>
  );
}
