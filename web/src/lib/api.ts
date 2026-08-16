import type { ClientSheet } from "./types/ClientSheet";
import type { FinishResponse } from "./types/FinishResponse";
import type { ReviewQueue } from "./types/ReviewQueue";
import type { Stats } from "./types/Stats";
import type { Rating } from "./types/Rating";
import type { TopicView } from "./types/TopicView";

export type { ClientSheet, Rating, TopicView };
export type { ClientItem } from "./types/ClientItem";
export type { ClientSegment } from "./types/ClientSegment";
export type { DayPoint } from "./types/DayPoint";
export type { FinishResponse } from "./types/FinishResponse";
export type { ReviewQueue } from "./types/ReviewQueue";
export type { Level } from "./types/Level";
export type { LevelProgress } from "./types/LevelProgress";
export type { Stats } from "./types/Stats";
export type { TopicScore } from "./types/TopicScore";
export type { Weakness } from "./types/Weakness";

/*
 * The graded payload is hand written rather than generated, because it is an
 * internally tagged Rust enum and ts-rs cannot express the flattened shape.
 * Keep it in step with `Verdict` and `BlankResult` in server/src/grade.rs.
 */
export type Verdict =
  | { verdict: "correct" }
  | { verdict: "correct_with_note"; note: string; expected: string }
  | { verdict: "wrong"; expected: string; tags: string[] };

export type BlankResult = { blank_id: string; given: string } & Verdict;

export type GradedSheet = {
  results: BlankResult[];
  correct: number;
  total: number;
  score: number;
  rating: Rating;
};

/**
 * One item as review mode grades it. Hand written for the same reason as
 * GradedSheet: `results` is a flattened Rust enum that ts-rs cannot express.
 */
export type ItemVerdict = {
  correct: boolean;
  grade: Rating;
  results: BlankResult[];
};

export type CheckResponse = {
  graded: GradedSheet;
  rating: Rating;
  interval_days: number;
  due: string;
};

/** One answered item on its way back to the server. */
export type ItemAttempt = {
  n: number;
  elapsed_ms: number;
  answers: Record<string, string>;
};

export type AcceptResponse = {
  blank_id: string;
  accept: string[];
  verdict: Verdict;
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...init?.headers },
    cache: "no-store",
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `request to ${path} failed`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  topics: () => request<TopicView[]>("/api/topics"),
  today: () => request<TopicView[]>("/api/today"),
  sheet: (topicId: string) => request<ClientSheet>(`/api/sheet/${topicId}`),
  stats: () => request<Stats>("/api/stats"),

  reviewNext: () => request<ReviewQueue>("/api/review/next"),

  reviewItem: (sheetId: number, n: number, elapsedMs: number, answers: Record<string, string>) =>
    request<ItemVerdict>(`/api/review/${sheetId}/item`, {
      method: "POST",
      body: JSON.stringify({ n, elapsed_ms: Math.round(elapsedMs), answers }),
    }),

  reviewFinish: (sheetId: number, items: ItemAttempt[]) =>
    request<FinishResponse>(`/api/review/${sheetId}/finish`, {
      method: "POST",
      body: JSON.stringify({ items }),
    }),

  check: (sheetId: number, answers: Record<string, string>, ratingOverride?: Rating) =>
    request<CheckResponse>(`/api/sheet/${sheetId}/check`, {
      method: "POST",
      body: JSON.stringify({ answers, rating_override: ratingOverride ?? null }),
    }),

  acceptAlso: (sheetId: number, blankId: string, answer: string) =>
    request<AcceptResponse>(`/api/sheet/${sheetId}/accept`, {
      method: "POST",
      body: JSON.stringify({ blank_id: blankId, answer }),
    }),
};
