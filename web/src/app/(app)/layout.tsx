import type { ReactNode } from "react";

import Shell from "@/components/Shell";

/**
 * Every page but review mode shares this shell, so the navbar mounts once and
 * stays put across navigation instead of being rebuilt by each page.
 */
export default function AppLayout({ children }: { children: ReactNode }) {
  return <Shell>{children}</Shell>;
}
