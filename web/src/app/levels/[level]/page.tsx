import { notFound } from "next/navigation";

import Levels from "@/components/Levels";
import Shell from "@/components/Shell";
import { levelFromSlug } from "@/lib/levels";

export default async function LevelPage({ params }: PageProps<"/levels/[level]">) {
  const { level: slug } = await params;
  const level = levelFromSlug(slug);
  if (!level) notFound();
  return (
    <Shell>
      <Levels level={level} />
    </Shell>
  );
}
