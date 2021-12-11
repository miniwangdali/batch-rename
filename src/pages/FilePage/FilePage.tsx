import React, { MouseEventHandler } from "react";
import Button from "@mui/material/Button";
import AddIcon from "@mui/icons-material/Add";
import AutoAwesomeIcon from "@mui/icons-material/AutoAwesome";
import { FileList } from "../../components/FileList/FileList";
import { FilePageMessages as messages } from "./messages";
import { FormattedMessage } from "react-intl";
import { useRecoilState } from "recoil";
import { FileItem, filesState, useAddFiles } from "../../atoms/files";
import { renameFile } from "@tauri-apps/api/fs";
import { open } from "@tauri-apps/api/dialog";
import "./FilePage.scss";

export const FilePage = () => {
  const [files, setFiles] = useRecoilState(filesState);
  const addFiles = useAddFiles();

  const onAddFiles: MouseEventHandler<HTMLButtonElement> = async (evt) => {
    const filepaths = await open({ multiple: true });
    if (Array.isArray(filepaths)) {
      addFiles(filepaths);
    }
  };

  const onRenameFilesClick: MouseEventHandler<HTMLButtonElement> = async () => {
    const newFiles: FileItem[] = [];
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      try {
        const newPath =
          file.path.slice(0, -file.filename.length) + file.newFileName;
        // const newPath = await join(
        //   file.path.slice(0, -file.filename.length),
        //   file.newFileName
        // );
        await renameFile(file.path, newPath);
        newFiles.push({
          ...file,
          path: newPath,
          filename: file.newFileName,
          status: "success"
        });
      } catch (e) {
        newFiles.push({ ...file, status: `${e}` });
      }
    }
    setFiles(newFiles);
  };

  return (
    <div className="FilePage">
      <div className="FilePage__header">
        <h2 className="FilePage__title">
          <FormattedMessage {...messages.fileList} />
        </h2>
        <Button
          variant="contained"
          startIcon={<AddIcon />}
          onClick={onAddFiles}
        >
          <FormattedMessage {...messages.addFiles} />
        </Button>
      </div>
      <FileList />
      <div className="FilePage__footer">
        <p className="FilePage__title">
          <FormattedMessage
            {...messages.fileCount}
            values={{ count: files.length }}
          />
        </p>
        <Button
          disabled={files.length === 0}
          variant="contained"
          color="secondary"
          startIcon={<AutoAwesomeIcon />}
          onClick={onRenameFilesClick}
        >
          <FormattedMessage {...messages.renameFile} />
        </Button>
      </div>
    </div>
  );
};
