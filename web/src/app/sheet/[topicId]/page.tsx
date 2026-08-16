import Shell from "@/components/Shell";
import Sheet from "@/components/Sheet";

export default async function SheetPage({ params }: PageProps<"/sheet/[topicId]">) {
  const { topicId } = await params;
  return (
    <Shell>
      {/* The key remounts the sheet when the topic changes, so no answers leak
          from one sheet into the next. */}
      <Sheet key={topicId} topicId={topicId} />
    </Shell>
  );
}
