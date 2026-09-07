import type { TagDbStatus, TagSuggestion } from "../types";
import { invokeOrThrow, isTauri, loadPreferences } from "./transport";

export async function suggestTags(query: string): Promise<TagSuggestion[]> {
  if (isTauri()) return invokeOrThrow("suggest_tags", { query });
  return (await loadPreferences()).suggestTags(query);
}

export async function tagDbStatus(): Promise<TagDbStatus> {
  if (isTauri()) return invokeOrThrow("tag_db_status");
  return (await loadPreferences()).tagDbStatus();
}

export async function updateTagDb(): Promise<void> {
  if (isTauri()) return invokeOrThrow("update_tag_db");
}

export async function translateTags(tags: Array<{ namespace: string; name: string }>): Promise<Array<string | null>> {
  if (isTauri()) return invokeOrThrow("translate_tags", { tags });
  return (await loadPreferences()).translateTags(tags);
}
