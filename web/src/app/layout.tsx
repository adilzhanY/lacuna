import type { Metadata } from "next";
import { Geist_Mono, Space_Grotesk } from "next/font/google";
import localFont from "next/font/local";
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
      <body>{children}</body>
    </html>
  );
}
