import React, { useMemo, useState } from "react";
import cx from "classnames";
import {
  Box,
  CircularProgress,
  createTheme,
  IconButton,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListSubheader,
  Paper,
  ThemeProvider,
  Tooltip,
  Typography
} from "@mui/material";
import DeleteIcon from "@mui/icons-material/Delete";
import CheckIcon from "@mui/icons-material/Check";
import ErrorIcon from "@mui/icons-material/Error";
import { useMount, useUnmount } from "react-use";
import { getCurrent } from "@tauri-apps/api/window";
import { UnlistenFn } from "@tauri-apps/api/event";
import { FileListMessages as messages } from "./messages";
import { FormattedMessage, useIntl } from "react-intl";
import { useRecoilState } from "recoil";
import { filesState } from "../../atoms/files";
import "./FileList.scss";

const theme = createTheme({ palette: { mode: "light" } });

let unlistenList: UnlistenFn[] = [];

export const FileList = () => {
  const intl = useIntl();
  const [files, setFiles] = useRecoilState(filesState);
  const [draggingOver, setDraggingOver] = useState(false);

  const headers = useMemo(
    () => [
      { id: "filename", label: intl.formatMessage(messages.filename) },
      { id: "newFilename", label: intl.formatMessage(messages.newFilename) }
    ],
    [intl]
  );

  useMount(() => {
    const currentWindow = getCurrent();
    currentWindow
      .listen("tauri://file-drop-hover", () => {
        setDraggingOver(true);
      })
      .then((unlistenFunc) => {
        unlistenList.push(unlistenFunc);
      });
    currentWindow
      .listen("tauri://file-drop", () => {
        setDraggingOver(false);
      })
      .then((unlistenFunc) => {
        unlistenList.push(unlistenFunc);
      });
    currentWindow
      .listen("tauri://file-drop-cancelled", () => {
        setDraggingOver(false);
      })
      .then((unlistenFunc) => {
        unlistenList.push(unlistenFunc);
      });
  });

  useUnmount(() => {
    unlistenList.forEach((unlisten) => unlisten());
  });

  const removeFile = (filePath: string) => {
    setFiles(files.filter((file) => file.path !== filePath));
  };

  return (
    <div
      className={cx("FileList", { "FileList--dragging-over": draggingOver })}
    >
      <Box className="FileList__drop-message" borderColor="primary.main">
        <Typography variant="h5">
          <FormattedMessage {...messages.dropToAddFiles} />
        </Typography>
      </Box>
      <ThemeProvider theme={theme}>
        <Paper className="FileList__list-container">
          <List className="FileList__list" disablePadding>
            <ListSubheader
              className="FileList__list-header"
              component={ListItem}
              disableGutters
              disablePadding
            >
              <ListItemIcon></ListItemIcon>
              {headers.map((h, i) => (
                <ListItemText className="FileList__list-item-text" key={h.id}>
                  {h.label}
                </ListItemText>
              ))}
              <ListItemIcon></ListItemIcon>
            </ListSubheader>
            {files.map((file) => (
              <ListItem
                key={file.path}
                className="FileList__list-item"
                disablePadding
              >
                <ListItemIcon sx={{ justifyContent: "center" }}>
                  {file.status === "loading" && (
                    <CircularProgress
                      size={20}
                      variant="indeterminate"
                      disableShrink
                    />
                  )}
                  {file.status === "success" && <CheckIcon color="success" />}
                  {file.status &&
                    file.status !== "loading" &&
                    file.status !== "success" && (
                      <Tooltip title={file.status} placement="right">
                        <IconButton size="small">
                          <ErrorIcon color="error" />
                        </IconButton>
                      </Tooltip>
                    )}
                </ListItemIcon>
                <ListItemText className="FileList__list-item-text">
                  {file.filename}
                </ListItemText>
                <ListItemText className="FileList__list-item-text">
                  {file.newFileName ?? file.filename}
                </ListItemText>
                <ListItemIcon sx={{ justifyContent: "center" }}>
                  <IconButton
                    edge="end"
                    size="small"
                    color="error"
                    onClick={() => removeFile(file.path)}
                  >
                    <DeleteIcon />
                  </IconButton>
                </ListItemIcon>
              </ListItem>
            ))}
          </List>
        </Paper>
      </ThemeProvider>
    </div>
  );
};
