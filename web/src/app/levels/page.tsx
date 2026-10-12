"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import Shell from "@/components/Shell";
import { Centered } from "@/components/ui/Section";
import { api } from "@/lib/api";
import { levelSlug } from "@/lib/levels";

/** Levels without a level in the URL opens the level you are working on. */
export default function LevelsIndex() {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .plan()
      .then((plan) => router.replace(`/levels/${levelSlug(plan.level)}`))
      .catch((e: Error) => setError(e.message));
  }, [router]);

  return (
    <Shell>
      {error ? (
        <Centered title="Could not load" body={error} error />
      ) : (
        <p className="label">Loading</p>
      )}
    </Shell>
  );
}
