"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { Centered } from "@/components/ui/Section";
import { levelSlug } from "@/lib/levels";
import { useData } from "@/lib/store";

/** Levels without a level in the URL opens the level you are working on. */
export default function LevelsIndex() {
  const router = useRouter();
  const { data: plan, error } = useData("plan");

  useEffect(() => {
    if (plan) router.replace(`/levels/${levelSlug(plan.level)}`);
  }, [plan, router]);

  return error ? (
    <Centered title="Could not load" body={error} error />
  ) : (
    <p className="label">Loading</p>
  );
}
