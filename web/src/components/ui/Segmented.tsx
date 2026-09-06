import type { KeyboardEvent, ReactNode } from "react";

import styles from "./ui.module.css";

export interface SegmentOption<V extends string> {
  value: V;
  label: string;
  icon?: ReactNode;
}

interface Props<V extends string> {
  label: string;
  value: V;
  options: readonly SegmentOption<V>[];
  onChange: (value: V) => void;
}

/** Mutually exclusive options as a radiogroup. Arrow keys move, roving tabindex. */
export function Segmented<V extends string>({
  label,
  value,
  options,
  onChange,
}: Props<V>) {
  const index = options.findIndex((o) => o.value === value);
  const onKeyDown = (event: KeyboardEvent) => {
    const forward = event.key === "ArrowRight" || event.key === "ArrowDown";
    const back = event.key === "ArrowLeft" || event.key === "ArrowUp";
    if (!forward && !back) return;
    event.preventDefault();
    const next =
      options[(index + (forward ? 1 : -1) + options.length) % options.length];
    if (next) {
      onChange(next.value);
      const sibling =
        event.currentTarget.parentElement?.children[options.indexOf(next)];
      (sibling as HTMLElement | undefined)?.focus();
    }
  };
  const dense = options.length > 3;
  return (
    <div role="radiogroup" aria-label={label} className={styles.segmented}>
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={onKeyDown}
            className={[
              styles.segment,
              selected && styles.selected,
              dense && styles.dense,
            ]
              .filter(Boolean)
              .join(" ")}
          >
            {option.icon}
            <span className={styles.segmentText}>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
