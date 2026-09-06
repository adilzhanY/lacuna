import type { ReactNode } from "react";

import styles from "./ui.module.css";

interface SectionProps {
  title: string;
  /** Small text on the right of the title, or a control. */
  action?: ReactNode;
  children: ReactNode;
  /** Draws the section without the top rule, for the first one in a panel. */
  first?: boolean;
  className?: string;
}

/** A titled block. The title is an h2 so a page reads as an outline. */
export function Section({
  title,
  action,
  children,
  first = false,
  className,
}: SectionProps) {
  return (
    <section
      className={[styles.section, first && styles.first, className]
        .filter(Boolean)
        .join(" ")}
    >
      <header className={styles.sectionHead}>
        <h2 className={styles.sectionTitle}>{title}</h2>
        {typeof action === "string" ? (
          <span className={styles.sectionNote}>{action}</span>
        ) : (
          action
        )}
      </header>
      {children}
    </section>
  );
}

/** Label on the left, value on the right. */
export function ReadOnly({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "ok" | "mark";
}) {
  return (
    <div className={styles.readOnly}>
      <span className={styles.readOnlyLabel}>{label}</span>
      <span
        className={[styles.readOnlyValue, tone && styles[tone]]
          .filter(Boolean)
          .join(" ")}
      >
        {value}
      </span>
    </div>
  );
}

/** Small pill with a count or a state. */
export function Chip({
  children,
  tone = "select",
}: {
  children: ReactNode;
  tone?: "select" | "neutral" | "ok" | "mark";
}) {
  return (
    <span
      className={[styles.chip, tone !== "select" && styles[tone]]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </span>
  );
}

/** Title, one line of body and an optional action, centred. Every state has a face. */
export function Centered({
  title,
  body,
  error = false,
  children,
}: {
  title: string;
  body?: string;
  error?: boolean;
  children?: ReactNode;
}) {
  return (
    <div className={styles.centered}>
      <h1 className={styles.centeredTitle}>{title}</h1>
      {body && (
        <p
          className={[styles.centeredBody, error && styles.mark]
            .filter(Boolean)
            .join(" ")}
        >
          {body}
        </p>
      )}
      {children}
    </div>
  );
}
