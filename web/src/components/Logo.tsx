import styles from "./Logo.module.css";

/**
 * The Slot mark: a dark tile holding two lines of text and one accent blank,
 * the thing lacuna is about. The tile follows the ink token, so it is dark on
 * the light page and light on the dark one, and the blank is always the accent.
 */
export function Mark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      className={[styles.mark, className].filter(Boolean).join(" ")}
      aria-hidden="true"
    >
      <rect x="1" y="1" width="30" height="30" rx="8" className={styles.tile} />
      <rect x="7" y="10" width="7" height="3" rx="1.5" className={styles.text} />
      <rect x="16" y="10" width="9" height="3" rx="1.5" className={styles.textDim} />
      <rect x="7" y="18" width="18" height="5" rx="2" className={styles.blank} />
    </svg>
  );
}

/** Mark and wordmark, as the navbar shows them. */
export default function Logo() {
  return (
    <span className={styles.logo}>
      <Mark />
      <span className={styles.word}>lacuna</span>
    </span>
  );
}
