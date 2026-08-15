import type { ClientSheet } from "./types/ClientSheet";
import type { Rating } from "./types/Rating";
import type { TopicView } from "./types/TopicView";

export type { ClientSheet, Rating, TopicView };
export type { ClientItem } from "./types/ClientItem";
export type { ClientSegment } from "./types/ClientSegment";
export type { Level } from "./types/Level";

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

export type CheckResponse = {
  graded: GradedSheet;
  rating: Rating;
  interval_days: number;
  due: string;
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
