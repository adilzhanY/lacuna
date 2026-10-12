"use client";

import { BarChart3, Layers, Sun } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, type ReactNode } from "react";

import Logo from "@/components/Logo";
import { levelSlug } from "@/lib/levels";
import { pop, slideTo } from "@/lib/motion";
import { refresh, useData } from "@/lib/store";
import styles from "./Shell.module.css";

const LINKS: { href: string; label: string; icon: ReactNode }[] = [
  { href: "/", label: "Today", icon: <Sun size={15} /> },
  { href: "/levels", label: "Levels", icon: <Layers size={15} /> },
  { href: "/stats", label: "Stats", icon: <BarChart3 size={15} /> },
];

/**
 * One glass pill floating over the page: the mark, the pages, and your level
 * with what today still holds. The page itself sits on the bare paper, no card.
 */
export default function Shell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const { data: plan } = useData("plan");
  const nav = useRef<HTMLElement>(null);
  const indicator = useRef<HTMLSpanElement>(null);
  const badge = useRef<HTMLAnchorElement>(null);
  const placed = useRef(false);

  // The count in the level badge can change on any page, so look again on each.
  useEffect(() => {
    refresh("plan").catch(() => {});
  }, [pathname]);

  const left = plan ? plan.reviews.length + plan.new_topics.length : 0;

  // The active pill slides from the old link to the new one. The first time it
  // is simply put in place, so a fresh load does not sweep in from the left.
  useEffect(() => {
    const active = nav.current?.querySelector<HTMLElement>('[aria-current="page"]') ?? null;
    if (!placed.current && active && indicator.current) {
      placed.current = true;
      slideTo(indicator.current, active, { x: active.offsetLeft, width: active.offsetWidth });
      return;
    }
    slideTo(indicator.current, active);
  }, [pathname]);

  // The badge swells when your level or today's count changes, not on first paint.
  const seen = useRef<string | null>(null);
  useEffect(() => {
    if (!plan) return;
    const now = `${plan.level}:${left}`;
    if (seen.current !== null && seen.current !== now) pop(badge.current);
    seen.current = now;
  }, [plan, left]);

  return (
    <div className={styles.frame}>
      <header className={styles.header}>
        <div className={styles.pill}>
          <Link href="/" className={styles.brand} aria-label="lacuna, today">
            <Logo />
          </Link>
          <nav aria-label="Pages" className={styles.nav} ref={nav}>
            <span className={styles.indicator} ref={indicator} aria-hidden="true" />
            {LINKS.map((link) => {
              // Straight to your level, so Levels is one hop, not a redirect.
              const href =
                link.href === "/levels" && plan
                  ? `/levels/${levelSlug(plan.level)}`
                  : link.href;
              const active =
                link.href === "/"
                  ? pathname === "/"
                  : pathname.startsWith(link.href);
              return (
                <Link
                  key={link.href}
                  href={href}
                  aria-current={active ? "page" : undefined}
                  className={`${styles.link} ${active ? styles.active : ""}`}
                >
                  {link.icon}
                  <span>{link.label}</span>
                </Link>
              );
            })}
          </nav>
          {plan && (
            <Link
              ref={badge}
              href={`/levels/${levelSlug(plan.level)}`}
              className={styles.level}
              title={`Your level. ${left} to study today.`}
            >
              {plan.level}
              {left > 0 && <span className={styles.count}>{left}</span>}
            </Link>
          )}
        </div>
      </header>
      <main className={styles.main}>
        <div className={styles.panel}>{children}</div>
      </main>
    </div>
  );
}
