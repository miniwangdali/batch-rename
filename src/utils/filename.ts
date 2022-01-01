import { basename, extname } from "@tauri-apps/api/path";

export const generateNewFilename = async (
  pattern: string,
  filename: string,
  index: number,
  options?: {
    fillUp?: boolean;
    overwriteExtension?: boolean;
    fillUpLength?: number;
  }
) => {
  if (!pattern) return filename;
  const ext = await extname(filename);
  let filenameWithoutExtension = await basename(filename, ext);
  if (filenameWithoutExtension.endsWith("."))
    filenameWithoutExtension = filenameWithoutExtension.slice(0, -1);
  const newFilename = pattern
    .replaceAll(/\*/g, filenameWithoutExtension)
    .replaceAll("#", `${index}`.padStart(options?.fillUpLength ?? 0, "0"));
  if (options?.overwriteExtension) return newFilename;
  return `${newFilename}.${ext}`;
};
