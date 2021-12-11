import "@fontsource/roboto/300.css";
import "@fontsource/roboto/400.css";
import "@fontsource/roboto/500.css";
import "@fontsource/roboto/700.css";

// lang files
import zh_CN from "./lang/zh_CN.json";
import en_US from "./lang/en_US.json";

import React from "react";
import ReactDOM from "react-dom";
import App from "./App";
import reportWebVitals from "./reportWebVitals";
import { createTheme, ThemeProvider } from "@mui/material";
import { IntlProvider } from "react-intl";
import { RecoilRoot, useRecoilValue } from "recoil";
import { languageState } from "./atoms/language";
import "./index.scss";

const theme = createTheme({ palette: { mode: "dark" } });

const messages = {
  "zh-CN": zh_CN,
  "en-US": en_US
};

const Root = () => {
  const lang = useRecoilValue(languageState);

  return (
    <React.StrictMode>
      <ThemeProvider theme={theme}>
        <IntlProvider
          locale={lang}
          defaultLocale="en"
          // @ts-ignore
          messages={messages[lang] ?? messages["zh-CN"]}
        >
          <App />
        </IntlProvider>
      </ThemeProvider>
    </React.StrictMode>
  );
};

ReactDOM.render(
  <RecoilRoot>
    <Root />
  </RecoilRoot>,
  document.getElementById("root")
);

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
