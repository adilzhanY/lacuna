import type { Metadata } from "next";
import { Archivo, Manrope } from "next/font/google";
import "./globals.css";

// Headings and big numbers.
const display = Archivo({
  variable: "--font-display",
  subsets: ["latin"],
});

// Labels, body and every number.
const sans = Manrope({
  variable: "--font-sans",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "lacuna",
  description: "Grammar practice by filling in the gaps",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${display.variable} ${sans.variable}`}>
      <body>{children}</body>
    </html>
  );
}
