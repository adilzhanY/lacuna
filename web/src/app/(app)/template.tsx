"use client";

import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { useRef, type ReactNode } from "react";

import { EASE, reducedMotion } from "@/lib/motion";

/**
 * A template remounts on every navigation, unlike the layout around it, so
 * this is where a page change gets its entrance: the new page settles in from
 * just below while the navbar above it stays still.
 */
export default function PageTransition({ children }: { children: ReactNode }) {
  const page = useRef<HTMLDivElement>(null);

  useGSAP(() => {
    if (reducedMotion()) return;
    gsap.from(page.current, {
      autoAlpha: 0,
      y: 10,
      duration: 0.35,
      ease: EASE,
      clearProps: "opacity,visibility,transform",
    });
  });

  return <div ref={page}>{children}</div>;
}
