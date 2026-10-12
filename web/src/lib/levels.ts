import type { Level } from "./types/Level";

/** Every level in roadmap order, the same order as `Level::ALL` on the server. */
export const LEVELS: Level[] = [
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

/** The level as it appears in a URL: "A1+" becomes a1plus. */
export function levelSlug(level: Level): string {
  return level.toLowerCase().replace("+", "plus");
}

export function levelFromSlug(slug: string): Level | undefined {
  return LEVELS.find((level) => levelSlug(level) === slug);
}

/** Tab label. Muttersprache is too wide to sit in a row of twelve. */
export function levelShort(level: Level): string {
  return level === "Muttersprache" ? "M" : level;
}
