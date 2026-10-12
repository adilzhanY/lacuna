import type { Metadata } from "next";
import { Geist_Mono, Space_Grotesk } from "next/font/google";
import localFont from "next/font/local";
import Script from "next/script";
import "./globals.css";

// Speaks: titles, big numbers, the main button.
const display = Space_Grotesk({
  variable: "--font-display",
  subsets: ["latin"],
  weight: ["500", "600", "700"],
});

// Reads: body text and every German sentence. A rounded cut of Inter, OFL
// licensed, subset to Latin, which covers the umlauts and ß.
const sans = localFont({
  variable: "--font-sans",
  src: [
    { path: "./fonts/OpenRunde-Regular-latin.woff2", weight: "400" },
    { path: "./fonts/OpenRunde-Medium-latin.woff2", weight: "500" },
    { path: "./fonts/OpenRunde-Semibold-latin.woff2", weight: "600" },
  ],
});

// Labels: section labels, values, keys and links.
const mono = Geist_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
});

/*
 * Some browser extensions stamp their own attributes (`bis_skin_checked`,
 * `__processed_...`) onto every element before React hydrates, and React then
 * reports the page as not matching the server. This strips those attributes as
 * they appear, until the page has loaded, then stops watching so it costs
 * nothing while the app runs and animates.
 */
const STRIP_EXTENSION_ATTRIBUTES = `(() => {
  const foreign = (name) => name.startsWith("bis_") || name.startsWith("__processed_");
  const clean = (el) => {
    if (!el.attributes) return;
    for (const { name } of [...el.attributes]) if (foreign(name)) el.removeAttribute(name);
  };
  const sweep = (root) => { clean(root); root.querySelectorAll?.("*").forEach(clean); };
  const watcher = new MutationObserver((changes) => {
    for (const change of changes) {
      if (change.type === "attributes") { if (foreign(change.attributeName)) clean(change.target); }
      else change.addedNodes.forEach((node) => node.nodeType === 1 && sweep(node));
    }
  });
  watcher.observe(document.documentElement, { attributes: true, childList: true, subtree: true });
  sweep(document.documentElement);
  addEventListener("load", () => setTimeout(() => watcher.disconnect(), 2000), { once: true });
})();`;

export const metadata: Metadata = {
  title: "lacuna",
  description: "Grammar practice by filling in the gaps",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={`${display.variable} ${sans.variable} ${mono.variable}`}
    >
      <body>
        <Script id="strip-extension-attributes" strategy="beforeInteractive">
          {STRIP_EXTENSION_ATTRIBUTES}
        </Script>
        {children}
      </body>
    </html>
  );
}
