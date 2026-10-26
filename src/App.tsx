import { useEffect, useState } from "react";
import { FluentProvider, webDarkTheme, webLightTheme } from "@fluentui/react-components";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { I18nProvider, resolveLocale } from "./i18n";
import { loadSettings, saveSettings } from "./settings";
import { Workspace } from "./Workspace";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function useSystemDark() {
  const [dark, setDark] = useState(() => matchMedia(DARK_QUERY).matches);
  useEffect(() => {
    const query = matchMedia(DARK_QUERY);
    const onChange = (e: MediaQueryListEvent) => setDark(e.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return dark;
}

export default function App() {
  const [settings, setSettings] = useState(loadSettings);
  const systemDark = useSystemDark();
  const dark = settings.theme === "dark" || (settings.theme === "system" && systemDark);
  const locale = resolveLocale(settings.language);

  useEffect(() => saveSettings(settings), [settings]);

  useEffect(() => {
    // Keeps the native title bar in sync with the chosen theme.
    getCurrentWindow()
      .setTheme(settings.theme === "system" ? null : settings.theme)
      .catch(() => {});
  }, [settings.theme]);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return (
    <FluentProvider theme={dark ? webDarkTheme : webLightTheme} className="provider">
      <I18nProvider locale={locale}>
        <Workspace settings={settings} onSettingsChange={setSettings} />
      </I18nProvider>
    </FluentProvider>
  );
}
