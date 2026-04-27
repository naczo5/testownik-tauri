import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";

export interface BaseIndex {
  name: string;
  slug: string;
  displayName?: string;
  questionCount: number;
}

export interface Answer {
  key: string;
  text?: string;
  image?: string;
}

export interface Question {
  id?: string;
  question: string;
  images?: string[];
  answers: Answer[];
  correct: string[];
}

export interface BaseData {
  name: string;
  slug: string;
  displayName?: string;
  questionCount: number;
  questions: Question[];
}

export async function getAppDataDir(): Promise<string> {
  return await invoke<string>("get_app_data_dir");
}

export async function getBasesIndex(): Promise<BaseIndex[]> {
  const jsonStr = await invoke<string>("get_bases_index");
  const data = JSON.parse(jsonStr);
  return data.bases;
}

export async function getBase(slug: string): Promise<BaseData> {
  const jsonStr = await invoke<string>("get_base", { slug });
  const data = JSON.parse(jsonStr);
  data.slug = slug;
  return data;
}

export async function saveBase(slug: string, data: BaseData): Promise<void> {
  await invoke("save_base", { slug, content: JSON.stringify(data, null, 2) });
}

export async function importOldBase(path: string): Promise<void> {
  await invoke("import_old_base", { path });
}

export async function importNewBase(path: string): Promise<void> {
  await invoke("import_new_base", { path });
}

export async function exportToAnki(slug: string, exportPath: string): Promise<void> {
  await invoke("export_to_anki", { slug, exportPath });
}

export async function getImageUrl(baseSlug: string, imageName: string): Promise<string> {
  const appDataDir = await getAppDataDir();
  const path = `${appDataDir}/bazy/${baseSlug}/baza/${imageName}`;
  return convertFileSrc(path);
}
