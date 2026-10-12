"use client";

import { BarChart3, CalendarCheck, ListTree } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";

import { api } from "@/lib/api";
import { Chip } from "@/components/ui/Section";
import styles from "./Shell.module.css";

const LINKS: { href: string; label: string; icon: ReactNode }[] = [
  { href: "/", label: "Today", icon: <CalendarCheck size={15} /> },
  { href: "/levels", label: "Levels", icon: <ListTree size={15} /> },
  { href: "/stats", label: "Statistics", icon: <BarChart3 size={15} /> },
];

/**
 * Floating pill bar over the paper, the page content in a rounded card below.
 * The nav is a segmented control of links: the active page is the ink pill.
 */
export default function Shell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const [due, setDue] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .today()
      .then((topics) => {
        if (!cancelled) setDue(topics.length);
      })
      .catch(() => {
        if (!cancelled) setDue(null);
      });
    return () => {
      cancelled = true;
    };
  }, [pathname]);

  return (
    <div className={styles.frame}>
      <header className={styles.bar}>
        <Link href="/" className={styles.brand}>
          lacuna
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
        <span className={styles.spacer} />
        {due !== null && due > 0 && (
          <Chip>
            {due} to study today
          </Chip>
        )}
      </header>
      <main className={styles.main}>
        <div className={styles.panel}>{children}</div>
      </main>
    </div>
  );
}
