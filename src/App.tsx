import { EventCallback, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrent } from "@tauri-apps/api/window";
import React, { useEffect } from "react";
import { useMount, useUnmount } from "react-use";
import { FilePage } from "./pages/FilePage/FilePage";
import { OptionsPage } from "./pages/OptionsPage/OptionsPage";
import { getMatches } from "@tauri-apps/api/cli";
import { SettingsPage } from "./pages/SettingsPage/SettingsPage";
import { useAddFiles } from "./atoms/files";
import { AppMessages as messages } from "./messages";
import "./App.scss";
import { useIntl } from "react-intl";
import { useRecoilValue } from "recoil";
import { languageState } from "./atoms/language";

let unlistenList: UnlistenFn[] = [];

const App = () => {
  const intl = useIntl();
  const lang = useRecoilValue(languageState);
  const addFiles = useAddFiles();

  const onFileDrop: EventCallback<string[]> = (evt) => {
    const { payload } = evt;
    addFiles(payload);
  };

  useMount(() => {
    console.log("try to get args");
    getMatches()
      .then((m) => console.log("args: ", m))
      .catch(console.warn);
    const currentWindow = getCurrent();
    currentWindow
      .listen("tauri://file-drop", onFileDrop)
      .then((unlistenFunc) => {
        unlistenList.push(unlistenFunc);
      });
  });

  useUnmount(() => {
    unlistenList.forEach((unlisten) => unlisten());
  });

  useEffect(() => {
    const currentWindow = getCurrent();
    currentWindow.setTitle(intl.formatMessage(messages.title));
  }, [lang]);

  return (
    <div className="App">
      <OptionsPage />
      <SettingsPage />
      <FilePage />
    </div>
  );
};

export default App;
