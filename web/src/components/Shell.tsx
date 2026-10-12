"use client";

import { BarChart3, Layers, Sun } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";

import Logo from "@/components/Logo";
import { api, type TodayPlan } from "@/lib/api";
import { levelSlug } from "@/lib/levels";
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
  const [plan, setPlan] = useState<TodayPlan | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .plan()
      .then((loaded) => {
        if (!cancelled) setPlan(loaded);
      })
      .catch(() => {
        if (!cancelled) setPlan(null);
      });
    return () => {
      cancelled = true;
    };
  }, [pathname]);

  const left = plan ? plan.reviews.length + plan.new_topics.length : 0;

  return (
    <div className={styles.frame}>
      <header className={styles.header}>
        <div className={styles.pill}>
          <Link href="/" className={styles.brand} aria-label="lacuna, today">
            <Logo />
          </Link>
          <nav aria-label="Pages" className={styles.nav}>
            {LINKS.map((link) => {
              const active =
                link.href === "/"
                  ? pathname === "/"
                  : pathname.startsWith(link.href);
              return (
                <Link
                  key={link.href}
                  href={link.href}
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
