import Link from "next/link";
import type { ButtonHTMLAttributes, ReactNode } from "react";

import styles from "./ui.module.css";

export type ButtonVariant = "default" | "primary" | "quiet" | "danger";

interface Common {
  variant?: ButtonVariant;
  icon?: ReactNode;
  active?: boolean;
  large?: boolean;
  className?: string;
  children?: ReactNode;
}

type ButtonProps = Common &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className">;
type LinkProps = Common & { href: string };

const classes = ({ variant = "default", active, large, className }: Common) =>
  [
    styles.button,
    variant !== "default" && styles[variant],
    active && styles.active,
    large && styles.large,
    className,
  ]
    .filter(Boolean)
    .join(" ");

/** Text button as a pill. Keyboard focus is drawn by the global focus ring. */
export function Button({
  variant,
  icon,
  active,
  large,
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      aria-pressed={active || undefined}
      className={classes({ variant, active, large, className })}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}

/** The same pill, but a link. */
export function LinkButton({ href, icon, children, ...rest }: LinkProps) {
  return (
    <Link href={href} className={classes(rest)}>
      {icon}
      {children}
    </Link>
  );
}
