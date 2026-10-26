import { useEffect, useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  DialogTrigger,
  Field,
  Select,
  Switch,
  Text,
} from "@fluentui/react-components";
import { version } from "../../package.json";
import { api } from "../api";
import { useI18n } from "../i18n";
import type { Language, Settings, Theme } from "../settings";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: Settings;
  onChange: (settings: Settings) => void;
}

export function SettingsDialog({ open, onOpenChange, settings, onChange }: Props) {
  const { t } = useI18n();
  // `null` means Explorer integration isn't available on this platform.
  const [shell, setShell] = useState<boolean | null>(null);
  const [shellError, setShellError] = useState<string | null>(null);

  useEffect(() => {
    if (open) api.shellIntegration().then(setShell, () => setShell(null));
  }, [open]);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    onChange({ ...settings, [key]: value });

  const toggleShell = async (enabled: boolean) => {
    try {
      await api.setShellIntegration(enabled, t("shellMenuLabel"));
      setShell(enabled);
      setShellError(null);
    } catch (e) {
      setShellError(String(e));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(_, d) => onOpenChange(d.open)}>
      <DialogSurface>
        <DialogBody>
          <DialogTitle>{t("settingsTitle")}</DialogTitle>
          <DialogContent className="settings">
            <Field label={t("language")}>
              <Select
                value={settings.language}
                onChange={(_, d) => set("language", d.value as Language)}
              >
                <option value="system">{t("languageSystem")}</option>
                <option value="en">English</option>
                <option value="zh-CN">简体中文</option>
              </Select>
            </Field>
            <Field label={t("theme")}>
              <Select value={settings.theme} onChange={(_, d) => set("theme", d.value as Theme)}>
                <option value="system">{t("themeSystem")}</option>
                <option value="light">{t("themeLight")}</option>
                <option value="dark">{t("themeDark")}</option>
              </Select>
            </Field>
            <Switch
              checked={settings.restoreOptions}
              onChange={(_, d) => set("restoreOptions", d.checked)}
              label={t("restoreOptions")}
            />
            <Switch
              checked={settings.fillLastUsed}
              onChange={(_, d) => set("fillLastUsed", d.checked)}
              label={t("fillLastUsed")}
            />
            <Switch
              checked={settings.rememberRecent}
              onChange={(_, d) => set("rememberRecent", d.checked)}
              label={t("rememberRecent")}
            />
            {settings.rememberRecent && (
              <Field label={t("maxRecent")} orientation="horizontal" className="indented">
                <Select
                  value={String(settings.maxRecent)}
                  onChange={(_, d) => set("maxRecent", Number(d.value))}
                >
                  {[5, 10, 20, 50].map((n) => (
                    <option key={n} value={n}>
                      {n}
                    </option>
                  ))}
                </Select>
              </Field>
            )}
            <Switch
              checked={settings.closeAfterRename}
              onChange={(_, d) => set("closeAfterRename", d.checked)}
              label={t("closeAfterRename")}
            />
            {shell !== null && (
              <Field
                hint={t("shellIntegrationHint")}
                validationState={shellError ? "error" : "none"}
                validationMessage={shellError ?? undefined}
              >
                <Switch
                  checked={shell}
                  onChange={(_, d) => toggleShell(d.checked)}
                  label={t("shellIntegration")}
                />
              </Field>
            )}
            <Text size={200} className="hint">
              {t("version", { version })}
            </Text>
          </DialogContent>
          <DialogActions>
            <DialogTrigger disableButtonEnhancement>
              <Button appearance="primary">{t("close")}</Button>
            </DialogTrigger>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
