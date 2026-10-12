"use client";

import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import type { RefObject } from "react";

gsap.registerPlugin(useGSAP);

/*
 * All motion in lacuna goes through here, so it moves alike everywhere: one
 * ease, short durations, and nothing at all when the system asks for reduced
 * motion. Elements opt in with data attributes instead of each page writing
 * its own timeline:
 *
 *   data-reveal   fades and rises in, in reading order
 *   data-grow     a progress fill that grows from the left
 *   data-rise     a column that rises from the baseline
 *   data-count    a number that counts up to the value it shows
 *   data-wave     a container whose children sweep in, like the heatmap cells
 */

export const EASE = "power3.out";

export function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/**
 * Bring a page in once it has something to show. Runs before the browser
 * paints, so the parts never flash at full opacity first.
 */
export function useReveal(
  scope: RefObject<HTMLElement | null>,
  ready: boolean,
  key: unknown = null,
) {
  useGSAP(
    () => {
      const root = scope.current;
      if (!ready || !root || reducedMotion()) return;

      const parts = root.querySelectorAll("[data-reveal]");
      if (parts.length) {
        gsap.from(parts, {
          opacity: 0,
          y: 14,
          duration: 0.5,
          ease: EASE,
          // A long list should not take seconds to arrive: cap the whole run.
          stagger: Math.min(0.05, 0.6 / parts.length),
          clearProps: "opacity,transform",
        });
      }

      gsap.from(root.querySelectorAll("[data-grow]"), {
        scaleX: 0,
        transformOrigin: "left center",
        duration: 0.9,
        ease: "power2.out",
        delay: 0.15,
        stagger: 0.05,
        clearProps: "transform",
      });

      gsap.from(root.querySelectorAll("[data-rise]"), {
        scaleY: 0,
        transformOrigin: "center bottom",
        duration: 0.7,
        ease: "power2.out",
        delay: 0.15,
        stagger: { amount: 0.4 },
        clearProps: "transform",
      });

      root.querySelectorAll<HTMLElement>("[data-count]").forEach((element) => {
        const target = Number(element.dataset.count);
        if (!Number.isFinite(target) || target === 0) return;
        const counter = { value: 0 };
        element.textContent = "0";
        gsap.to(counter, {
          value: target,
          duration: 0.9,
          ease: "power2.out",
          delay: 0.1,
          onUpdate: () => {
            element.textContent = String(Math.round(counter.value));
          },
        });
      });

      root.querySelectorAll("[data-wave]").forEach((wave) => {
        gsap.from(wave.querySelectorAll(":scope span"), {
          opacity: 0,
          scale: 0.4,
          duration: 0.35,
          ease: EASE,
          delay: 0.2,
          stagger: { amount: 0.8 },
          clearProps: "opacity,transform",
        });
      });
    },
    { scope, dependencies: [ready, key] },
  );
}

/** A quick swell, for something that just changed: a right answer, a new badge. */
export function pop(element: Element | null) {
  if (!element || reducedMotion()) return;
  gsap.fromTo(
    element,
    { scale: 1 },
    { scale: 1.08, duration: 0.14, ease: "power2.out", yoyo: true, repeat: 1 },
  );
}

/** A short head shake, for a wrong answer. */
export function shake(element: Element | null) {
  if (!element || reducedMotion()) return;
  gsap.fromTo(
    element,
    { x: 0 },
    {
      keyframes: { x: [0, -10, 9, -6, 4, -2, 0] },
      duration: 0.45,
      ease: "power1.out",
    },
  );
}

/**
 * Move an indicator (the navbar pill, the tab underline) onto a target, from
 * wherever it was. `from` lets a freshly mounted indicator start where the
 * previous page left it, so it slides instead of jumping.
 */
export function slideTo(
  indicator: HTMLElement | null,
  target: HTMLElement | null,
  from?: { x: number; width: number } | null,
) {
  if (!indicator) return;
  if (!target) {
    gsap.to(indicator, { opacity: 0, duration: 0.2 });
    return;
  }
  const place = { x: target.offsetLeft, width: target.offsetWidth };
  if (reducedMotion()) {
    gsap.set(indicator, { ...place, autoAlpha: 1 });
    return;
  }
  if (from) gsap.set(indicator, { ...from, autoAlpha: 1 });
  gsap.to(indicator, { ...place, autoAlpha: 1, duration: 0.45, ease: EASE });
}
