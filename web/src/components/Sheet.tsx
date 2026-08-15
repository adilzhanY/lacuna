"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  api,
  type BlankResult,
  type CheckResponse,
  type ClientSheet,
  type Rating,
} from "@/lib/api";
import styles from "./Sheet.module.css";

const RATINGS: Rating[] = ["again", "hard", "good", "easy"];

export default function Sheet({ topicId }: { topicId: string }) {
  const [sheet, setSheet] = useState<ClientSheet | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [checked, setChecked] = useState<CheckResponse | null>(null);
  const [patched, setPatched] = useState<Record<string, true>>({});
  const formRef = useRef<HTMLFormElement>(null);

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
    return () => {
      cancelled = true;
    };
  }, [topicId]);

  const byBlank = useMemo(() => {
    const map = new Map<string, BlankResult>();
    checked?.graded.results.forEach((r) => map.set(r.blank_id, r));
    return map;
  }, [checked]);

  const filled = Object.values(answers).filter((a) => a.trim() !== "").length;
  const totalBlanks =
    sheet?.items.reduce(
      (sum, item) => sum + item.segments.filter((s) => s.type === "blank").length,
      0,
    ) ?? 0;

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

  // Enter checks the sheet from anywhere in it, so a run of twenty items never
  // needs the mouse.
  const onSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!checked) void check();
  };

  const acceptAlso = async (blankId: string) => {
    if (!sheet) return;
    const given = answers[blankId] ?? "";
    try {
      await api.acceptAlso(sheet.sheet_id, blankId, given);
      setPatched((p) => ({ ...p, [blankId]: true }));
    } catch (e) {
      setError((e as Error).message);
    }
  };

  if (error) return <p className={styles.error}>{error}</p>;
  if (!sheet) return <p className="label">Loading sheet</p>;

  return (
    <form ref={formRef} onSubmit={onSubmit}>
      <div className={styles.head}>
        <h2 className={styles.title}>{sheet.topic_title}</h2>
        <span className={styles.counter}>
          {checked ? `${checked.graded.correct} / ${checked.graded.total}` : `${filled} / ${totalBlanks}`}
        </span>
      </div>
      <p className={`label ${styles.breadcrumb}`}>
        {sheet.topic_category} &rsaquo; {sheet.cefr}
      </p>

      {sheet.items.map((item) => (
        <div key={item.n} className={styles.item}>
          <span className={styles.n}>{item.n}</span>
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
                  onChange={(value) => setAnswers((a) => ({ ...a, [segment.id]: value }))}
                  onAccept={() => acceptAlso(segment.id)}
                />
              ),
            )}
            {item.hint && <span className={styles.hint}>({item.hint})</span>}
          </span>
        </div>
      ))}

      {!checked && (
        <div className={styles.actions}>
          <button type="submit" className={styles.button}>
            Check sheet
          </button>
          <span className={styles.keys}>Tab for the next blank, Enter to check</span>
        </div>
      )}

      {checked && <Result checked={checked} onRate={(rating) => check(rating)} />}
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
  onAccept,
}: {
  id: string;
  value: string;
  result?: BlankResult;
  patched: boolean;
  locked: boolean;
  onChange: (value: string) => void;
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
        className={className}
        aria-label={`blank ${id}`}
        value={value}
        disabled={locked}
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
      />
      {wrong && result.verdict === "wrong" && (
        <>
          <span className={styles.correction}>
            <em>{result.expected}</em>
          </span>
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

function Result({
  checked,
  onRate,
}: {
  checked: CheckResponse;
  onRate: (rating: Rating) => void;
}) {
  const percent = Math.round(checked.graded.score * 100);
  return (
    <div className={styles.result}>
      <p className="label">Result</p>
      <p className={styles.score}>{percent}%</p>
      <p className={styles.resultLine}>
        {checked.graded.correct} of {checked.graded.total} blanks. Rated{" "}
        <strong>{checked.rating}</strong>, back in {checked.interval_days}{" "}
        {checked.interval_days === 1 ? "day" : "days"} on {checked.due}.
      </p>
      <div className={styles.ratings}>
        {RATINGS.map((rating) => (
          <button
            key={rating}
            type="button"
            className={`${styles.rating} ${rating === checked.rating ? styles.chosen : ""}`}
            onClick={() => onRate(rating)}
          >
            {rating}
          </button>
        ))}
      </div>
      <p className={styles.resultLine} style={{ marginTop: 10 }}>
        Override the rating if the score does not match what you actually knew.
      </p>
    </div>
  );
}
