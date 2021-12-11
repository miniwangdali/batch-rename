import { basename } from "@tauri-apps/api/path";
import { atom, useRecoilState } from "recoil";

export type FileItem = {
  path: string;
  id: string;
  filename: string;
  newFileName: string;
  status?: "loading" | "success" | string;
};

export const filesState = atom<FileItem[]>({
  key: "filesState",
  default: []
});

export const useAddFiles = () => {
  const [files, setFiles] = useRecoilState(filesState);
  return async (filepaths: string[]) => {
    const newFiles = [...files];
    for (let filepath of filepaths) {
      const filename = await basename(filepath);
      if (newFiles.some((f) => f.id === filename)) continue;
      const fileItem = {
        path: filepath,
        id: filename,
        filename: filename,
        newFileName: filename,
        status: undefined
      };
      newFiles.push(fileItem);
    }
    setFiles(newFiles);
  };
};
