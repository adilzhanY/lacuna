"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";

import { api } from "@/lib/api";
import styles from "./Shell.module.css";

const LINKS = [
  { href: "/", label: "Curriculum" },
  { href: "/stats", label: "Statistics" },
];

export default function Shell({ children }: { children: React.ReactNode }) {
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
    <>
      <header className={styles.bar}>
        <Link href="/" className={styles.brand}>
          lacuna
        </Link>
        <nav className={styles.nav}>
          {LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={`${styles.link} ${pathname === link.href ? styles.active : ""}`}
            >
              {link.label}
            </Link>
          ))}
        </nav>
        <span className={styles.spacer} />
        {due !== null && due > 0 && <span className={styles.due}>{due} due</span>}
      </header>
      <main className={styles.main}>{children}</main>
    </>
  );
}
