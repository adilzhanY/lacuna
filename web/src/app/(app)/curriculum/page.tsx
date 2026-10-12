import { redirect } from "next/navigation";

/** The old name of the levels page, kept so existing links still land. */
export default function Curriculum() {
  redirect("/levels");
}
