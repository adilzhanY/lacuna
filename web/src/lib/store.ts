import { useEffect, useState, useSyncExternalStore } from "react";

import { api, type Stats, type TodayPlan, type TopicView } from "./api";

/*
 * A small client cache for the three things most pages read. A page gets the
 * last value at once, so navigating back to it never shows "Loading" again,
 * and the value is refreshed in the background every time a page asks for it.
 * Two pages asking at the same moment share one request.
 */

type Data = { topics: TopicView[]; plan: TodayPlan; stats: Stats };
type Key = keyof Data;

const fetchers: { [K in Key]: () => Promise<Data[K]> } = {
  topics: api.topics,
  plan: api.plan,
  stats: api.stats,
};

const cache: Partial<Data> = {};
const inflight: Partial<Record<Key, Promise<unknown>>> = {};
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Fetch a value again and tell every reader. */
export function refresh<K extends Key>(key: K): Promise<Data[K]> {
  const running = inflight[key] as Promise<Data[K]> | undefined;
  if (running) return running;
  const request = fetchers[key]()
    .then((value) => {
      cache[key] = value;
      emit();
      return value;
    })
    .finally(() => {
      delete inflight[key];
    });
  inflight[key] = request;
  return request;
}

/** Put a value the server just returned, so readers need not wait for a refetch. */
export function setCached<K extends Key>(key: K, value: Data[K]) {
  cache[key] = value;
  emit();
}

/** After an answer is recorded everything a page shows may have moved. */
export function refreshAll() {
  void Promise.allSettled([refresh("topics"), refresh("plan"), refresh("stats")]);
}

/** The cached value, refreshed once when the component mounts. */
export function useData<K extends Key>(key: K): {
  data: Data[K] | undefined;
  error: string | null;
} {
  const data = useSyncExternalStore(
    subscribe,
    () => cache[key] as Data[K] | undefined,
    () => undefined,
  );
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    refresh(key).catch((e: Error) => {
      // A stale value is better than an error screen, so only report when
      // there is nothing to show.
      if (!cancelled && cache[key] === undefined) setError(e.message);
    });
    return () => {
      cancelled = true;
    };
  }, [key]);

  return { data, error };
}
